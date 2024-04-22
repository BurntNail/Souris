use crate::{
    types::{
        array::{Array, ArraySerError},
        integer::{Integer, IntegerSerError},
        ts::{TSError, Timestamp},
    },
    utilities::cursor::Cursor,
    version::Version,
};
use alloc::{
    format,
    string::{FromUtf8Error, String, ToString},
    vec,
    vec::Vec,
};
use core::{
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
};
use serde_json::{Error as SJError, Value as SJValue};

#[derive(Clone, Eq, PartialEq)]
pub enum Value {
    Ch(char),
    String(String),
    Binary(Vec<u8>),
    Bool(bool),
    Int(Integer),
    Imaginary(Integer, Integer),
    Array(Array),
    Timestamp(Timestamp),
    JSON(SJValue),
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        //adapted from derive macro expansion
        let tag = core::mem::discriminant(self);
        tag.hash(state);

        match self {
            Value::Ch(v) => v.hash(state),
            Value::String(v) => v.hash(state),
            Value::Binary(v) => v.hash(state),
            Value::Bool(v) => v.hash(state),
            Value::Int(v) => v.hash(state),
            Value::Imaginary(a, b) => {
                a.hash(state);
                b.hash(state);
            }
            Value::Array(v) => v.hash(state),
            Value::Timestamp(v) => v.hash(state),
            Value::JSON(j) => {
                let str = j.to_string();
                str.hash(state);
            }
        }
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl Debug for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut s = f.debug_struct("Value");
        s.field("ty", &self.to_ty());

        match &self {
            Self::Ch(ch) => s.field("content", ch),
            Self::String(str) => s.field("content", str),
            Self::Binary(b) => s.field("content", &display_bytes_as_hex_array(b)),
            Self::Bool(b) => s.field("content", b),
            Self::Int(i) => s.field("content", i),
            Self::Imaginary(a, b) => s.field("content", &(a, b)),
            Self::Array(a) => s.field("content", &a),
            Self::Timestamp(ndt) => s.field("content", &ndt),
            Self::JSON(v) => s.field("json", &v),
        };

        s.finish()
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match &self {
            Self::Ch(ch) => write!(f, "{ch:?}"),
            Self::String(str) => write!(f, "{str:?}"),
            Self::Binary(b) => {
                write!(f, "{}", display_bytes_as_hex_array(b))
            }
            Self::Bool(b) => write!(f, "{b}"),
            Self::Int(i) => write!(f, "{i}"),
            Self::Imaginary(a, b) => {
                if b.is_negative() {
                    write!(f, "{a}{b}i")
                } else {
                    write!(f, "{a}+{b}i")
                }
            }
            Self::Array(a) => write!(f, "{a}"),
            Self::Timestamp(ndt) => write!(f, "{ndt}"),
            Self::JSON(v) => write!(f, "{v}"),
        }
    }
}

fn display_bytes_as_hex_array(b: &[u8]) -> String {
    let mut out;
    match b.len() {
        0 => out = "[]".to_string(),
        1 => out = format!("[{:#X}]", b[0]),
        _ => {
            out = format!("[{:#X}", b[0]);
            for b in b.iter().skip(1) {
                out.push_str(&format!(", {b:#X}"));
            }
            out.push(']');
        }
    };
    out
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ValueTy {
    Ch,
    String,
    Binary,
    Bool,
    Int,
    Imaginary,
    Array,
    Timestamp,
    JSON,
}

impl ValueTy {
    #[must_use]
    pub fn id(self) -> u8 {
        match self {
            ValueTy::Ch => 0b0000,
            ValueTy::String => 0b0001,
            ValueTy::Binary => 0b0010,
            ValueTy::Bool => 0b0011,
            ValueTy::Int => 0b0100,
            ValueTy::Imaginary => 0b0101,
            ValueTy::Array => 0b0110,
            ValueTy::Timestamp => 0b0111,
            ValueTy::JSON => 0b1000,
        }
    }
}

#[derive(Debug)]
pub enum ValueSerError {
    InvalidType(u8),
    Empty,
    IntegerSerError(IntegerSerError),
    NotEnoughBytes,
    InvalidCharacter,
    NonUTF8String(FromUtf8Error),
    ArraySerError(ArraySerError),
    TSError(TSError),
    SerdeJson(SJError),
}

impl Display for ValueSerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            ValueSerError::InvalidType(b) => write!(f, "Invalid Type Discriminant found: {b:#b}"),
            ValueSerError::Empty => write!(
                f,
                "Length provided was zero - what did you expect to deserialise there?"
            ),
            ValueSerError::IntegerSerError(e) => write!(f, "Error de/ser-ing integer: {e:?}"),
            ValueSerError::NotEnoughBytes => write!(f, "Not enough bytes provided"),
            ValueSerError::InvalidCharacter => write!(f, "Invalid character provided"),
            ValueSerError::NonUTF8String(e) => write!(f, "Error converting to UTF-8: {e:?}"),
            ValueSerError::ArraySerError(e) => write!(f, "Error de/ser-ing array: {e:?}"),
            ValueSerError::TSError(e) => write!(f, "Error de/ser-ing timestamp: {e:?}"),
            ValueSerError::SerdeJson(e) => write!(f, "Error deserialising serde_json: {e:?}"),
        }
    }
}

