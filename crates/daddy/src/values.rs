use crate::{
    niches::integer::{Integer, IntegerSerError},
    utilities::cursor::Cursor,
};
use alloc::{
    string::{FromUtf8Error, String},
    vec,
    vec::Vec,
};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum Value {
    Ch(char),
    String(String),
    Binary(Vec<u8>),
    Bool(bool),
    Int(Integer),
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
pub enum ValueSerError {
    TooLong,
    InvalidType,
    Empty,
    IntegerSerFailure(IntegerSerError),
    NotEnoughBytes,
    InvalidCharacter,
    NonUTF8String(FromUtf8Error),
}

impl From<IntegerSerError> for ValueSerError {
    fn from(value: IntegerSerError) -> Self {
        Self::IntegerSerFailure(value)
    }
}
impl From<FromUtf8Error> for ValueSerError {
    fn from(value: FromUtf8Error) -> Self {
        Self::NonUTF8String(value)
    }
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
    pub fn serialise(self) -> Result<Vec<u8>, ValueSerError> {
        let mut res = vec![];

        let vty = self.to_ty();
        let ty = vty.id() << 5;

        let niche = match &self {
            Self::Bool(b) => Some(u8::from(*b)),
            _ => None,
        };
        if let Some(niche) = niche {
            res.push(niche | ty);
            return Ok(res);
        }

        res.push(ty);

        match self {
            Self::Ch(ch) => {
                res.extend(Integer::u32(ch as u32).ser());
            }
            Self::String(s) => {
                res.extend(s.as_bytes().iter());
            }
            Self::Binary(b) => {
                res.extend(b.iter());
            }
            Self::Bool(_) => unreachable!("reached bool after niche optimisations applied uh oh"),
            Self::Int(i) => {
                res.extend(i.ser().iter());
            }
        }

        Ok(res)
    }

    pub fn deserialise(bytes: &mut Cursor, len: usize) -> Result<Self, ValueSerError> {
        enum State {
            Start,
            FoundType(ValueTy, u8),
            FindingContent(ValueTy),
        }

        let mut state = State::Start;

        let mut tmp = vec![];
        let starting_pos = bytes.position();

        loop {
            if bytes.position() - starting_pos == len {
                break;
            }
            let [byte] = bytes.read(1).ok_or(ValueSerError::NotEnoughBytes)? else {
                unreachable!("didn't get just one byte back")
            };
            let byte = *byte;

            state = match state {
                State::Start => {
                    let ty = match byte >> 5 {
                        0b000 => ValueTy::Ch,
                        0b001 => ValueTy::String,
                        0b010 => ValueTy::Binary,
                        0b011 => ValueTy::Bool,
                        0b100 => ValueTy::Int,
                        _ => return Err(ValueSerError::InvalidType),
                    };

                    match ty {
                        ValueTy::Int => {
                            let int = Integer::deser(bytes)?;
                            return Ok(Self::Int(int));
                        }
                        ValueTy::Ch => {
                            let ch = char::from_u32(Integer::deser(bytes)?.try_into()?)
                                .ok_or(ValueSerError::InvalidCharacter)?;
                            return Ok(Self::Ch(ch));
                        }
                        _ => {}
                    }

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
            State::Start => return Err(ValueSerError::Empty),
            State::FoundType(ty, ty_byte) => {
                let relevant_niche = ty_byte & 0b000_11111;
                match ty {
                    ValueTy::Bool => Value::Bool(relevant_niche > 0),
                    _ => unreachable!("no other niche optimisations apart from bool"),
                }
            }
            State::FindingContent(ty) => {
                let tmp = core::mem::take(&mut tmp);
                match ty {
                    ValueTy::Ch => unreachable!("already dealt with character type"),
                    ValueTy::String => {
                        let st = String::from_utf8(tmp)?;
                        Self::String(st)
                    }
                    ValueTy::Binary => Self::Binary(tmp),
                    ValueTy::Bool => unreachable!("all bools go through nice optimisation"),
                    ValueTy::Int => unreachable!("already dealt with integer type"),
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Value;
    use crate::{niches::integer::Integer, utilities::cursor::Cursor, values::ValueTy};

    #[test]
    fn test_bools() {
        {
            let t = Value::Bool(true);
            let ser = t.clone().serialise().unwrap();

            let expected = &[ValueTy::Bool.id() << 5 | 1];
            assert_eq!(&ser, expected);

            assert_eq!(
                t,
                Value::deserialise(&mut Cursor::new(&ser), ser.len()).unwrap()
            );
        }
        {
            let f = Value::Bool(false);
            let ser = f.clone().serialise().unwrap();

            let expected = &[ValueTy::Bool.id() << 5];
            assert_eq!(&ser, expected);

            assert_eq!(
                f,
                Value::deserialise(&mut Cursor::new(&ser), ser.len()).unwrap()
            );
        }
    }

    #[test]
    fn test_ints() {
        {
            let neg = Value::Int(Integer::i8(-15));
            let ser = neg.clone().serialise().unwrap();

            assert_eq!(
                neg,
                Value::deserialise(&mut Cursor::new(&ser), ser.len()).unwrap()
            );
        }
        {
            let big = Value::Int(Integer::usize(123_456_789));
            let ser = big.clone().serialise().unwrap();

            assert_eq!(
                big,
                Value::deserialise(&mut Cursor::new(&ser), ser.len()).unwrap()
            );
        }
    }
}
