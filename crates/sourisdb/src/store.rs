use crate::{
    types::integer::{Integer, IntegerSerError},
    utilities::cursor::Cursor,
    values::{Value, ValueSerError},
};
use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::{
    fmt::{Display, Formatter},
    ops::{Index, IndexMut},
};
use hashbrown::hash_map::{HashMap, IntoIter};
use serde_json::{Error as SJError, Map, Value as SJValue};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Store {
    Map { kvs: HashMap<Value, Value> },
    Array { arr: Vec<Value> },
}

pub enum Version {
    Map,
    Array,
}

impl<'a> From<&'a Store> for Version {
    fn from(value: &'a Store) -> Self {
        match value {
            Store::Map { .. } => Self::Map,
            Store::Array { .. } => Self::Array,
        }
    }
}

impl From<Version> for u8 {
    fn from(val: Version) -> u8 {
        match val {
            Version::Map => 0b0001,
            Version::Array => 0b0010,
        }
    }
}
impl TryFrom<u8> for Version {
    type Error = StoreError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0b0001 => Version::Map,
            0b0010 => Version::Array,
            _ => return Err(StoreError::InvalidVersion(value)),
        })
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::Map {
            kvs: HashMap::default(),
        }
    }
}

impl Display for Store {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Map { kvs } => {
                writeln!(f, "{{")?;

                for (k, v) in kvs {
                    writeln!(f, "\t{k}: {v},")?;
                }

                writeln!(f, "}}")
            }
            Self::Array { arr } => {
                writeln!(f, "[")?;

                for i in arr {
                    writeln!(f, "\t{i},")?;
                }

                writeln!(f, "]")
            }
        }
    }
}

impl Store {
    #[must_use]
    pub fn new_map(kvs: HashMap<Value, Value>) -> Self {
        Self::Map { kvs }
    }

    #[must_use]
    pub fn new_arr(arr: Vec<Value>) -> Self {
        Self::Array { arr }
    }

    ///map: inserts normally
    ///
    ///arr: assumes k can be a usize, inserts at relevant index. else adds to end
    pub fn insert(&mut self, k: Value, v: Value) {
        match self {
            Self::Map { kvs } => {
                kvs.insert(k, v);
            }
            Self::Array { arr } => {
                if let Value::Int(i) = k {
                    if let Ok(u) = usize::try_from(i) {
                        let current_len = arr.len();

                        if u < current_len {
                            arr[u] = v;
                        } else if u == current_len {
                            arr.push(v);
                        } else {
                            arr.extend(vec![Value::Null; current_len - u]);
                            arr.push(v);
                        }
                    }
                } else {
                    arr.push(v);
                }
            }
        }
    }

    ///map: noop //TODO: what should this do?
    ///arr: obvs
    pub fn push(&mut self, v: Value) {
        match self {
            Self::Array { arr } => {
                arr.push(v);
            }
            Self::Map { .. } => unimplemented!("push should be a noop if not an array"),
        }
    }

    pub fn remove(&mut self, k: &Value) -> Option<Value> {
        match self {
            Self::Map { kvs } => kvs.remove(k),
            Self::Array { arr } => {
                if let Value::Int(i) = k {
                    if let Ok(u) = usize::try_from(*i) {
                        if u < arr.len() {
                            return Some(arr.remove(u));
                        }
                    }
                }
                None
            }
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Map { kvs } => kvs.is_empty(),
            Self::Array { arr } => arr.is_empty(),
        }
    }
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Map { kvs } => kvs.len(),
            Self::Array { arr } => arr.len(),
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = Value> {
        match self {
            Self::Map { kvs } => kvs.keys().cloned().collect::<Vec<_>>().into_iter(),
            Self::Array { arr } => (0..arr.len())
                .map(|x| Value::Int(x.into()))
                .collect::<Vec<_>>()
                .into_iter(),
        }
    }

    pub fn values(&self) -> impl Iterator<Item = Value> {
        match self {
            Self::Map { kvs } => kvs.keys().cloned().collect::<Vec<_>>().into_iter(),
            Self::Array { arr } => arr.clone().into_iter(),
        }
    }

    #[must_use]
    pub fn get(&self, k: &Value) -> Option<&Value> {
        match self {
            Self::Map { kvs } => kvs.get(k),
            Self::Array { arr } => {
                if let Value::Int(i) = k {
                    if let Ok(u) = usize::try_from(*i) {
                        return arr.get(u);
                    }
                }

                None
            }
        }
    }

    #[must_use]
    pub fn get_mut(&mut self, k: &Value) -> Option<&mut Value> {
        match self {
            Self::Map { kvs } => kvs.get_mut(k),
            Self::Array { arr } => {
                if let Value::Int(i) = k {
                    if let Ok(u) = usize::try_from(*i) {
                        return arr.get_mut(u);
                    }
                }

                None
            }
        }
    }

