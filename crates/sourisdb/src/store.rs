//! Provides the main key-value store designed to be used for communications.

use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::{
    fmt::{Display, Formatter},
    ops::{Deref, DerefMut},
};

use hashbrown::HashMap;
use serde_json::{Error as SJError, Value as SJValue};

use crate::{
    types::{
        binary::{BinaryCompression, BinaryData, BinarySerError},
        integer::IntegerSerError,
    },
    utilities::{
        cursor::Cursor,
        huffman::{Huffman, HuffmanSerError},
    },
    values::{Value, ValueSerError, ValueTy},
};

///A key-value store where the keys are [`String`]s and the values are [`Value`]s - this is a thin wrapper around [`hashbrown::HashMap`] and implements both [`Deref`] and [`DerefMut`] pointing to it. This database is optimised for storage when serialised.
///
/// The expectation is that if you need an in-memory key-value database, you do one of two things:
/// - Spin up a server running `sourisd` and make HTTP requests to it. Then, serialise or deserialise the values appropriately.
/// - Create a `Store` and keep it in the state of your program. To access values just use it as a [`hashbrown::HashMap`]. When your program exits (or periodically to allow for if the program quits unexpectedly), serialise the database and write it to a file. Then, when starting the program again read the database in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Store(HashMap<String, Value>);

impl Store {
    ///Serialises a store into bytes. There are 8 magic bytes at the front which read `SOURISDB` and the rest is serialised as a [`Value::Map`] containing the map stored within the caller.
    ///
    /// # Errors
    /// - [`ValueSerError`] if there is an error serialising the internal map as a [`Value::Map`]
    pub fn ser(&self) -> Result<Vec<u8>, StoreSerError> {
        fn add_value_text_to_string(value: &Value, string: &mut String) {
            match value {
                Value::Map(map) => {
                    for (k, v) in map {
                        string.push_str(k);
                        add_value_text_to_string(v, string);
                    }
                }
                Value::Array(a) => {
                    for v in a {
                        add_value_text_to_string(v, string);
                    }
                }
                Value::JSON(sjv) => {
                    string.push_str(&sjv.to_string());
                }
                Value::Timezone(tz) => {
                    string.push_str(tz.name());
                }
                Value::String(s) => string.push_str(s),
                _ => {}
            }
        }

        let raw_map = Value::Map(self.0.clone());
        let mut all_text = String::new();
        add_value_text_to_string(&raw_map, &mut all_text);

        let huffman = Huffman::new_str(&all_text);
        let map = raw_map.ser(huffman.as_ref());

        let huffman_exists = huffman.is_some();
        let mut res = if let Some(huffman) = huffman {
            huffman.ser()
        } else {
            vec![]
        };
        res.extend(&map);

        let (compression_type, compressed) = BinaryData(res).ser();

        let magic_ty = (u8::from(huffman_exists) << 7) | u8::from(compression_type);

        let mut fin = vec![];
        fin.extend(b"SOURISDB");
        fin.push(magic_ty);
        fin.extend(compressed);

        Ok(fin)
    }

    /// Deserialises bytes (which must require the magic bytes) into a Store.
    ///
    /// # Errors
    /// - [`StoreSerError::NotEnoughBytes`] if we can't read enough bytes.
    /// - [`StoreSerError::ExpectedMagicBytes`] if we don't find the magic bytes.
    /// - [`BinarySerError`] if we cannot work out which binary compression type was used, or there's an error deserialising the binary.
    /// - [`HuffmanSerError`] if we cannot deserialise anything huffman related
    /// - [`ValueSerError`] if we cannot turn the bytes back into [`Value::Map`]
    pub fn deser(bytes: &[u8]) -> Result<Self, StoreSerError> {
        let mut bytes = Cursor::new(&bytes);
        {
            let Some(magic_bytes) = bytes.read_exact() else {
                return Err(StoreSerError::NotEnoughBytes);
            };
            if magic_bytes != b"SOURISDB" {
                return Err(StoreSerError::ExpectedMagicBytes);
            }
        }
        let Some(compression) = bytes.next().copied() else {
            return Err(StoreSerError::NotEnoughBytes);
        };
        let is_huffman_encoded = (compression & 0b1000_0000) != 0;
        let compression_ty = BinaryCompression::try_from(compression & 0b0111_1111)?;

        let bytes = BinaryData::deser(compression_ty, &mut bytes)?.0;
        let mut bytes = Cursor::new(&bytes);

        let huffman = if is_huffman_encoded {
            Some(Huffman::<char>::deser(&mut bytes)?)
        } else {
            None
        };

        let val = Value::deser(&mut Cursor::new(&bytes), huffman.as_ref())?;
        let ty = val.as_ty();
        let Some(map) = val.to_map() else {
            return Err(StoreSerError::ExpectedMap(ty));
        };
        Ok(Self(map))
    }

