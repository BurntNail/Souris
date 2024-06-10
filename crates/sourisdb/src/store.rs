//! Provides the main key-value store designed to be used for communications.

use alloc::{string::String, vec, vec::Vec};
use core::{
    fmt::{Display, Formatter},
    ops::{Deref, DerefMut},
};

use hashbrown::HashMap;
use lz4_flex::{block::DecompressError as Lz4DecompressError, compress, decompress};
use miniz_oxide::{
    deflate::compress_to_vec,
    inflate::{decompress_to_vec, DecompressError as MinizDecompressError},
};
use serde_json::{Error as SJError, Value as SJValue};

use crate::{
    types::integer::{Integer, IntegerSerError, SignedState},
    utilities::cursor::Cursor,
    values::{Value, ValueSerError, ValueTy},
};

///A key-value store where the keys are [`String`]s and the values are [`Value`]s - this is a thin wrapper around [`hashbrown::HashMap`] and implements both [`Deref`] and [`DerefMut`] pointing to it. This database is optimised for storage when serialised.
///
/// The expectation is that if you need an in-memory key-value database, you do one of two things:
/// - Spin up a server running `sourisd` and make HTTP requests to it. Then, serialise or deserialise the values appropriately.
/// - Create a `Store` and keep it in the state of your program. To access values just use it as a [`hashbrown::HashMap`]. When your program exits (or periodically to allow for if the program quits unexpectedly), serialise the database and write it to a file. Then, when starting the program again read the database in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Store(HashMap<String, Value>);

enum CompressionType {
    None,
    Lz4,
    Miniz,
}
impl From<CompressionType> for u8 {
    fn from(value: CompressionType) -> Self {
        match value {
            CompressionType::None => 0,
            CompressionType::Lz4 => 1,
            CompressionType::Miniz => 2,
        }
    }
}
impl TryFrom<u8> for CompressionType {
    type Error = StoreSerError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::None,
            1 => Self::Lz4,
            2 => Self::Miniz,
            _ => return Err(StoreSerError::UnsupportedCompression(value)),
        })
    }
}

impl Store {
    fn compress(bytes: &[u8]) -> (Option<Vec<u8>>, CompressionType) {
        let raw = bytes;

        let mut lz4 = Integer::from(raw.len()).ser().1;
        lz4.extend(compress(bytes));

        let miniz = compress_to_vec(bytes, 10);

        if [miniz.len(), lz4.len()]
            .iter()
            .into_iter()
            .all(|x| *x >= raw.len())
        {
            (None, CompressionType::None)
        } else if miniz.len() < lz4.len() {
            (Some(miniz), CompressionType::Miniz)
        } else {
            (Some(lz4), CompressionType::Lz4)
        }
    }
    fn decompress(
        bytes: &[u8],
        compression_type: CompressionType,
    ) -> Result<Vec<u8>, StoreSerError> {
        match compression_type {
            CompressionType::None => Ok(bytes.to_vec()),
            CompressionType::Lz4 => {
                let mut cursor = Cursor::new(&bytes);
                let original_len: usize =
                    Integer::deser(SignedState::Positive, &mut cursor)?.try_into()?;

                Ok(decompress(cursor.as_ref(), original_len)?)
            }
            CompressionType::Miniz => Ok(decompress_to_vec(bytes)?),
        }
    }

    ///Serialises a store into bytes. There are 8 magic bytes at the front which read `SOURISDB` and the rest is serialised as a [`Value::Map`] containing the map stored within the caller.
    pub fn ser(&self) -> Result<Vec<u8>, StoreSerError> {
        let raw_map = Value::Map(self.0.clone()).ser()?;
        let (map, compression_ty) = Self::compress(&raw_map);

        let mut res = vec![];

        res.extend(b"SOURISDB");
        res.push(u8::from(compression_ty));
        if let Some(map) = map {
            res.extend(map);
        } else {
            res.extend(raw_map);
        }

        Ok(res)
    }

    pub fn deser(bytes: &[u8]) -> Result<Self, StoreSerError> {
        let mut bytes = Cursor::new(&bytes);
        {
            let Some(magic_bytes) = bytes.read_specific() else {
                return Err(StoreSerError::NotEnoughBytes);
            };
            if magic_bytes != b"SOURISDB" {
                return Err(StoreSerError::ExpectedMagicBytes);
            }
        }
        let Some(compression_ty) = bytes.next().copied() else {
            return Err(StoreSerError::NotEnoughBytes);
        };
        let compression_ty = CompressionType::try_from(compression_ty)?;

        let uncompressed_bytes = Self::decompress(bytes.as_ref(), compression_ty)?;
        drop(bytes);

        let val = Value::deser(&mut Cursor::new(&uncompressed_bytes))?;
        let ty = val.as_ty();
        let Some(map) = val.to_map() else {
            return Err(StoreSerError::ExpectedMap(ty));
        };
        Ok(Self(map))
    }

    pub fn from_json_bytes(json: &[u8]) -> Result<Self, StoreSerError> {
        let val = serde_json::from_slice(json)?;
        Ok(Self::from_json(val))
    }

    #[cfg(feature = "serde")]
    pub fn from_bytes<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, StoreSerError> {
        let s = Self::deser(bytes)?;
        let v = s.to_json().ok_or(StoreSerError::UnableToConvertToJson)?;
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
    pub fn to_json(mut self) -> Option<SJValue> {
        if self.len() == 1 {
            if let Some(v) = self.0.remove("JSON") {
                return v.convert_to_json();
            }
        }

        Some(SJValue::Object(
            self.0
                .into_iter()
                .map(|(k, v)| v.convert_to_json().map(|v| (k, v)))
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
    Lz4Decompress(Lz4DecompressError),
    MinizDecompresss(MinizDecompressError),
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
            StoreSerError::Lz4Decompress(d) => write!(f, "Error with Lz4 decompression: {d}"),
            StoreSerError::MinizDecompresss(d) => write!(f, "Error with miniz decompression: {d}"),
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
impl From<Lz4DecompressError> for StoreSerError {
    fn from(value: Lz4DecompressError) -> Self {
        Self::Lz4Decompress(value)
    }
}
impl From<MinizDecompressError> for StoreSerError {
    fn from(value: MinizDecompressError) -> Self {
        Self::MinizDecompresss(value)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for StoreSerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Integer(i) => Some(i),
            Self::Value(e) => Some(e),
            Self::SerdeJson(e) => Some(e),
            Self::Lz4Decompress(d) => Some(d),
            Self::MinizDecompresss(d) => Some(d),
            _ => None,
        }
    }
}
