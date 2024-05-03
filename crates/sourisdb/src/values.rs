use crate::{
    store::{Store, StoreError},
    types::{
        integer::{Integer, IntegerSerError},
        ts::{TSError, Timestamp},
    },
    utilities::cursor::Cursor,
};
use alloc::{
    boxed::Box,
    format,
    string::{FromUtf8Error, String, ToString},
    vec,
    vec::Vec,
};
use core::{
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
    num::FpCategory,
};
use serde_json::{Error as SJError, Value as SJValue};

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum Value {
    Ch(char),
    String(String),
    Binary(Vec<u8>),
    Bool(bool),
    Int(Integer),
    Imaginary(Integer, Integer),
    Timestamp(Timestamp),
    JSON(SJValue),
    Store(Store),
    Null(Option<()>),
    Float(f64), //TODO: optimise storage?
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        if self.to_ty() != other.to_ty() {
            return false;
        }

        match (self, other) {
            (Self::Ch(c), Self::Ch(c2)) => c.eq(c2),
            (Self::String(s), Self::String(s2)) => s.eq(s2),
            (Self::Binary(b), Self::Binary(b2)) => b.eq(b2),
            (Self::Bool(b), Self::Bool(b2)) => b.eq(b2),
            (Self::Int(i), Self::Int(i2)) => i.eq(i2),
            (Self::Imaginary(a, b), Self::Imaginary(a2, b2)) => a.eq(a2) && b.eq(b2),
            (Self::Timestamp(t), Self::Timestamp(t2)) => t.eq(t2),
            (Self::JSON(j), Self::JSON(j2)) => j.eq(j2),
            (Self::Store(s), Self::Store(s2)) => s.eq(s2),
            (Self::Null(_), Self::Null(_)) => true,
            (Self::Float(f), Self::Float(f2)) => f.eq(f2),
            _ => unreachable!("already checked ty equality"),
        }
    }
}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Value::Ch(v) => {
                v.hash(state);
            }
            Value::String(v) => {
                v.hash(state);
            }
            Value::Binary(v) => {
                v.hash(state);
            }
            Value::Bool(v) => {
                v.hash(state);
            }
            Value::Int(v) => {
                v.hash(state);
            }
            Value::Imaginary(a, b) => {
                a.hash(state);
                b.hash(state);
            }
            Value::Timestamp(v) => {
                v.hash(state);
            }
            Value::JSON(j) => {
                j.to_string().hash(state);
            }
            Value::Store(s) => {
                for k in s.keys() {
                    k.hash(state);
                }
                for v in s.values() {
                    v.hash(state);
                }
            }
            Value::Float(f) => {
                match f.classify() {
                    FpCategory::Nan => 0,
                    FpCategory::Infinite => 1,
                    FpCategory::Zero => 2,
                    FpCategory::Subnormal => 3,
                    FpCategory::Normal => 4,
                }
                .hash(state);
                f.to_le_bytes().hash(state);
            }
            Value::Null(_) => {}
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
            Self::Timestamp(ndt) => s.field("content", ndt),
            Self::JSON(v) => s.field("content", v),
            Self::Store(store) => s.field("content", store),
            Self::Float(f) => s.field("content", f),
            Self::Null(o) => s.field("content", &o),
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
            Self::Timestamp(ndt) => write!(f, "{ndt}"),
            Self::JSON(v) => write!(f, "{v}"),
            Self::Store(s) => write!(f, "{s}"),
            Self::Float(fl) => write!(f, "{fl:?}"),
            Self::Null(o) => write!(f, "{:?}", o),
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
    Timestamp,
    JSON,
    Store,
    Null,
    Float,
}