    ///Gets a store back from bytes that represent JSON.
    ///
    /// # Errors
    ///
    /// - [`serde_json::Error`] if we cannot parse the JSON.
    pub fn from_json_bytes(json: &[u8]) -> Result<Self, StoreSerError> {
        let val = serde_json::from_slice(json)?;
        Ok(Self::from_json(val))
    }

    #[cfg(feature = "serde")]
    pub fn from_bytes<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, StoreSerError> {
        let s = Self::deser(bytes)?;
        let v = s
            .to_json(false)
            .ok_or(StoreSerError::UnableToConvertToJson)?;
        Ok(serde_json::from_value(v)?)
    }

    #[cfg(feature = "serde")]
    pub fn to_bytes(t: &impl serde::Serialize) -> Result<Vec<u8>, StoreSerError> {
        let v = serde_json::to_value(t)?;
        let s = Self::from_json(v);
        s.ser()
    }

    ///fails if integer out of range, or float is NaN or infinite
    #[must_use]
    pub fn to_json(mut self, add_souris_types: bool) -> Option<SJValue> {
        if self.len() == 1 {
            if let Some(v) = self.0.remove("JSON") {
                return v.convert_to_json(add_souris_types);
            }
        }

        Some(SJValue::Object(
            self.0
                .into_iter()
                .map(|(k, v)| v.convert_to_json(add_souris_types).map(|v| (k, v)))
                .collect::<Option<_>>()?,
        ))
    }

    #[must_use]
    pub fn from_json(val: SJValue) -> Self {
        Self(match Value::convert_from_json(val) {
            Value::Map(m) => m,
            v => {
                let mut map = HashMap::new();
                map.insert("JSON".into(), v);
                map
            }
        })
    }
}

impl TryFrom<Value> for Store {
    type Error = StoreSerError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        let ty = value.as_ty();
        let Some(db) = value.to_map() else {
            return Err(StoreSerError::ExpectedMap(ty));
        };
        Ok(Self(db))
    }
}

impl Deref for Store {
    type Target = HashMap<String, Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Store {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Display for Store {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", Value::Map(self.0.clone()))
    }
}

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum StoreSerError {
    ExpectedMap(ValueTy),
    ExpectedMagicBytes,
    NotEnoughBytes,
    Value(ValueSerError),
    Integer(IntegerSerError),
    SerdeJson(SJError),
    UnableToConvertToJson,
    UnsupportedCompression(u8),
    Huffman(HuffmanSerError),
    Binary(BinarySerError),
}

impl Display for StoreSerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            StoreSerError::ExpectedMap(t) => write!(
                f,
                "Expected to find a map when deserialising, found {t:?} instead"
            ),
            StoreSerError::NotEnoughBytes => write!(f, "Not enough bytes"),
            StoreSerError::ExpectedMagicBytes => write!(f, "Unable to find starting magic bytes"),
            StoreSerError::Integer(i) => write!(f, "Error with integer: {i}"),
            StoreSerError::Value(e) => write!(f, "Error with values: {e}"),
            StoreSerError::SerdeJson(e) => write!(f, "Error with serde_json: {e}"),
            StoreSerError::UnableToConvertToJson => write!(f, "Unable to convert self to JSON"),
            StoreSerError::UnsupportedCompression(b) => {
                write!(f, "Unable to read compression type: {b:#b}")
            }
            StoreSerError::Huffman(h) => write!(f, "Error with huffman: {h}"),
            StoreSerError::Binary(b) => write!(f, "Error with binary compression: {b}"),
        }
    }
}

impl From<ValueSerError> for StoreSerError {
    fn from(value: ValueSerError) -> Self {
        Self::Value(value)
    }
}
impl From<SJError> for StoreSerError {
    fn from(value: SJError) -> Self {
        Self::SerdeJson(value)
    }
}
impl From<IntegerSerError> for StoreSerError {
    fn from(value: IntegerSerError) -> Self {
        Self::Integer(value)
    }
}
impl From<HuffmanSerError> for StoreSerError {
    fn from(value: HuffmanSerError) -> Self {
        Self::Huffman(value)
    }
}
impl From<BinarySerError> for StoreSerError {
    fn from(value: BinarySerError) -> Self {
        Self::Binary(value)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for StoreSerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Integer(i) => Some(i),
            Self::Value(e) => Some(e),
            Self::SerdeJson(e) => Some(e),
            Self::Huffman(h) => Some(h),
            _ => None,
        }
    }
}
