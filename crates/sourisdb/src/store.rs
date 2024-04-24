use crate::{
    types::integer::{Integer, IntegerSerError},
    utilities::cursor::Cursor,
    values::{Value, ValueSerError},
    version::{Version, VersionSerError},
};
use alloc::{vec, vec::Vec};
use core::{
    fmt::{Display, Formatter},
    ops::{Index, IndexMut},
};
use hashbrown::hash_map::{HashMap, IntoIter, Keys, Values};

#[derive(Debug, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Store {
    version: Version,
    kvs: HashMap<Value, Value>,
}

//TODO: consider bit twiddling tricks for runtime RAM cost

impl Store {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn from_version_and_map(version: Version, kvs: HashMap<Value, Value>) -> Self {
        Self { version, kvs }
    }

    pub fn insert(&mut self, k: Value, v: Value) {
        self.kvs.insert(k, v);
    }

    pub fn remove(&mut self, k: &Value) -> Option<Value> {
        self.kvs.remove(k)
    }

    #[must_use]
    pub fn version(&self) -> Version {
        self.version
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.kvs.is_empty()
    }
    #[must_use]
    pub fn size(&self) -> usize {
        self.kvs.len()
    }

    #[must_use]
    pub fn keys(&self) -> Keys<'_, Value, Value> {
        self.kvs.keys()
    }

    #[must_use]
    pub fn values(&self) -> Values<'_, Value, Value> {
        self.kvs.values()
    }

    #[must_use]
    pub fn get(&self, k: &Value) -> Option<&Value> {
        self.kvs.get(k)
    }

    #[must_use]
    pub fn get_mut(&mut self, k: &Value) -> Option<&mut Value> {
        self.kvs.get_mut(k)
    }

    pub fn clear(&mut self) {
        self.kvs.clear();
    }
}

impl IntoIterator for Store {
    type Item = (Value, Value);
    type IntoIter = IntoIter<Value, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.kvs.into_iter()
    }
}

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum StoreError {
    ValueError(ValueSerError),
    IntegerError(IntegerSerError),
    VersionError(VersionSerError),
    CouldntFindKey,
}
impl Display for StoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ValueError(e) => write!(f, "Error de/ser-ing value: {e:?}"),
            Self::IntegerError(e) => write!(f, "Error de/ser-ing integer: {e:?}"),
            Self::VersionError(e) => write!(f, "Error de/ser-ing version: {e:?}"),
            Self::CouldntFindKey => write!(f, "Could not find key"),
        }
    }
}

impl From<ValueSerError> for StoreError {
    fn from(value: ValueSerError) -> Self {
        Self::ValueError(value)
    }
}
impl From<IntegerSerError> for StoreError {
    fn from(value: IntegerSerError) -> Self {
        Self::IntegerError(value)
    }
}
impl From<VersionSerError> for StoreError {
    fn from(value: VersionSerError) -> Self {
        Self::VersionError(value)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StoreError::ValueError(e) => Some(e),
            StoreError::IntegerError(e) => Some(e),
            StoreError::VersionError(e) => Some(e),
            StoreError::CouldntFindKey => None,
        }
    }
}

impl Default for Store {
    fn default() -> Self {
        Self {
            version: Version::V0_1_0,
            kvs: HashMap::new(),
        }
    }
}

impl Store {
    ///format:
    ///
    /// 10 bytes: title
    /// 1 byte: \0
    /// 6 bytes: version
    /// 1 byte: \0
    /// 4 bytes: size text
    /// 1 byte: \0
    /// 8 bytes: size
    /// 1 byte: \0
    ///
    /// keys:
    ///     8 bytes: `key_length`
    ///     8 bytes: `value_length`
    ///     `key_length` bytes: content
    ///
    /// values:
    ///     see value serialisations lol
    ///     NB: same order as keys
    pub fn ser(&self) -> Result<Vec<u8>, StoreError> {
        let mut res = vec![];
        res.extend(b"DADDYSTORE".iter());
        res.push(0);
        res.extend(self.version.to_bytes().iter());
        res.push(0);

        match self.version {
            Version::V0_1_0 => {
                let length = self.kvs.len();
                res.extend(b"SIZE".iter());
                res.push(0);
                res.extend(Integer::usize(length).ser(self.version));
                res.push(0);

                for (k, v) in &self.kvs {
                    let ser_key = k.ser(self.version)?;
                    let ser_value = v.ser(self.version)?;

                    res.extend(ser_key.iter());
                    res.extend(ser_value.iter());
                }

                Ok(res)
            }
        }
    }

    pub fn deser(bytes: &mut Cursor<u8>) -> Result<Self, StoreError> {
        bytes.seek(10); //title
        bytes.seek(1); //\0
        let version = Version::from_bytes(bytes)?;
        bytes.seek(1); //\0

        match version {
            Version::V0_1_0 => {
                bytes.seek(4); //size
                bytes.seek(1); //\0
                let length: usize = Integer::deser(bytes, version)?.try_into()?;
                bytes.seek(1); //\0

                let mut kvs = HashMap::new();
                for _ in 0..length {
                    let key = Value::deserialise(bytes, version)?;
                    let value = Value::deserialise(bytes, version)?;
                    kvs.insert(key, value);
                }

                Ok(Self { version, kvs })
            }
        }
    }
}

impl Index<Value> for Store {
    type Output = Value;

    fn index(&self, index: Value) -> &Self::Output {
        &self.kvs[&index]
    }
}
impl IndexMut<Value> for Store {
    fn index_mut(&mut self, index: Value) -> &mut Self::Output {
        self.kvs
            .get_mut(&index)
            .unwrap_or_else(|| panic!("key not found"))
    }
}