impl From<ValueTy> for u8 {
    fn from(value: ValueTy) -> Self {
        match value {
            ValueTy::Ch => 0b0000,
            ValueTy::String => 0b0001,
            ValueTy::Binary => 0b0010,
            ValueTy::Bool => 0b0011,
            ValueTy::Int => 0b0100,
            ValueTy::Imaginary => 0b0101,
            // ValueTy::Array => 0b0110,
            ValueTy::Timestamp => 0b0111,
            ValueTy::JSON => 0b1000,
            ValueTy::Store => 0b1001,
            ValueTy::Null => 0b1010,
            ValueTy::Float => 0b1011,
        }
    }
}
impl TryFrom<u8> for ValueTy {
    type Error = ValueSerError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0b0000 => ValueTy::Ch,
            0b0001 => ValueTy::String,
            0b0010 => ValueTy::Binary,
            0b0011 => ValueTy::Bool,
            0b0100 => ValueTy::Int,
            0b0101 => ValueTy::Imaginary,
            // 0b0110 => ValueTy::Array,
            0b0111 => ValueTy::Timestamp,
            0b1000 => ValueTy::JSON,
            0b1001 => ValueTy::Store,
            0b1010 => ValueTy::Null,
            0b1011 => ValueTy::Float,
            _ => return Err(ValueSerError::InvalidType(value)),
        })
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
    TSError(TSError),
    SerdeJson(SJError),
    StoreError(Box<StoreError>),
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
            ValueSerError::TSError(e) => write!(f, "Error de/ser-ing timestamp: {e:?}"),
            ValueSerError::SerdeJson(e) => write!(f, "Error de/ser-ing serde_json: {e:?}"),
            ValueSerError::StoreError(e) => write!(f, "Error de/ser-ing souris store: {e:?}"),
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
impl From<StoreError> for ValueSerError {
    fn from(value: StoreError) -> Self {
        Self::StoreError(Box::new(value))
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ValueSerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ValueSerError::IntegerSerError(e) => Some(e),
            ValueSerError::NonUTF8String(e) => Some(e),
            ValueSerError::TSError(e) => Some(e),
            ValueSerError::SerdeJson(e) => Some(e),
            ValueSerError::StoreError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<SJValue> for Value {
    fn from(v: SJValue) -> Self {
        match v {
            SJValue::Null => Value::Null(None),
            SJValue::Bool(b) => Value::Bool(b),
            SJValue::Number(n) => {
                if let Some(neg) = n.as_i64() {
                    Value::Int(Integer::i64(neg))
                } else if let Some(pos) = n.as_u64() {
                    Value::Int(Integer::u64(pos))
                } else if let Some(float) = n.as_f64() {
                    Value::Float(float)
                } else {
                    unreachable!("must be one of the three JSON integer types")
                }
            }
            SJValue::String(s) => Value::String(s.to_string()),
            SJValue::Array(a) => {
                Value::Store(Store::new_arr(a.into_iter().map(Self::from).collect()))
            }
            SJValue::Object(o) => Value::Store(Store::from(o)),
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
            Self::Timestamp(_) => ValueTy::Timestamp,
            Self::JSON(_) => ValueTy::JSON,
            Self::Store(_) => ValueTy::Store,
            Self::Float(_) => ValueTy::Float,
            Self::Null(_) => ValueTy::Null,
        }
    }

    pub fn ser(&self) -> Result<Vec<u8>, ValueSerError> {
        let mut res = vec![];

        let vty = self.to_ty();
        let ty = u8::from(vty) << 4;

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
                res.extend(Integer::u32(*ch as u32).ser());
            }
            Self::String(s) => {
                let bytes = s.as_bytes();

                res.extend(Integer::usize(bytes.len()).ser().iter());
                res.extend(bytes.iter());
            }
            Self::Binary(b) => {
                res.extend(b.iter());
            }
            Self::Bool(_) => {
                unreachable!("reached bool after niche optimisations applied uh oh")
            }
            Self::Int(i) => {
                res.extend(i.ser().iter());
            }
            Self::Imaginary(a, b) => {
                res.extend(a.ser().iter());
                res.extend(b.ser().iter());
            }
            Self::Timestamp(t) => {
                res.extend(t.ser().iter());
            }
            Self::JSON(v) => {
                let str = v.to_string();
                let bytes = str.as_bytes();

                res.extend(Integer::usize(bytes.len()).ser().iter());
                res.extend(bytes.iter());
            }
            Self::Store(s) => {
                res.extend(s.ser()?);
            }
            Self::Null(_) => {}
            Self::Float(f) => {
                let bytes = f.to_le_bytes();
                res.extend(bytes.iter()); //TODO: optimise this
            }
        }

        Ok(res)
    }

