use crate::{
    types::{
        integer::{Integer, IntegerSerError, SignedState},
        ts::{TSError, Timestamp},
    },
    utilities::cursor::Cursor,
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
    num::FpCategory,
};
use core::net::{Ipv4Addr, Ipv6Addr};
use core::str::FromStr;
use chrono_tz::{Tz};
use hashbrown::HashMap;
use rust_decimal::Decimal;
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
    Null(()),
    Float(f64),
    Array(Vec<Value>),
    Map(HashMap<String, Value>),
    Timezone(Tz),
    Ipv4Addr(Ipv4Addr),
    Ipv6Addr(Ipv6Addr),
    Decimal(Decimal),
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
            (Self::Null(_), Self::Null(_)) => true,
            (Self::Float(f), Self::Float(f2)) => f.eq(f2),
            (Self::Array(a), Self::Array(a2)) => a.eq(a2),
            (Self::Map(m), Self::Map(m2)) => m.eq(m2),
            (Self::Timezone(t), Self::Timezone(t2)) => t.eq(t2),
            (Self::Ipv4Addr(t), Self::Ipv4Addr(t2)) => t.eq(t2),
            (Self::Ipv6Addr(t), Self::Ipv6Addr(t2)) => t.eq(t2),
            (Self::Decimal(t), Self::Decimal(t2)) => t.eq(t2),
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
            Value::Map(m) => {
                for k in m.keys() {
                    k.hash(state);
                }
                for v in m.values() {
                    v.hash(state);
                }
            }
            Value::Array(a) => {
                for v in a {
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
            Value::Timezone(tz) => {
                tz.hash(state);
            }
            Value::Ipv4Addr(a) => {
                a.hash(state);
            }
            Value::Ipv6Addr(a) => {
                a.hash(state);
            }
            Value::Decimal(d) => {
                d.hash(state);
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
            Self::Timestamp(ndt) => s.field("content", ndt),
            Self::JSON(v) => s.field("content", v),
            Self::Float(f) => s.field("content", f),
            Self::Null(o) => s.field("content", &o),
            Self::Array(a) => s.field("content", &a),
            Self::Map(m) => s.field("content", &m),
            Self::Ipv4Addr(m) => s.field("content", &m),
            Self::Ipv6Addr(m) => s.field("content", &m),
            Self::Decimal(m) => s.field("content", &m),
            Self::Timezone(m) => s.field("content", &m),
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
            Self::Float(fl) => write!(f, "{fl}"),
            Self::Null(_o) => write!(f, "null"),
            Self::Map(m) => write!(f, "{m:?}"),
            Self::Array(a) => write!(f, "{a:?}"),
            Self::Timezone(v) => write!(f, "{v}"),
            Self::Ipv4Addr(v) => write!(f, "{v}"),
            Self::Ipv6Addr(v) => write!(f, "{v}"),
            Self::Decimal(v) => write!(f, "{v}"),
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
    Null,
    Float,
    Array,
    Map,
    Timezone,
    IpV4,
    IpV6,
    Decimal,
}

impl From<ValueTy> for u8 {
    fn from(value: ValueTy) -> Self {
        match value {
            ValueTy::Ch => 0,
            ValueTy::String => 1,
            ValueTy::Binary => 2,
            ValueTy::Bool => 3,
            ValueTy::Int => 4,
            ValueTy::Imaginary => 5,
            ValueTy::Timestamp => 6,
            ValueTy::JSON => 7,
            ValueTy::Map => 8,
            ValueTy::Null => 9,
            ValueTy::Float => 10,
            ValueTy::Array => 11,
            ValueTy::Timezone => 12,
            ValueTy::IpV4 => 13,
            ValueTy::IpV6 => 14,
            ValueTy::Decimal => 15,
        }
    }
}
impl TryFrom<u8> for ValueTy {
    type Error = ValueSerError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => ValueTy::Ch,
            1 => ValueTy::String,
            2 => ValueTy::Binary,
            3 => ValueTy::Bool,
            4 => ValueTy::Int,
            5 => ValueTy::Imaginary,
            6 => ValueTy::Timestamp,
            7 => ValueTy::JSON,
            8 => ValueTy::Map,
            9 => ValueTy::Null,
            10 => ValueTy::Float,
            11 => ValueTy::Array,
            12 => ValueTy::Timezone,
            13 => ValueTy::IpV4,
            14 => ValueTy::IpV6,
            15 => ValueTy::Decimal,
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
    UnexpectedValueType(Value, ValueTy),
    TzError(chrono_tz::ParseError),
    DecimalError(rust_decimal::Error)
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
            ValueSerError::UnexpectedValueType(v, ex) => write!(f, "Expected {ex:?}, found: {v:?}"),
            ValueSerError::TzError(e) => write!(f, "Error parsing timezone: {e:?}"),
            ValueSerError::DecimalError(e) => write!(f, "Error with decimals: {e:?}"),
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
impl From<chrono_tz::ParseError> for ValueSerError {
    fn from(value: chrono_tz::ParseError) -> Self {
        Self::TzError(value)
    }
}
impl From<rust_decimal::Error> for ValueSerError {
    fn from(value: rust_decimal::Error) -> Self {
        Self::DecimalError(value)
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
            ValueSerError::TzError(e) => Some(e),
            ValueSerError::DecimalError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<SJValue> for Value {
    fn from(v: SJValue) -> Self {
        match v {
            SJValue::Null => Value::Null(()),
            SJValue::Bool(b) => Value::Bool(b),
            SJValue::Number(n) => {
                if let Some(neg) = n.as_i64() {
                    Value::Int(Integer::i64(neg))
                } else if let Some(pos) = n.as_u64() {
                    Value::Int(Integer::u64(pos))
                } else if let Some(float) = n.as_f64() {
                    Value::Float(float)
                } else {
                    unreachable!("must be one of the three JSON number types")
                }
            }
            SJValue::String(s) => Value::String(s.to_string()),
            SJValue::Array(a) => {
                Value::Array(a.into_iter().map(Self::from).collect())
            }
            SJValue::Object(o) => {
                Value::Map(o.into_iter().map(|(k, v)| (k, v.into())).collect())
            },
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
            Self::Map(_) => ValueTy::Map,
            Self::Array(_) => ValueTy::Array,
            Self::Float(_) => ValueTy::Float,
            Self::Null(_) => ValueTy::Null,
            Self::Timezone(_) => ValueTy::Timezone,
            Self::Ipv4Addr(_) => ValueTy::IpV4,
            Self::Ipv6Addr(_) => ValueTy::IpV6,
            Self::Decimal(_) => ValueTy::Decimal,
        }
    }

    pub fn ser(&self) -> Result<Vec<u8>, ValueSerError> {
        let mut res = vec![];

        let mut ty = u8::from(self.to_ty()) << 4;

        match self {
            Self::Ch(ch) => {
                let (_, bytes) = Integer::from(*ch as u32).ser();

                res.push(ty);
                res.extend(bytes.iter());
            }
            Self::String(s) => {
                let str_bytes = s.as_bytes();
                let (_, len_bytes) = Integer::from(str_bytes.len()).ser();

                res.push(ty);
                res.extend(len_bytes.iter());
                res.extend(str_bytes.iter());
            }
            Self::Binary(b) => {
                let (_, len_bytes) = Integer::from(b.len()).ser();

                res.push(ty);
                res.extend(len_bytes);
                res.extend(b.iter());
            }
            Self::Bool(b) => {
                ty |= u8::from(*b) << 2;
                res.push(ty);
            }
            Self::Int(i) => {
                let (signed_state, bytes) = i.ser();

                ty |= u8::from(signed_state);

                res.push(ty);
                res.extend(bytes.iter());
            }
            Self::Imaginary(a, b) => {
                let (re_ss, re_bytes) = a.ser();
                let (im_ss, im_bytes) = b.ser();

                ty |= u8::from(re_ss);
                ty |= u8::from(im_ss) << 1;

                res.push(ty);
                res.extend(re_bytes.iter());
                res.extend(im_bytes.iter());
            }
            Self::Timestamp(t) => {
                let (year_signed_state, bytes) = t.ser();

                ty |= u8::from(year_signed_state);

                res.push(ty);
                res.extend(bytes.iter());
            }
            Self::JSON(v) => {
                res.push(ty);
                res.extend(Value::String(v.to_string()).ser()?);
            }
            Self::Null(_) => {}
            Self::Float(f) => {
                let bytes = f.to_le_bytes();
                res.push(ty);
                res.extend(bytes.iter());
            }
            Self::Map(m) => {
                if m.len() < ((1_usize << 3) - 1) {
                    ty |= (m.len() as u8) << 1;
                    res.push(ty);
                } else {
                    let (_, integer_bytes) = Integer::from(m.len()).ser();
                    ty |= 0b1; //to signify that we used an integer
                    res.push(ty);
                    res.extend(integer_bytes);
                }

                for (k, v) in m.clone() {
                    res.extend(Value::String(k).ser()?);
                    res.extend(v.ser()?);
                }
            }
            Self::Array(a) => {
                //yes, DRY, but only 2 instances right next to each other so not too bad
                if a.len() < ((1_usize << 3) - 1) {
                    ty |= (a.len() as u8) << 1;
                    res.push(ty);
                } else {
                    let (_, integer_bytes) = Integer::from(a.len()).ser();
                    ty |= 0b1; //to signify that we used an integer
                    res.push(ty);
                    res.extend(integer_bytes);
                }

                for v in a.clone() {
                    res.extend(v.ser()?);
                }
            }
            Self::Timezone(tz) => {
                let name = tz.name();
                res.push(ty);
                res.extend(Value::String(name.into()).ser()?);
            }
            Self::Ipv4Addr(a) => {
                let parts = a.octets();
                res.push(ty);
                res.extend(parts);
            }
            Self::Ipv6Addr(a) => {
                let parts: Vec<u8> = a.octets().into_iter().map(|x| x.to_le_bytes()).flatten().collect();
                res.push(ty);
                res.extend(parts);
            }
            Self::Decimal(d) => {
                res.push(ty); //TODO: explore whether i can serialise certain parts separately?
                res.extend(Value::String(d.to_string()).ser()?);
            }
        }

        Ok(res)
    }

    pub fn deser(bytes: &mut Cursor<u8>) -> Result<Self, ValueSerError> {
        let byte = bytes.next().ok_or(ValueSerError::NotEnoughBytes).copied()?;

        let ty = byte >> 4;
        let ty = ValueTy::try_from(ty)?;

        //for lengths or single integers

        Ok(match ty {
            ValueTy::Int => {
                let signed_state = SignedState::try_from(byte & 0b0000_0001)?;
                let int = Integer::deser(signed_state, bytes)?;
                Self::Int(int)
            }
            ValueTy::Imaginary => {
                let first_signed_state = SignedState::try_from(byte & 0b0000_0001)?;
                let second_signed_state = SignedState::try_from((byte & 0b0000_0010) >> 1)?;

                let a = Integer::deser(first_signed_state, bytes)?;
                let b = Integer::deser(second_signed_state, bytes)?;
                Self::Imaginary(a, b)
            }
            ValueTy::Ch => {
                let ch = char::from_u32(Integer::deser(SignedState::Positive, bytes)?.try_into()?)
                    .ok_or(ValueSerError::InvalidCharacter)?;
                Self::Ch(ch)
            }
            ValueTy::Timestamp => {
                let signed_state = SignedState::try_from(byte & 0b0000_0001)?;
                let t = Timestamp::deser(signed_state, bytes)?;
                Self::Timestamp(t)
            }
            ValueTy::String => {
                let len: usize = Integer::deser(SignedState::Positive, bytes)?.try_into()?;
                let str_bytes = bytes
                    .read(len)
                    .ok_or(ValueSerError::NotEnoughBytes)?
                    .to_vec();
                Self::String(String::from_utf8(str_bytes)?)
            }
            ValueTy::JSON => {
                let val = Value::deser(bytes)?;
                let Value::String(s) = val else {
                    return Err(ValueSerError::UnexpectedValueType(val, ValueTy::String));
                };
                let value: SJValue = serde_json::from_str(&s)?;
                Self::JSON(value)
            }
            ValueTy::Binary => {
                let len: usize = Integer::deser(SignedState::Positive, bytes)?.try_into()?;
                let bytes = bytes
                    .read(len)
                    .ok_or(ValueSerError::NotEnoughBytes)?
                    .to_vec();
                Self::Binary(bytes)
            }
            ValueTy::Bool => Self::Bool((byte & 0b0000_0100) > 0),
            ValueTy::Null => Self::Null(()),
            ValueTy::Float => {
                let bytes = match bytes.read_specific::<8>().map(TryInto::try_into) {
                    None => return Err(ValueSerError::NotEnoughBytes),
                    Some(Err(_e)) => {
                        unreachable!("Trying to get 8 bytes into 8 bytes, no?")
                    }
                    Some(Ok(b)) => b,
                };
                Self::Float(f64::from_le_bytes(bytes))
            }
            ValueTy::Map | ValueTy::Array => {
                let len: usize = {
                    if (byte & 0b0000_0001) > 0 {
                        //we used an integer
                        Integer::deser(SignedState::Positive, bytes)?.try_into()?
                    } else {
                        //we encoded it in the byte
                        ((byte & 0b0000_0110) >> 1) as usize
                    }
                };

                if ty == ValueTy::Map {
                    let mut map = HashMap::with_capacity(len);

                    for _ in 0..len {
                        let key = Value::deser(bytes)?;
                        let Value::String(key) = key else {
                            return Err(ValueSerError::UnexpectedValueType(key, ValueTy::String));
                        };
                        let value = Value::deser(bytes)?;
                        map.insert(key, value);
                    }

                    Value::Map(map)
                } else {
                    Value::Array((0..len).map(|_| Value::deser(bytes)).collect::<Result<_, _>>()?)
                }
            }
            ValueTy::Timezone => {
                let val = Value::deser(bytes)?;
                let Value::String(val) = val else {
                    return Err(ValueSerError::UnexpectedValueType(val, ValueTy::String));
                };
                let tz = Tz::from_str(&val)?;
                Self::Timezone(tz)
            }
            ValueTy::IpV4 => {
                let Some([a, b, c, d]) = bytes.read_specific::<4>() else {
                    return Err(ValueSerError::NotEnoughBytes);
                };
                Self::Ipv4Addr(Ipv4Addr::new(*a, *b, *c, *d))
            }
            ValueTy::IpV6 => {
                let Some(bytes) = bytes.read_specific::<16>() else {
                    return Err(ValueSerError::NotEnoughBytes);
                };

                let mut octets = [0_u16; 8];
                for i in (0..8_usize).map(|x| x * 2) {
                    octets[i] = u16::from_le_bytes([bytes[i], bytes[i+1]]);
                }
                let [a, b, c, d, e, f, g, h] = octets;

                Self::Ipv6Addr(Ipv6Addr::new(a, b, c, d, e, f, g, h))
            }
            ValueTy::Decimal => {
                let val = Value::deser(bytes)?;
                let Value::String(val) = val else {
                    return Err(ValueSerError::UnexpectedValueType(val, ValueTy::String));
                };
                let decimal = Decimal::from_str(&val)?;
                Value::Decimal(decimal)
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

    //TODO: tests
}
