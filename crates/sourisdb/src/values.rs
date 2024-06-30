use alloc::{
    string::{FromUtf8Error, String, ToString},
    vec,
    vec::Vec,
};
use core::{
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
    net::{Ipv4Addr, Ipv6Addr},
    num::FpCategory,
    str::FromStr,
};

use cfg_if::cfg_if;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use chrono_tz::Tz;
use hashbrown::HashMap;
use serde_json::{Error as SJError, Map as SJMap, Number, Value as SJValue};

use crate::{
    display_bytes_as_hex_array,
    types::{
        imaginary::Imaginary,
        integer::{Integer, IntegerSerError, SignedState},
    },
    utilities::{bits::Bits, cursor::Cursor, huffman::Huffman},
};

#[derive(Clone)]
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

macro_rules! from_integer {
    ($($t:ty),+) => {
        $(
            impl From<$t> for Value {
                fn from (int: $t) -> Value {
                    Value::Integer(Integer::from(int))
                }
            }

            impl TryFrom<Value> for $t {
                type Error = ValueSerError;

                fn try_from (value: Value) -> Result<Self, Self::Error> {
                    Ok(<$t>::try_from(Integer::try_from(value)?)?)
                }
            }
        )+
    };
}

from_integer!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

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
    NoHuffman,
    UnableToDecodeHuffman,
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
            ValueSerError::NoHuffman => write!(
                f,
                "Encountered huffman-encoded string with no huffman tree provided"
            ),
            ValueSerError::UnableToDecodeHuffman => {
                write!(
                    f,
                    "Encountered huffman-encoded string but was unable to decode it"
                )
            }
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

impl Value {
    ///if it is an integer outside the bounds of [`i64::MIN`] to [`u64::MAX`], then it will fail. it will also fail if it was a float that wasn't NaN or infinity
    #[allow(clippy::too_many_lines)]
    pub fn convert_to_json(self) -> Option<SJValue> {
        Some(match self {
            Value::Character(c) => SJValue::String(c.into()),
            Value::String(s) => SJValue::String(s),
            Value::Boolean(b) => SJValue::Bool(b),
            Value::Integer(i) => i.to_json()?,
            Value::JSON(j) => j,
            Value::Null(()) => SJValue::Null,
            Value::SingleFloat(f) => SJValue::Number(Number::from_f64(f64::from(f))?),
            Value::DoubleFloat(f) => SJValue::Number(Number::from_f64(f)?),
            Value::Array(arr) => SJValue::Array(
                arr.into_iter()
                    .map(Value::convert_to_json)
                    .collect::<Option<Vec<_>>>()?,
            ),
            Value::Map(m) => SJValue::Object(
                m.into_iter()
                    .map(|(k, v)| Value::convert_to_json(v).map(|v| (k, v)))
                    .collect::<Option<SJMap<_, _>>>()?,
            ),
            Value::Imaginary(im) => {
                let mut obj = SJMap::new();
                obj.insert(
                    "souris_type".into(),
                    SJValue::Number(Number::from(u8::from(ValueTy::Imaginary))),
                );

                match im {
                    Imaginary::CartesianForm { real, imaginary } => {
                        obj.insert("real".into(), real.to_json()?);
                        obj.insert("imaginary".into(), imaginary.to_json()?);
                    }
                    Imaginary::PolarForm { modulus, argument } => {
                        let real = modulus * argument.cos();
                        let imaginary = modulus * argument.sin();

                        let to_json = |float| {
                            if let Some(n) = Number::from_f64(float) {
                                SJValue::Number(n)
                            } else {
                                SJValue::Number(Number::from(0))
                            }
                        };

                        obj.insert("real".into(), to_json(real));
                        obj.insert("imaginary".into(), to_json(imaginary));
                    }
                }

                SJValue::Object(obj)
            }
            Value::Timestamp(ts) => {
                let mut obj = SJMap::new();
                obj.insert(
                    "souris_type".into(),
                    SJValue::Number(Number::from(u8::from(ValueTy::Timestamp))),
                );

                obj.insert("timestamp".into(), SJValue::String(ts.to_string()));

                SJValue::Object(obj)
            }
            Value::Timezone(tz) => {
                let mut obj = SJMap::new();
                obj.insert(
                    "souris_type".into(),
                    SJValue::Number(Number::from(u8::from(ValueTy::Timezone))),
                );

                obj.insert("timezone".into(), SJValue::String(tz.to_string()));

                SJValue::Object(obj)
            }
            Value::Binary(b) => {
                let mut obj = SJMap::new();
                obj.insert(
                    "souris_type".into(),
                    SJValue::Number(Number::from(u8::from(ValueTy::Binary))),
                );

                obj.insert(
                    "bytes".into(),
                    SJValue::Array(
                        b.into_iter()
                            .map(|n| SJValue::Number(Number::from(n)))
                            .collect(),
                    ),
                );

                SJValue::Object(obj)
            }
            Value::Ipv4Addr(a) => {
                let mut obj = SJMap::new();
                obj.insert(
                    "souris_type".into(),
                    SJValue::Number(Number::from(u8::from(ValueTy::Ipv4Addr))),
                );

                obj.insert(
                    "octets".into(),
                    SJValue::Array(
                        a.octets()
                            .into_iter()
                            .map(|o| SJValue::Number(Number::from(o)))
                            .collect(),
                    ),
                );

                SJValue::Object(obj)
            }
            Value::Ipv6Addr(a) => {
                let mut obj = SJMap::new();
                obj.insert(
                    "souris_type".into(),
                    SJValue::Number(Number::from(u8::from(ValueTy::Ipv6Addr))),
                );

                obj.insert(
                    "octets".into(),
                    SJValue::Array(
                        a.segments()
                            .into_iter()
                            .map(|o| SJValue::Number(Number::from(o)))
                            .collect(),
                    ),
                );

                SJValue::Object(obj)
            }
        })
    }

