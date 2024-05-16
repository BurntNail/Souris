use crate::{
    display_bytes_as_hex_array,
    types::{
        imaginary::Imaginary,
        integer::{Integer, IntegerSerError, SignedState},
    },
    utilities::cursor::Cursor,
};
use alloc::{
    string::{FromUtf8Error, String, ToString},
    vec,
    vec::Vec,
};
use cfg_if::cfg_if;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use chrono_tz::Tz;
use core::{
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
    net::{Ipv4Addr, Ipv6Addr},
    num::FpCategory,
    str::FromStr,
};
use hashbrown::HashMap;
use serde_json::{Error as SJError, Value as SJValue};

#[cfg(feature = "axum")]
mod axum;
#[cfg(feature = "axum")]
pub use axum::*;

#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum Value {
    Character(char),
    String(String),
    Binary(Vec<u8>),
    Boolean(bool),
    Integer(Integer),
    Imaginary(Imaginary),
    Timestamp(NaiveDateTime),
    JSON(SJValue),
    Null(()),
    SingleFloat(f32),
    DoubleFloat(f64),
    Array(Vec<Value>),
    Map(HashMap<String, Value>),
    Timezone(Tz),
    Ipv4Addr(Ipv4Addr),
    Ipv6Addr(Ipv6Addr),
}

macro_rules! as_ty {
    ($($variant:ident $name:ident -> $t:ty),+) => {
        paste::paste!{
            impl Value {
                $(
                    #[must_use]
                    pub fn [<as_ $name>] (&self) -> Option<&$t> {
                        if let Value::$variant(v) = self {
                            Some(v)
                        } else {
                            None
                        }
                    }

                    #[must_use]
                    pub fn [<as_mut_ $name>] (&mut self) -> Option<&mut $t> {
                        if let Value::$variant(v) = self {
                            Some(v)
                        } else {
                            None
                        }
                    }

                    #[must_use]
                    pub fn [<to_ $name>] (self) -> Option<$t> {
                        if let Value::$variant(v) = self {
                            Some(v)
                        } else {
                            None
                        }
                    }

                    #[must_use]
                    pub fn [<is_ $name>] (&self) -> bool {
                        matches!(self, Value::$variant(_))
                    }
                )+
            }
        }

        $(
        impl TryFrom<Value> for $t {
            type Error = ValueSerError;

            fn try_from(value: Value) -> Result<Self, Self::Error> {
                let found = value.as_ty();
                paste::paste!{
                    value.[<to_ $name>]().ok_or(ValueSerError::UnexpectedValueType(found, ValueTy::$variant))
                }
            }
        }
        )+
    };
}

