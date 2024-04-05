#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum Value {
    Ch(char),
    String(String),
    Binary(Vec<u8>),
    Bool(bool),
    Int(i64),
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum ValueTy {
    Ch,
    String,
    Binary,
    Bool,
    Int,
}

impl ValueTy {
    pub fn id(self) -> u8 {
        match self {
            ValueTy::Ch => 0b000,
            ValueTy::String => 0b001,
            ValueTy::Binary => 0b010,
            ValueTy::Bool => 0b011,
            ValueTy::Int => 0b100,
        }
    }
}

#[derive(Debug)]
pub enum ValueFailure {
    TooLong,
    InvalidType(u8),
    Empty,
}

impl Value {
    pub(crate) const fn to_ty(&self) -> ValueTy {
        match self {
            Self::Ch(_) => ValueTy::Ch,
            Self::String(_) => ValueTy::String,
            Self::Binary(_) => ValueTy::Binary,
            Self::Bool(_) => ValueTy::Bool,
            Self::Int(_) => ValueTy::Int,
        }
    }

    ///Structure of Value in DB:
    ///
    /// end marker: 0xDEADBEEF
    ///
    ///
    /// 3 bits: type
    /// either:
    ///     5 bits: niche
    /// or:
    ///     5 bits: zero
    ///     length bytes: content
    ///     4 bytes: end
    pub fn serialise(self) -> Result<Vec<u8>, ValueFailure> {
        let mut res = vec![];

        let vty = self.to_ty();
        let ty = vty.id() << 5;

        let niche = match &self {
            Self::Bool(b) => Some(if *b { 1 } else { 0 }),
            Self::Int(i) if (*i < (1 << 5) && *i >= 0) => Some(*i as u8),
            _ => None,
        };
        if let Some(niche) = niche {
            res.push(niche | ty);
            return Ok(res);
        }

        res.push(ty);

        match self {
            Self::Ch(ch) => {
                res.extend((ch as u32).to_le_bytes());
            }
            Self::String(s) => {
                res.extend(s.as_bytes().iter());
            }
            Self::Binary(b) => {
                res.extend(b.iter());
            }
            Self::Bool(_) => unreachable!("reached bool after niche optimisations applied uh oh"),
            Self::Int(i) => {
                res.extend(i.to_le_bytes().iter());
            }
        }

        Ok(res)
    }

    pub fn deserialise(bytes: &[u8]) -> Result<Self, ValueFailure> {
        enum State {
            Start,
            FoundType(ValueTy, u8),
            FindingContent(ValueTy),
        }

        let mut state = State::Start;
        let mut tmp: Vec<u8> = vec![];

        for byte in bytes.iter() {
            let byte = *byte;
            state = match state {
                State::Start => {
                    let ty = match byte >> 5 {
                        0b000 => ValueTy::Ch,
                        0b001 => ValueTy::String,
                        0b010 => ValueTy::Binary,
                        0b011 => ValueTy::Bool,
                        0b100 => ValueTy::Int,
                        _ => return Err(ValueFailure::InvalidType(byte >> 5)),
                    };
                    State::FoundType(ty, byte)
                }
                State::FoundType(ty, _ty_byte) => {
                    tmp.push(byte);
                    State::FindingContent(ty)
                }
                State::FindingContent(ty) => {
                    tmp.push(byte);
                    State::FindingContent(ty)
                }
            }
        }

        Ok(match state {
            State::Start => return Err(ValueFailure::Empty),
            State::FoundType(ty, ty_byte) => {
                let relevant_niche = ty_byte & 0b000_11111;
                match ty {
                    ValueTy::Bool => Value::Bool(relevant_niche > 0),
                    ValueTy::Int => Value::Int(relevant_niche as i64),
                    _ => unreachable!("no other niche optimisations apart from bool & int"),
                }
            }
            State::FindingContent(ty) => {
                let tmp = std::mem::take(&mut tmp);
                match ty {
                    ValueTy::Ch => {
                        let ch =
                            char::from_u32(u32::from_le_bytes(
                                tmp.try_into().unwrap(),
                            )).unwrap();
                        Self::Ch(ch)
                    }
                    ValueTy::String => {
                        let st = String::from_utf8(tmp).unwrap();
                        Self::String(st)
                    }
                    ValueTy::Binary => Self::Binary(tmp),
                    ValueTy::Bool => unreachable!("all bools go through nice optimisation"),
                    ValueTy::Int => {
                        let int = i64::from_le_bytes(tmp.try_into().unwrap());
                        Self::Int(int)
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::values::ValueTy;
    use super::Value;

    #[test]
    fn test_bools() {
        {
            let t = Value::Bool(true);
            let ser = t.clone().serialise().unwrap();

            let expected = &[ValueTy::Bool.id() << 5 | 1];
            assert_eq!(&ser, expected);

            assert_eq!(t, Value::deserialise(&ser).unwrap());
        }
        {
            let f = Value::Bool(false);
            let ser = f.clone().serialise().unwrap();

            let expected = &[ValueTy::Bool.id() << 5];
            assert_eq!(&ser, expected);

            assert_eq!(f, Value::deserialise(&ser).unwrap());
        }
    }

    #[test]
    fn test_ints() {
        {
            let smol = Value::Int(5);
            let ser = smol.clone().serialise().unwrap();

            let expected = &[ValueTy::Int.id() << 5 | 5];
            assert_eq!(&ser, expected);

            assert_eq!(smol, Value::deserialise(&ser).unwrap());
        }
        {
            let neg = Value::Int(-15);
            let ser = neg.clone().serialise().unwrap();

            let bytes = (-15_i64).to_le_bytes();
            let expected = &[ValueTy::Int.id() << 5, bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]];
            assert_eq!(&ser, expected);

            assert_eq!(neg, Value::deserialise(&ser).unwrap());
        }
        {
            let big = Value::Int(1234567890);
            let ser = big.clone().serialise().unwrap();

            let bytes = (1234567890_i64).to_le_bytes();
            let expected = &[ValueTy::Int.id() << 5, bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]];
            assert_eq!(&ser, expected);

            assert_eq!(big, Value::deserialise(&ser).unwrap());
        }
    }
}