    pub fn deser(bytes: &mut Cursor<u8>) -> Result<Self, ValueSerError> {
        let [byte] = bytes.read(1).ok_or(ValueSerError::NotEnoughBytes)? else {
            unreachable!("didn't get just one byte back")
        };
        let byte = *byte;

        let ty = byte >> 4;
        let ty = ValueTy::try_from(ty)?;

        Ok(match ty {
            ValueTy::Int => {
                let int = Integer::deser(bytes)?;
                Self::Int(int)
            }
            ValueTy::Imaginary => {
                let a = Integer::deser(bytes)?;
                let b = Integer::deser(bytes)?;
                Self::Imaginary(a, b)
            }
            ValueTy::Ch => {
                let ch = char::from_u32(Integer::deser(bytes)?.try_into()?)
                    .ok_or(ValueSerError::InvalidCharacter)?;
                Self::Ch(ch)
            }
            ValueTy::Timestamp => {
                let t = Timestamp::deser(bytes)?;
                Self::Timestamp(t)
            }
            ValueTy::String => {
                let len: usize = Integer::deser(bytes)?.try_into()?;
                let str_bytes = bytes
                    .read(len)
                    .ok_or(ValueSerError::NotEnoughBytes)?
                    .to_vec();
                Self::String(String::from_utf8(str_bytes)?)
            }
            ValueTy::JSON => {
                let len: usize = Integer::deser(bytes)?.try_into()?;
                let str_bytes = bytes
                    .read(len)
                    .ok_or(ValueSerError::NotEnoughBytes)?
                    .to_vec();
                let value: SJValue = serde_json::from_slice(&str_bytes)?;
                Self::JSON(value)
            }
            ValueTy::Binary => {
                let len: usize = Integer::deser(bytes)?.try_into()?;
                let bytes = bytes
                    .read(len)
                    .ok_or(ValueSerError::NotEnoughBytes)?
                    .to_vec();
                Self::Binary(bytes)
            }
            ValueTy::Bool => Self::Bool((byte & 0b0000_0001) > 0),
            ValueTy::Store => Self::Store(Store::deser(bytes)?),
            ValueTy::Null => Self::Null(None),
            ValueTy::Float => {
                let bytes: [u8; 8] = match bytes.read(8).map(TryInto::try_into) {
                    None => return Err(ValueSerError::NotEnoughBytes),
                    Some(Err(_e)) => {
                        unreachable!("Trying to get 8 bytes into 8 bytes, no?")
                    }
                    Some(Ok(b)) => b,
                };
                Self::Float(f64::from_le_bytes(bytes))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Value;
    use crate::{types::integer::Integer, utilities::cursor::Cursor};

    #[test]
    fn test_bools() {
        {
            let t = Value::Bool(true);
            let ser = t.clone().ser().unwrap();

            assert_eq!(t, Value::deser(&mut Cursor::new(&ser)).unwrap());
        }
        {
            let f = Value::Bool(false);
            let ser = f.clone().ser().unwrap();

            assert_eq!(f, Value::deser(&mut Cursor::new(&ser)).unwrap());
        }
    }

    #[test]
    fn test_ints() {
        {
            let neg = Value::Int(Integer::i8(-15));
            let ser = neg.clone().ser().unwrap();

            assert_eq!(neg, Value::deser(&mut Cursor::new(&ser)).unwrap());
        }
        {
            let big = Value::Int(Integer::usize(123_456_789));
            let ser = big.clone().ser().unwrap();

            assert_eq!(big, Value::deser(&mut Cursor::new(&ser)).unwrap());
        }
    }
}