as_ty!(Character char -> char, String str -> String, Boolean bool -> bool, Integer int -> Integer, Imaginary imaginary -> Imaginary, Timestamp timestamp -> NaiveDateTime, JSON json -> SJValue, Null null -> (), DoubleFloat double_float -> f64, SingleFloat single_float -> f32, Array array -> Vec<Value>, Map map -> HashMap<String, Value>, Timezone tz -> Tz, Ipv4Addr ipv4 -> Ipv4Addr, Ipv6Addr ipv6 -> Ipv6Addr, Binary binary -> Vec<u8>);

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        if self.as_ty() != other.as_ty() {
            return false;
        }

        match (self, other) {
            (Self::Character(c), Self::Character(c2)) => c.eq(c2),
            (Self::String(s), Self::String(s2)) => s.eq(s2),
            (Self::Binary(b), Self::Binary(b2)) => b.eq(b2),
            (Self::Boolean(b), Self::Boolean(b2)) => b.eq(b2),
            (Self::Integer(i), Self::Integer(i2)) => i.eq(i2),
            (Self::Imaginary(i), Self::Imaginary(i2)) => i.eq(i2),
            (Self::Timestamp(t), Self::Timestamp(t2)) => t.eq(t2),
            (Self::JSON(j), Self::JSON(j2)) => j.eq(j2),
            (Self::Null(()), Self::Null(())) => true,
            (Self::DoubleFloat(f), Self::DoubleFloat(f2)) => f.eq(f2),
            (Self::Array(a), Self::Array(a2)) => a.eq(a2),
            (Self::Map(m), Self::Map(m2)) => m.eq(m2),
            (Self::Timezone(t), Self::Timezone(t2)) => t.eq(t2),
            (Self::Ipv4Addr(t), Self::Ipv4Addr(t2)) => t.eq(t2),
            (Self::Ipv6Addr(t), Self::Ipv6Addr(t2)) => t.eq(t2),
            (Self::SingleFloat(t), Self::SingleFloat(t2)) => t.eq(t2),
            _ => unreachable!("already checked ty equality"),
        }
    }
}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Value::Character(v) => {
                v.hash(state);
            }
            Value::String(v) => {
                v.hash(state);
            }
            Value::Binary(v) => {
                v.hash(state);
            }
            Value::Boolean(v) => {
                v.hash(state);
            }
            Value::Integer(v) => {
                v.hash(state);
            }
            Value::Imaginary(i) => {
                i.hash(state);
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
            Value::DoubleFloat(f) => {
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
            Value::Null(()) => {}
            Value::Timezone(tz) => {
                tz.hash(state);
            }
            Value::Ipv4Addr(a) => {
                a.hash(state);
            }
            Value::Ipv6Addr(a) => {
                a.hash(state);
            }
            Value::SingleFloat(f) => {
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
        }
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl Debug for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut s = f.debug_struct("Value");
        s.field("ty", &self.as_ty());

        match &self {
            Self::Character(ch) => s.field("content", ch),
            Self::String(str) => s.field("content", str),
            Self::Binary(b) => s.field("content", &display_bytes_as_hex_array(b)),
            Self::Boolean(b) => s.field("content", b),
            Self::Integer(i) => s.field("content", i),
            Self::Imaginary(i) => s.field("content", i),
            Self::Timestamp(ndt) => s.field("content", ndt),
            Self::JSON(v) => s.field("content", v),
            Self::DoubleFloat(f) => s.field("content", f),
            Self::Null(o) => s.field("content", o),
            Self::Array(a) => s.field("content", a),
            Self::Map(m) => s.field("content", m),
            Self::Ipv4Addr(m) => s.field("content", m),
            Self::Ipv6Addr(m) => s.field("content", m),
            Self::SingleFloat(m) => s.field("content", m),
            Self::Timezone(m) => s.field("content", m),
        };

        s.finish()
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match &self {
            Self::Character(ch) => write!(f, "{ch:?}"),
            Self::String(str) => write!(f, "{str:?}"),
            Self::Binary(b) => {
                write!(f, "{}", display_bytes_as_hex_array(b))
            }
            Self::Boolean(b) => write!(f, "{b}"),
            Self::Integer(i) => write!(f, "{i}"),
            Self::Imaginary(i) => write!(f, "{i}"),
            Self::Timestamp(ndt) => write!(f, "{ndt}"),
            Self::JSON(v) => write!(f, "{v}"),
            Self::DoubleFloat(fl) => write!(f, "{fl}"),
            Self::Null(_o) => write!(f, "null"),
            Self::Map(m) => {
                cfg_if! {
                    if #[cfg(feature = "std")] {
                        use alloc::format;

                        let mut table = comfy_table::Table::new();
                        table
                            .set_header(vec!["Key", "Value"])
                            .load_preset(comfy_table::presets::UTF8_FULL)
                            .apply_modifier(comfy_table::modifiers::UTF8_ROUND_CORNERS)
                            .set_content_arrangement(comfy_table::ContentArrangement::Dynamic);

                        for (k, v) in m {
                            table.add_row(vec![format!("{k}"), format!("{v}")]);
                        }
                        write!(f, "\n{table}")
                    } else {
                        write!(f, "{{")?;

                        let mut first = true;
                        for (k, v) in m {
                            if first {
                                first = false;

                                write!(f, "{k}: {v}")?;
                            } else {
                                write!(f, ", {k}: {v}")?;
                            }
                        }
                        write!(f, "}}")
                    }
                }
            }
            Self::Array(a) => {
                write!(f, "[")?;
                let mut first = true;
                for v in a {
                    if first {
                        first = false;
                        write!(f, "{v}")?;
                    } else {
                        write!(f, ", {v}")?;
                    }
                }
                write!(f, "]")
            }
            Self::Timezone(v) => write!(f, "{v}"),
            Self::Ipv4Addr(v) => write!(f, "{v}"),
            Self::Ipv6Addr(v) => write!(f, "{v}"),
            Self::SingleFloat(v) => write!(f, "{v}"),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ValueTy {
    Character,
    String,
    Binary,
    Boolean,
    Integer,
    Imaginary,
    Timestamp,
    JSON,
    Null,
    DoubleFloat,
    Array,
    Map,
    Timezone,
    Ipv4Addr,
    Ipv6Addr,
    SingleFloat,
}

impl From<ValueTy> for u8 {
    fn from(value: ValueTy) -> Self {
        match value {
            ValueTy::Character => 0,
            ValueTy::String => 1,
            ValueTy::Binary => 2,
            ValueTy::Boolean => 3,
            ValueTy::Integer => 4,
            ValueTy::Imaginary => 5,
            ValueTy::Timestamp => 6,
            ValueTy::JSON => 7,
            ValueTy::Map => 8,
            ValueTy::Null => 9,
            ValueTy::DoubleFloat => 10,
            ValueTy::Array => 11,
            ValueTy::Timezone => 12,
            ValueTy::Ipv4Addr => 13,
            ValueTy::Ipv6Addr => 14,
            ValueTy::SingleFloat => 15,
        }
    }
}
impl TryFrom<u8> for ValueTy {
    type Error = ValueSerError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => ValueTy::Character,
            1 => ValueTy::String,
            2 => ValueTy::Binary,
            3 => ValueTy::Boolean,
            4 => ValueTy::Integer,
            5 => ValueTy::Imaginary,
            6 => ValueTy::Timestamp,
            7 => ValueTy::JSON,
            8 => ValueTy::Map,
            9 => ValueTy::Null,
            10 => ValueTy::DoubleFloat,
            11 => ValueTy::Array,
            12 => ValueTy::Timezone,
            13 => ValueTy::Ipv4Addr,
            14 => ValueTy::Ipv6Addr,
            15 => ValueTy::SingleFloat,
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
    TooManyBytes,
    InvalidCharacter,
    NonUTF8String(FromUtf8Error),
    SerdeJson(SJError),
    UnexpectedValueType(ValueTy, ValueTy),
    TzError(chrono_tz::ParseError),
    InvalidDateOrTime,
    #[cfg(feature = "serde")]
    SerdeCustom(String),
}

impl Display for ValueSerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            #[cfg(feature = "serde")]
            ValueSerError::SerdeCustom(s) => write!(f, "Serde Error: {s}"),
            ValueSerError::InvalidType(b) => write!(f, "Invalid Type Discriminant found: {b:#b}"),
            ValueSerError::Empty => write!(
                f,
                "Length provided was zero - what did you expect to deserialise there?"
            ),
            ValueSerError::IntegerSerError(e) => write!(f, "Error de/ser-ing integer: {e}"),
            ValueSerError::NotEnoughBytes => write!(f, "Not enough bytes provided"),
            ValueSerError::TooManyBytes => write!(f, "Extra bytes provided"),
            ValueSerError::InvalidCharacter => write!(f, "Invalid character provided"),
            ValueSerError::NonUTF8String(e) => write!(f, "Error converting to UTF-8: {e}"),
            ValueSerError::SerdeJson(e) => write!(f, "Error de/ser-ing serde_json: {e}"),
            ValueSerError::UnexpectedValueType(found, ex) => {
                write!(f, "Expected {ex:?}, found: {found:?}")
            }
            ValueSerError::TzError(e) => write!(f, "Error parsing timezone: {e}"),
            ValueSerError::InvalidDateOrTime => write!(f, "Error with invalid time given"),
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

#[cfg(feature = "std")]
impl std::error::Error for ValueSerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ValueSerError::IntegerSerError(e) => Some(e),
            ValueSerError::NonUTF8String(e) => Some(e),
            ValueSerError::SerdeJson(e) => Some(e),
            ValueSerError::TzError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<SJValue> for Value {
    fn from(v: SJValue) -> Self {
        match v {
            SJValue::Null => Value::Null(()),
            SJValue::Bool(b) => Value::Boolean(b),
            SJValue::Number(n) => {
                if let Some(neg) = n.as_i64() {
                    Value::Integer(Integer::i64(neg))
                } else if let Some(pos) = n.as_u64() {
                    Value::Integer(Integer::u64(pos))
                } else if let Some(float) = n.as_f64() {
                    Value::DoubleFloat(float)
                } else {
                    unreachable!("must be one of the three JSON number types")
                }
            }
            SJValue::String(s) => Value::String(s.to_string()),
            SJValue::Array(a) => Value::Array(a.into_iter().map(Self::from).collect()),
            SJValue::Object(o) => Value::Map(o.into_iter().map(|(k, v)| (k, v.into())).collect()),
        }
    }
}

impl Value {
    pub(crate) const fn as_ty(&self) -> ValueTy {
        match self {
            Self::Character(_) => ValueTy::Character,
            Self::String(_) => ValueTy::String,
            Self::Binary(_) => ValueTy::Binary,
            Self::Boolean(_) => ValueTy::Boolean,
            Self::Integer(_) => ValueTy::Integer,
            Self::Imaginary(_) => ValueTy::Imaginary,
            Self::Timestamp(_) => ValueTy::Timestamp,
            Self::JSON(_) => ValueTy::JSON,
            Self::Map(_) => ValueTy::Map,
            Self::Array(_) => ValueTy::Array,
            Self::DoubleFloat(_) => ValueTy::DoubleFloat,
            Self::Null(()) => ValueTy::Null,
            Self::Timezone(_) => ValueTy::Timezone,
            Self::Ipv4Addr(_) => ValueTy::Ipv4Addr,
            Self::Ipv6Addr(_) => ValueTy::Ipv6Addr,
            Self::SingleFloat(_) => ValueTy::SingleFloat,
        }
    }

    pub(crate) fn deser_array_or_map_len(
        byte: u8,
        input: &mut Cursor<u8>,
        expected_type: ValueTy,
    ) -> Result<usize, ValueSerError> {
        let ty = ValueTy::try_from((byte & 0b1111_0000) >> 4)?;
        if ty == expected_type {
            let len = {
                if (byte & 0b0000_0001) > 0 {
                    // we used an integer
                    Integer::deser(SignedState::Positive, input)?.try_into()?
                } else {
                    //we encoded it in the byte
                    ((byte & 0b0000_1110) >> 1) as usize
                }
            };

            Ok(len)
        } else {
            Err(ValueSerError::UnexpectedValueType(ty, expected_type))
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn ser(&self) -> Result<Vec<u8>, ValueSerError> {
        let mut res = vec![];

        let mut ty = u8::from(self.as_ty()) << 4;

        match self {
            Self::Character(ch) => {
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
            Self::Boolean(b) => {
                ty |= u8::from(*b);
                res.push(ty);
            }
            Self::Integer(i) => {
                let (signed_state, bytes) = i.ser();

                ty |= u8::from(signed_state);

                res.push(ty);
                res.extend(bytes.iter());
            }
            Self::Imaginary(i) => {
                let (re_ss, im_ss, bytes) = i.ser();

                ty |= u8::from(re_ss);
                ty |= u8::from(im_ss) << 1;

                res.push(ty);
                res.extend(bytes);
            }
            Self::Timestamp(t) => {
                let date = t.date();
                let (year_ss, year) = Integer::from(date.year()).ser();
                let (_, month) = Integer::from(date.month()).ser();
                let (_, day) = Integer::from(date.day()).ser();

                let time = t.time();
                let (_, hour) = Integer::from(time.hour()).ser();
                let (_, minute) = Integer::from(time.minute()).ser();
                let (_, sec) = Integer::from(time.second()).ser();
                let (_, nanos) = Integer::from(time.nanosecond()).ser();

                ty |= u8::from(year_ss);

                res.push(ty);

                res.extend(year.iter());
                res.extend(month.iter());
                res.extend(day.iter());
                res.extend(hour.iter());
                res.extend(minute.iter());
                res.extend(sec.iter());
                res.extend(nanos.iter());
            }
            Self::JSON(v) => {
                res.push(ty);
                res.extend(Value::String(v.to_string()).ser()?);
            }
            Self::Null(()) => {
                res.push(ty);
            }
            Self::SingleFloat(f) => {
                res.push(ty);
                res.extend(f.to_le_bytes());
            }
            Self::DoubleFloat(f) => {
                res.push(ty);
                res.extend(f.to_le_bytes());
            }
            Self::Map(m) => {
                #[allow(clippy::cast_possible_truncation)]
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
                // yes, DRY, but only 2 instances right next to each other so not too bad
                #[allow(clippy::cast_possible_truncation)]
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
                res.push(ty);
                res.extend(a.octets());
            }
            Self::Ipv6Addr(a) => {
                res.push(ty);
                res.extend(a.octets());
            }
        }

        Ok(res)
    }

    #[allow(clippy::many_single_char_names, clippy::too_many_lines)]
    pub fn deser(bytes: &mut Cursor<u8>) -> Result<Self, ValueSerError> {
        let byte = bytes.next().ok_or(ValueSerError::NotEnoughBytes).copied()?;

        let ty = (byte & 0b1111_0000) >> 4;
        let ty = ValueTy::try_from(ty)?;

        //for lengths or single integers

        Ok(match ty {
            ValueTy::Integer => {
                let signed_state = SignedState::try_from(byte & 0b0000_0001)?;
                let int = Integer::deser(signed_state, bytes)?;
                Self::Integer(int)
            }
            ValueTy::Imaginary => {
                let first_signed_state = SignedState::try_from(byte & 0b0000_0001)?;
                let second_signed_state = SignedState::try_from((byte & 0b0000_0010) >> 1)?;

                Self::Imaginary(Imaginary::deser(
                    first_signed_state,
                    second_signed_state,
                    bytes,
                )?)
            }
            ValueTy::Character => {
                let ch = char::from_u32(Integer::deser(SignedState::Positive, bytes)?.try_into()?)
                    .ok_or(ValueSerError::InvalidCharacter)?;
                Self::Character(ch)
            }
            ValueTy::Timestamp => {
                let year_signed_state = SignedState::try_from(byte & 0b0000_0001)?;

                let year = Integer::deser(year_signed_state, bytes)?.try_into()?;
                let month = Integer::deser(SignedState::Positive, bytes)?.try_into()?;
                let day = Integer::deser(SignedState::Positive, bytes)?.try_into()?;

                let date = NaiveDate::from_ymd_opt(year, month, day)
                    .ok_or(ValueSerError::InvalidDateOrTime)?;

                let hour = Integer::deser(SignedState::Positive, bytes)?.try_into()?;
                let min = Integer::deser(SignedState::Positive, bytes)?.try_into()?;
                let sec = Integer::deser(SignedState::Positive, bytes)?.try_into()?;
                let ns = Integer::deser(SignedState::Positive, bytes)?.try_into()?;

                let time = NaiveTime::from_hms_nano_opt(hour, min, sec, ns)
                    .ok_or(ValueSerError::InvalidDateOrTime)?;

                Self::Timestamp(NaiveDateTime::new(date, time))
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
                    return Err(ValueSerError::UnexpectedValueType(
                        val.as_ty(),
                        ValueTy::String,
                    ));
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
            ValueTy::Boolean => Self::Boolean((byte & 0b0000_0001) > 0),
            ValueTy::Null => Self::Null(()),
            ValueTy::SingleFloat => {
                let Some(bytes) = bytes.read_specific() else {
                    return Err(ValueSerError::NotEnoughBytes);
                };
                Self::SingleFloat(f32::from_le_bytes(*bytes))
            }
            ValueTy::DoubleFloat => {
                let Some(bytes) = bytes.read_specific() else {
                    return Err(ValueSerError::NotEnoughBytes);
                };
                Self::DoubleFloat(f64::from_le_bytes(*bytes))
            }
            ValueTy::Map => {
                let len = Self::deser_array_or_map_len(byte, bytes, ty)?;

                let mut map = HashMap::with_capacity(len);

                for _ in 0..len {
                    let key = Value::deser(bytes)?;
                    let Value::String(key) = key else {
                        return Err(ValueSerError::UnexpectedValueType(
                            key.as_ty(),
                            ValueTy::String,
                        ));
                    };
                    let value = Value::deser(bytes)?;
                    map.insert(key, value);
                }

                Value::Map(map)
            }
            ValueTy::Array => {
                let len = Self::deser_array_or_map_len(byte, bytes, ty)?;

                Value::Array(
                    (0..len)
                        .map(|_| Value::deser(bytes))
                        .collect::<Result<_, _>>()?,
                )
            }
            ValueTy::Timezone => {
                let val = Value::deser(bytes)?;
                let Value::String(val) = val else {
                    return Err(ValueSerError::UnexpectedValueType(
                        val.as_ty(),
                        ValueTy::String,
                    ));
                };
                let tz = Tz::from_str(&val)?;
                Self::Timezone(tz)
            }
            ValueTy::Ipv4Addr => {
                let Some([a, b, c, d]) = bytes.read_specific() else {
                    return Err(ValueSerError::NotEnoughBytes);
                };
                Self::Ipv4Addr(Ipv4Addr::new(*a, *b, *c, *d))
            }
            ValueTy::Ipv6Addr => {
                let Some(bytes) = bytes.read_specific::<16>() else {
                    return Err(ValueSerError::NotEnoughBytes);
                };

                let mut octets = [0_u16; 8];
                for i in (0..8_usize).map(|x| x * 2) {
                    octets[i / 2] = u16::from_le_bytes([bytes[i], bytes[i + 1]]);
                }
                let [a, b, c, d, e, f, g, h] = octets;

                Self::Ipv6Addr(Ipv6Addr::new(a, b, c, d, e, f, g, h))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Value;
    use crate::{
        types::integer::{BiggestInt, BiggestIntButSigned},
        utilities::cursor::Cursor,
    };
    use alloc::{
        format,
        string::{String, ToString},
        vec::Vec,
    };
    use proptest::{arbitrary::any, prop_assert_eq, proptest};

    proptest! {
        #[test]
        fn test_ch (c in any::<char>()) {
            let v = Value::Character(c);

            let bytes = v.ser().unwrap();
            let out_value = Value::deser(&mut Cursor::new(&bytes)).unwrap();
            let out = out_value.to_char().unwrap();

            prop_assert_eq!(c, out);
        }

        #[test]
        fn test_str (s in any::<String>()) {
            let v = Value::String(s.clone());

            let bytes = v.ser().unwrap();
            let out_value = Value::deser(&mut Cursor::new(&bytes)).unwrap();
            let out = out_value.as_str().unwrap().to_string();

            prop_assert_eq!(s, out);
        }

        #[test]
        fn test_bin (s in any::<Vec<u8>>()) {
            let v = Value::Binary(s.clone());

            let bytes = v.ser().unwrap();
            let out_value = Value::deser(&mut Cursor::new(&bytes)).unwrap();
            let out = out_value.as_binary().unwrap().to_vec();

            prop_assert_eq!(s, out);
        }

        #[test]
        fn test_bool (s in any::<bool>()) {
            let v = Value::Boolean(s.clone());

            let bytes = v.ser().unwrap();
            let out_value = Value::deser(&mut Cursor::new(&bytes)).unwrap();
            let out = out_value.to_bool().unwrap();

            prop_assert_eq!(s, out);
        }

        #[test]
        fn test_int (a in any::<BiggestInt>(), b in any::<BiggestIntButSigned>()) {
            {
                let v = Value::Integer(a.clone().into());

                let bytes = v.ser().unwrap();
                let out_value = Value::deser(&mut Cursor::new(&bytes)).unwrap();
                let out = BiggestInt::try_from(out_value.to_int().unwrap()).unwrap();

                prop_assert_eq!(a, out);
            }
            {
                let v = Value::Integer(b.clone().into());

                let bytes = v.ser().unwrap();
                let out_value = Value::deser(&mut Cursor::new(&bytes)).unwrap();
                let out = BiggestIntButSigned::try_from(out_value.to_int().unwrap()).unwrap();

                prop_assert_eq!(b, out);
            }
        }

        //TODO: more tests :)
    }
}