impl From<IntegerSerError> for ValueSerError {
    fn from(value: IntegerSerError) -> Self {
        Self::IntegerSerError(value)
    }
}
impl From<FromUtf8Error> for ValueSerError {
    fn from(value: FromUtf8Error) -> Self {
        Self::NonUTF8String(value)
    }
}
impl From<ArraySerError> for ValueSerError {
    fn from(value: ArraySerError) -> Self {
        Self::ArraySerError(value)
    }
}
impl From<TSError> for ValueSerError {
    fn from(value: TSError) -> Self {
        Self::TSError(value)
    }
}
impl From<SJError> for ValueSerError {
    fn from(value: SJError) -> Self {
        Self::SerdeJson(value)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ValueSerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ValueSerError::IntegerSerError(e) => Some(e),
            ValueSerError::NonUTF8String(e) => Some(e),
            ValueSerError::ArraySerError(e) => Some(e),
            ValueSerError::TSError(e) => Some(e),
            ValueSerError::SerdeJson(e) => Some(e),
            _ => None,
        }
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
            Self::Imaginary(_, _) => ValueTy::Imaginary,
            Self::Array(_) => ValueTy::Array,
            Self::Timestamp(_) => ValueTy::Timestamp,
            Self::JSON(_) => ValueTy::JSON,
        }
    }

    pub fn ser(&self, version: Version) -> Result<Vec<u8>, ValueSerError> {
        match version {
            Version::V0_1_0 => {
                let mut res = vec![];

                let vty = self.to_ty();
                let ty = vty.id() << 4;

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
                        res.extend(Integer::u32(*ch as u32).ser(version));
                    }
                    Self::String(s) => {
                        let bytes = s.as_bytes();

                        res.extend(Integer::usize(bytes.len()).ser(version).iter());
                        res.extend(bytes.iter());
                    }
                    Self::Binary(b) => {
                        res.extend(b.iter());
                    }
                    Self::Bool(_) => {
                        unreachable!("reached bool after niche optimisations applied uh oh")
                    }
                    Self::Int(i) => {
                        res.extend(i.ser(version).iter());
                    }
                    Self::Imaginary(a, b) => {
                        res.extend(a.ser(version).iter());
                        res.extend(b.ser(version).iter());
                    }
                    Self::Array(a) => {
                        res.extend(a.ser(version)?.iter());
                    }
                    Self::Timestamp(t) => {
                        res.extend(t.ser(version).iter());
                    }
                    Self::JSON(v) => {
                        let str = v.to_string();
                        let bytes = str.as_bytes();

                        res.extend(Integer::usize(bytes.len()).ser(version).iter());
                        res.extend(bytes.iter());
                    }
                }

                Ok(res)
            }
        }
    }

    pub fn deserialise(bytes: &mut Cursor<u8>, version: Version) -> Result<Self, ValueSerError> {
        match version {
            Version::V0_1_0 => {
                let [byte] = bytes.read(1).ok_or(ValueSerError::NotEnoughBytes)? else {
                    unreachable!("didn't get just one byte back")
                };
                let byte = *byte;

                let ty = byte >> 4;
                let ty = match ty {
                    0b0000 => ValueTy::Ch,
                    0b0001 => ValueTy::String,
                    0b0010 => ValueTy::Binary,
                    0b0011 => ValueTy::Bool,
                    0b0100 => ValueTy::Int,
                    0b0101 => ValueTy::Imaginary,
                    0b0110 => ValueTy::Array,
                    0b0111 => ValueTy::Timestamp,
                    0b1000 => ValueTy::JSON,
                    _ => return Err(ValueSerError::InvalidType(ty)),
                };

                Ok(match ty {
                    ValueTy::Int => {
                        let int = Integer::deser(bytes, version)?;
                        Self::Int(int)
                    }
                    ValueTy::Imaginary => {
                        let a = Integer::deser(bytes, version)?;
                        let b = Integer::deser(bytes, version)?;
                        Self::Imaginary(a, b)
                    }
                    ValueTy::Ch => {
                        let ch = char::from_u32(Integer::deser(bytes, version)?.try_into()?)
                            .ok_or(ValueSerError::InvalidCharacter)?;
                        Self::Ch(ch)
                    }
                    ValueTy::Array => {
                        let a = Array::deser(bytes, version)?;
                        Self::Array(a)
                    }
                    ValueTy::Timestamp => {
                        let t = Timestamp::deser(bytes, version)?;
                        Self::Timestamp(t)
                    }
                    ValueTy::String => {
                        let len: usize = Integer::deser(bytes, version)?.try_into()?;
                        let str_bytes = bytes
                            .read(len)
                            .ok_or(ValueSerError::NotEnoughBytes)?
                            .to_vec();
                        Self::String(String::from_utf8(str_bytes)?)
                    }
                    ValueTy::JSON => {
                        let len: usize = Integer::deser(bytes, version)?.try_into()?;
                        let str_bytes = bytes
                            .read(len)
                            .ok_or(ValueSerError::NotEnoughBytes)?
                            .to_vec();
                        let value: SJValue = serde_json::from_slice(&str_bytes)?;
                        Self::JSON(value)
                    }
                    ValueTy::Binary => {
                        let len: usize = Integer::deser(bytes, version)?.try_into()?;
                        let bytes = bytes
                            .read(len)
                            .ok_or(ValueSerError::NotEnoughBytes)?
                            .to_vec();
                        Self::Binary(bytes)
                    }
                    ValueTy::Bool => Self::Bool((byte & 0b0000_0001) > 0),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Value;
    use crate::{
        types::integer::Integer, utilities::cursor::Cursor, values::ValueTy, version::Version,
    };

    #[test]
    fn test_bools() {
        {
            let t = Value::Bool(true);
            let ser = t.clone().ser(Version::V0_1_0).unwrap();

            let expected = &[ValueTy::Bool.id() << 5 | 1];
            assert_eq!(&ser, expected);

            assert_eq!(
                t,
                Value::deserialise(&mut Cursor::new(&ser), Version::V0_1_0).unwrap()
            );
        }
        {
            let f = Value::Bool(false);
            let ser = f.clone().ser(Version::V0_1_0).unwrap();

            let expected = &[ValueTy::Bool.id() << 5];
            assert_eq!(&ser, expected);

            assert_eq!(
                f,
                Value::deserialise(&mut Cursor::new(&ser), Version::V0_1_0).unwrap()
            );
        }
    }

    #[test]
    fn test_ints() {
        {
            let neg = Value::Int(Integer::i8(-15));
            let ser = neg.clone().ser(Version::V0_1_0).unwrap();

            assert_eq!(
                neg,
                Value::deserialise(&mut Cursor::new(&ser), Version::V0_1_0).unwrap()
            );
        }
        {
            let big = Value::Int(Integer::usize(123_456_789));
            let ser = big.clone().ser(Version::V0_1_0).unwrap();

            assert_eq!(
                big,
                Value::deserialise(&mut Cursor::new(&ser), Version::V0_1_0).unwrap()
            );
        }
    }
}