    #[allow(clippy::too_many_lines)]
    pub fn convert_from_json(val: SJValue) -> Self {
        match val {
            SJValue::Null => Self::Null(()),
            SJValue::Bool(b) => Self::Boolean(b),
            SJValue::Number(n) => {
                if let Some(i) = Integer::from_json(&n) {
                    Self::Integer(i)
                } else {
                    let Some(float) = n.as_f64() else {
                        unreachable!("just checked if was integer");
                    };
                    Self::DoubleFloat(float)
                }
            }
            SJValue::String(s) => Value::String(s),
            SJValue::Array(a) => {
                Value::Array(a.into_iter().map(Value::convert_from_json).collect())
            }
            SJValue::Object(obj) => {
                if let Some(SJValue::Number(n)) = obj.get("souris_type").cloned() {
                    if let Some(ty) = n
                        .as_u64()
                        .map(u8::try_from)
                        .and_then(Result::ok)
                        .map(ValueTy::try_from)
                        .and_then(Result::ok)
                    {
                        match ty {
                            ValueTy::Imaginary => {
                                if let Some((SJValue::Number(real), SJValue::Number(imaginary))) =
                                    obj.get("real").cloned().zip(obj.get("imaginary").cloned())
                                {
                                    if let Some((real, imaginary)) = Integer::from_json(&real)
                                        .zip(Integer::from_json(&imaginary))
                                    {
                                        return Value::Imaginary(Imaginary::CartesianForm {
                                            real,
                                            imaginary,
                                        });
                                    }

                                    if let Some((real, imaginary)) =
                                        real.as_f64().zip(imaginary.as_f64())
                                    {
                                        return Value::Imaginary(Imaginary::polar_from_cartesian(
                                            real, imaginary,
                                        ));
                                    }
                                }
                            }
                            ValueTy::Timestamp => {
                                if let Some(SJValue::String(timestamp)) = obj.get("timestamp") {
                                    if let Ok(timestamp) = NaiveDateTime::from_str(timestamp) {
                                        return Value::Timestamp(timestamp);
                                    }
                                }
                            }
                            ValueTy::Timezone => {
                                if let Some(SJValue::String(tz)) = obj.get("timezone") {
                                    if let Ok(tz) = Tz::from_str(tz) {
                                        return Value::Timezone(tz);
                                    }
                                }
                            }
                            ValueTy::Binary => {
                                if let Some(SJValue::Array(bytes)) = obj.get("bytes") {
                                    if let Some(bytes) = bytes
                                        .iter()
                                        .map(|x| x.as_u64().and_then(|x| u8::try_from(x).ok()))
                                        .collect::<Option<Vec<_>>>()
                                    {
                                        return Value::Binary(bytes);
                                    }
                                }
                            }
                            ValueTy::Ipv4Addr => {
                                if let Some(SJValue::Array(bytes)) = obj.get("octets") {
                                    if let Some([a, b, c, d]) = bytes
                                        .iter()
                                        .map(|x| x.as_u64().and_then(|x| u8::try_from(x).ok()))
                                        .collect::<Option<Vec<_>>>()
                                        .and_then(|x| <[u8; 4]>::try_from(x).ok())
                                    {
                                        return Value::Ipv4Addr(Ipv4Addr::new(a, b, c, d));
                                    }
                                }
                            }
                            ValueTy::Ipv6Addr => {
                                if let Some(SJValue::Array(bytes)) = obj.get("octets") {
                                    if let Some([a, b, c, d, e, f, g, h]) = bytes
                                        .iter()
                                        .map(|x| x.as_u64().and_then(|x| u16::try_from(x).ok()))
                                        .collect::<Option<Vec<_>>>()
                                        .and_then(|x| <[u16; 8]>::try_from(x).ok())
                                    {
                                        return Value::Ipv6Addr(Ipv6Addr::new(
                                            a, b, c, d, e, f, g, h,
                                        ));
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }

                Self::Map(
                    obj.into_iter()
                        .map(|(k, v)| (k, Value::convert_from_json(v)))
                        .collect(),
                )
            }
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
                    Integer::deser(SignedState::Unsigned, input)?.try_into()?
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
    pub fn ser(&self, huffman: Option<&Huffman<char>>) -> Result<Vec<u8>, ValueSerError> {
        let mut res = vec![];

        let mut ty = u8::from(self.as_ty()) << 4;

        match self {
            Self::Character(ch) => {
                let (_, bytes) = Integer::from(*ch as u32).ser();

                res.push(ty);
                res.extend(bytes.iter());
            }
            Self::String(s) => {
                let huffman_encoded = huffman.and_then(|x| x.encode_string(s)); //unlikely to not be able to encode, but just in case ;)

                if let Some(huffman_encoded) = huffman_encoded {
                    let sered = huffman_encoded.ser();

                    ty |= 1;
                    res.push(ty);
                    res.extend(sered);
                } else {
                    let str_bytes = s.as_bytes();
                    let (_, len_bytes) = Integer::from(str_bytes.len()).ser();

                    res.push(ty);
                    res.extend(len_bytes.iter());
                    res.extend(str_bytes.iter());
                }
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
                let (magic_bits, bytes) = i.ser();

                ty |= magic_bits;

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
                res.extend(Value::String(v.to_string()).ser(huffman)?);
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
                    res.extend(Value::String(k).ser(huffman)?);
                    res.extend(v.ser(huffman)?);
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
                    res.extend(v.ser(huffman)?);
                }
            }
            Self::Timezone(tz) => {
                let name = tz.name();
                res.push(ty);
                res.extend(Value::String(name.into()).ser(huffman)?);
            }
            Self::Ipv4Addr(a) => {
                res.push(ty);
                res.extend(a.octets());
            }
            Self::Ipv6Addr(a) => {
                res.push(ty);
                res.extend(a.segments().into_iter().flat_map(u16::to_le_bytes));
            }
        }

        Ok(res)
    }

    #[allow(clippy::many_single_char_names, clippy::too_many_lines)]
    pub fn deser(
        bytes: &mut Cursor<u8>,
        huffman: Option<&Huffman<char>>,
    ) -> Result<Self, ValueSerError> {
        let byte = bytes.next().ok_or(ValueSerError::NotEnoughBytes).copied()?;

        let ty = (byte & 0b1111_0000) >> 4;
        let ty = ValueTy::try_from(ty)?;

        //for lengths or single integers

        Ok(match ty {
            ValueTy::Integer => {
                let signed_state = SignedState::try_from(byte & 0b0000_0011)?;
                let int = Integer::deser(signed_state, bytes)?;
                Self::Integer(int)
            }
            ValueTy::Imaginary => {
                let magic_bits = byte & 0b0000_1111;

                Self::Imaginary(Imaginary::deser(magic_bits, bytes)?)
            }
            ValueTy::Character => {
                let ch = char::from_u32(Integer::deser(SignedState::Unsigned, bytes)?.try_into()?)
                    .ok_or(ValueSerError::InvalidCharacter)?;
                Self::Character(ch)
            }
            ValueTy::Timestamp => {
                let year_signed_state = SignedState::try_from(byte & 0b0000_0001)?;

                let year = Integer::deser(year_signed_state, bytes)?.try_into()?;
                let month = Integer::deser(SignedState::Unsigned, bytes)?.try_into()?;
                let day = Integer::deser(SignedState::Unsigned, bytes)?.try_into()?;

                let date = NaiveDate::from_ymd_opt(year, month, day)
                    .ok_or(ValueSerError::InvalidDateOrTime)?;

                let hour = Integer::deser(SignedState::Unsigned, bytes)?.try_into()?;
                let min = Integer::deser(SignedState::Unsigned, bytes)?.try_into()?;
                let sec = Integer::deser(SignedState::Unsigned, bytes)?.try_into()?;
                let ns = Integer::deser(SignedState::Unsigned, bytes)?.try_into()?;

                let time = NaiveTime::from_hms_nano_opt(hour, min, sec, ns)
                    .ok_or(ValueSerError::InvalidDateOrTime)?;

                Self::Timestamp(NaiveDateTime::new(date, time))
            }
            ValueTy::String => {
                if (byte & 0b1) > 0 {
                    //huffman-encoded
                    let Some(huffman) = huffman else {
                        return Err(ValueSerError::NoHuffman);
                    };
                    let bits = Bits::deser(bytes)?;
                    let Some(decoded) = huffman.decode_string(bits) else {
                        return Err(ValueSerError::UnableToDecodeHuffman);
                    };

                    Self::String(decoded)
                } else {
                    let len: usize = Integer::deser(SignedState::Unsigned, bytes)?.try_into()?;
                    let str_bytes = bytes
                        .read(len)
                        .ok_or(ValueSerError::NotEnoughBytes)?
                        .to_vec();
                    Self::String(String::from_utf8(str_bytes)?)
                }
            }
            ValueTy::JSON => {
                let val = Value::deser(bytes, huffman)?;
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
                let len: usize = Integer::deser(SignedState::Unsigned, bytes)?.try_into()?;
                let bytes = bytes
                    .read(len)
                    .ok_or(ValueSerError::NotEnoughBytes)?
                    .to_vec();
                Self::Binary(bytes)
            }
            ValueTy::Boolean => Self::Boolean((byte & 0b0000_0001) > 0),
            ValueTy::Null => Self::Null(()),
            ValueTy::SingleFloat => {
                let Some(bytes) = bytes.read_exact() else {
                    return Err(ValueSerError::NotEnoughBytes);
                };
                Self::SingleFloat(f32::from_le_bytes(*bytes))
            }
            ValueTy::DoubleFloat => {
                let Some(bytes) = bytes.read_exact() else {
                    return Err(ValueSerError::NotEnoughBytes);
                };
                Self::DoubleFloat(f64::from_le_bytes(*bytes))
            }
            ValueTy::Map => {
                let len = Self::deser_array_or_map_len(byte, bytes, ty)?;

                let mut map = HashMap::with_capacity(len);

                for _ in 0..len {
                    let key = Value::deser(bytes, huffman)?;
                    let Value::String(key) = key else {
                        return Err(ValueSerError::UnexpectedValueType(
                            key.as_ty(),
                            ValueTy::String,
                        ));
                    };
                    let value = Value::deser(bytes, huffman)?;
                    map.insert(key, value);
                }

                Value::Map(map)
            }
            ValueTy::Array => {
                let len = Self::deser_array_or_map_len(byte, bytes, ty)?;

                Value::Array(
                    (0..len)
                        .map(|_| Value::deser(bytes, huffman))
                        .collect::<Result<_, _>>()?,
                )
            }
            ValueTy::Timezone => {
                let val = Value::deser(bytes, huffman)?;
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
                let Some([a, b, c, d]) = bytes.read_exact() else {
                    return Err(ValueSerError::NotEnoughBytes);
                };
                Self::Ipv4Addr(Ipv4Addr::new(*a, *b, *c, *d))
            }
            ValueTy::Ipv6Addr => {
                let Some(bytes) = bytes.read_exact::<16>() else {
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
    use alloc::{
        format,
        string::{String, ToString},
        vec::Vec,
    };

    use proptest::{arbitrary::any, prop_assert_eq, proptest};

    use crate::{
        types::{imaginary::Imaginary, integer::BiggestIntButSigned},
        utilities::cursor::Cursor,
    };

    use super::Value;

    proptest! {
        #[test]
        fn test_ch (c in any::<char>()) {
            let v = Value::Character(c);

            let bytes = v.ser(None).unwrap();
            let out_value = Value::deser(&mut Cursor::new(&bytes), None).unwrap();
            let out = out_value.to_char().unwrap();

            prop_assert_eq!(c, out);
        }

        #[test]
        fn test_str (s in any::<String>()) {
            let v = Value::String(s.clone());

            let bytes = v.ser(None).unwrap();
            let out_value = Value::deser(&mut Cursor::new(&bytes), None).unwrap();
            let out = out_value.as_str().unwrap().to_string();

            prop_assert_eq!(s, out);
        }

        #[test]
        fn test_bin (s in any::<Vec<u8>>()) {
            let v = Value::Binary(s.clone());

            let bytes = v.ser(None).unwrap();
            let out_value = Value::deser(&mut Cursor::new(&bytes), None).unwrap();
            let out = out_value.as_binary().unwrap().to_vec();

            prop_assert_eq!(s, out);
        }

        #[test]
        fn test_bool (s in any::<bool>()) {
            let v = Value::Boolean(s.clone());

            let bytes = v.ser(None).unwrap();
            let out_value = Value::deser(&mut Cursor::new(&bytes), None).unwrap();
            let out = out_value.to_bool().unwrap();

            prop_assert_eq!(s, out);
        }

        #[test]
        fn test_polar_form_ser (modulus in any::<f64>(), argument in any::<f64>()) {
            let modulus = if modulus == -0.0 {
                0.0
            } else {modulus};

            let val = Value::Imaginary(Imaginary::PolarForm { modulus, argument });

            let bytes = val.ser(None).unwrap();
            let out_value = Value::deser(&mut Cursor::new(&bytes), None).unwrap();
            let Some(Imaginary::PolarForm { modulus: nm, argument: na }) = out_value.to_imaginary() else {
                panic!("unable to get out in correct form")
            };

            assert!((modulus -  nm).abs() < f64::EPSILON);
            assert!((argument - na).abs() < f64::EPSILON);
        }

        #[test]
        fn test_int (i in any::<BiggestIntButSigned>()) {
            let v = Value::Integer(i.into());

            let bytes = v.ser(None).unwrap();
            let out_value = Value::deser(&mut Cursor::new(&bytes), None).unwrap();
            prop_assert_eq!(v, out_value.clone());

            let out = BiggestIntButSigned::try_from(out_value.to_int().unwrap()).unwrap();

            prop_assert_eq!(out, i);
        }

        //TODO: more tests :)
    }
}