    pub fn clear(&mut self) {
        match self {
            Self::Map { kvs } => {
                kvs.clear();
            }
            Self::Array { arr } => {
                arr.clear();
            }
        }
    }
}

impl IntoIterator for Store {
    type Item = (Value, Value);
    type IntoIter = IntoIter<Value, Value>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            Store::Map { kvs } => kvs.into_iter(),
            Store::Array { arr } => (0..arr.len())
                .map(|x| Value::Int(x.into()))
                .zip(arr)
                .collect::<HashMap<_, _>>()
                .into_iter(),
        }
    }
}

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum StoreError {
    ValueError(ValueSerError),
    IntegerError(IntegerSerError),
    CouldntFindKey,
    SerdeJson(SJError),
    InvalidVersion(u8),
    NotEnoughBytes,
}
impl Display for StoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ValueError(e) => write!(f, "Error de/ser-ing value: {e:?}"),
            Self::IntegerError(e) => write!(f, "Error de/ser-ing integer: {e:?}"),
            Self::InvalidVersion(e) => write!(f, "Error de/ser-ing version: {e:#b}"),
            Self::CouldntFindKey => write!(f, "Could not find key"),
            Self::SerdeJson(e) => write!(f, "Error de/ser-ing JSON: {e:?}"),
            Self::NotEnoughBytes => write!(f, "Not enough bytes"),
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
impl From<SJError> for StoreError {
    fn from(value: SJError) -> Self {
        Self::SerdeJson(value)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StoreError::ValueError(e) => Some(e),
            StoreError::IntegerError(e) => Some(e),
            StoreError::SerdeJson(e) => Some(e),
            _ => None,
        }
    }
}

impl From<SJValue> for Store {
    fn from(value: SJValue) -> Self {
        match value {
            SJValue::Array(v) => {
                let arr = v.into_iter().map(Value::from).collect();
                Self::Array { arr }
            }
            SJValue::Object(o) => Self::from(o),
            _ => {
                let item = Value::from(value);
                let key = Value::String(String::from("JSON Contents"));

                let mut map = HashMap::new();
                map.insert(key, item);
                Self::Map { kvs: map }
            }
        }
    }
}
impl From<Map<String, SJValue>> for Store {
    fn from(o: Map<String, SJValue>) -> Self {
        let mut map = HashMap::new();
        for (k, v) in o {
            let key = Value::String(k.to_string());
            let val = Value::from(v);
            map.insert(key, val);
        }
        Self::Map { kvs: map }
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
        res.extend(b"SourisDB".iter());
        res.push(0);
        let version: u8 = Version::from(self).into();
        res.push(version);
        res.push(0);

        match self {
            Store::Map { kvs } => {
                let length = kvs.len();
                res.extend(b"SIZE".iter());
                res.push(0);
                res.extend(Integer::usize(length).ser());
                res.push(0);

                for (k, v) in kvs {
                    let ser_key = k.ser()?;
                    let ser_value = v.ser()?;

                    res.extend(ser_key.iter());
                    res.extend(ser_value.iter());
                }
            }
            Store::Array { arr } => {
                res.extend(Integer::usize(arr.len()).ser());

                for v in arr {
                    res.extend(v.ser()?);
                }
            }
        }

        Ok(res)
    }

    pub fn deser(bytes: &mut Cursor<u8>) -> Result<Self, StoreError> {
        bytes.seek(8); //title
        bytes.seek(1); //\0
        let version = Version::try_from(bytes.next().copied().ok_or(StoreError::NotEnoughBytes)?)?;
        bytes.seek(1); //\0

        match version {
            Version::Map => {
                bytes.seek(4); //size
                bytes.seek(1); //\0
                let length: usize = Integer::deser(bytes)?.try_into()?;
                bytes.seek(1); //\0

                let mut kvs = HashMap::new();
                for _ in 0..length {
                    let key = Value::deserialise(bytes)?;
                    let value = Value::deserialise(bytes)?;
                    kvs.insert(key, value);
                }

                Ok(Self::Map { kvs })
            }
            Version::Array => {
                let len: usize = Integer::deser(bytes)?.try_into()?;

                let mut arr = Vec::with_capacity(len);
                for _ in 0..len {
                    arr.push(Value::deserialise(bytes)?);
                }

                Ok(Self::Array { arr })
            }
        }
    }

    pub fn from_json(bytes: &[u8]) -> Result<Self, StoreError> {
        let sjv: SJValue = serde_json::from_slice(bytes)?;
        Ok(Self::from(sjv))
    }
}

impl Index<Value> for Store {
    type Output = Value;

    fn index(&self, index: Value) -> &Self::Output {
        match self.get(&index) {
            Some(s) => s,
            None => panic!("unable to find key {index:?}"),
        }
    }
}
impl IndexMut<Value> for Store {
    fn index_mut(&mut self, index: Value) -> &mut Self::Output {
        match self.get_mut(&index) {
            Some(s) => s,
            None => panic!("unable to find key {index:?}"),
        }
    }
}
