use core::panic;
use std::{
    collections::{HashMap, VecDeque},
    io::Error as IOError,
    ops::{Index, IndexMut},
};
use std::io::{Cursor, Seek, SeekFrom};

use crate::{
    values::{Value, ValueSerError},
    version::Version,
};
use crate::niches::integer::{Integer, IntegerSerError};
use crate::version::VersionSerError;

#[derive(Debug)]
pub struct Store {
    version: Version,
    kvs: HashMap<Value, Value>,
}

#[derive(Debug)]
pub enum StoreFailure {
    ValueError(ValueSerError),
    IntegerError(IntegerSerError),
    VersionError(VersionSerError),
    IO(IOError),
    CouldntFindKey,
}

impl From<ValueSerError> for StoreFailure {
    fn from(value: ValueSerError) -> Self {
        Self::ValueError(value)
    }
}
impl From<IntegerSerError> for StoreFailure {
    fn from(value: IntegerSerError) -> Self {
        Self::IntegerError(value)
    }
}
impl From<IOError> for StoreFailure {
    fn from(value: IOError) -> Self {
        Self::IO(value)
    }
}
impl From<VersionSerError> for StoreFailure {
    fn from(value: VersionSerError) -> Self {
        Self::VersionError(value)
    }
}

trait TreatAVecLikeAnIterator<T> {
    fn skip_front(&mut self, n: usize);
    fn take_to_vec(&mut self, n: usize) -> Option<Vec<T>>;
}

impl<T> TreatAVecLikeAnIterator<T> for VecDeque<T> {
    fn skip_front(&mut self, n: usize) {
        for _ in 0..n {
            self.remove(0);
        }
    }

    fn take_to_vec(&mut self, n: usize) -> Option<Vec<T>> {
        (0..n)
            .map(|_| self.pop_front())
            .collect::<Option<Vec<T>>>()
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, k: Value, v: Value) {
        self.kvs.insert(k, v);
    }

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
    ///     8 bytes: key_length
    ///     8 bytes: value_length
    ///     key_length bytes: content
    ///
    /// values:
    ///     see value serialisations lol
    ///     NB: same order as keys
    pub fn ser(self) -> Result<Vec<u8>, StoreFailure> {
        let mut res = vec![];
        res.extend(b"DADDYSTORE".iter());
        res.push(0);
        res.extend(self.version.to_bytes().iter());
        res.push(0);

        let length = self.kvs.len();
        res.extend(b"SIZE".iter());
        res.push(0);
        res.extend(Integer::usize(length).ser());
        res.push(0);

            let mut keys: Vec<u8> = vec![];
            let mut values: Vec<u8> = vec![];

            for (k, v) in self.kvs.into_iter() {
                let ser_key = k.serialise()?;
                let ser_value = v.serialise()?;

                keys.extend(Integer::usize(ser_key.len()).ser());
                keys.extend(Integer::usize(ser_value.len()).ser());
                keys.extend(ser_key.iter());

                values.extend(ser_value.iter());
            }

            res.extend(keys);
        res.extend(values);

        Ok(res)
    }

    pub fn deser(bytes: Vec<u8>) -> Result<Self, StoreFailure> {
        let mut bytes = Cursor::new(bytes.as_slice());

        bytes.seek(SeekFrom::Current(10))?; //title
        bytes.seek(SeekFrom::Current(1))?; //\0

        let version = Version::from_bytes(&mut bytes)?;

        match version {
            Version::V0_1_0 => {
                bytes.seek(SeekFrom::Current(1))?; //\0
                bytes.seek(SeekFrom::Current(4))?; //size
                bytes.seek(SeekFrom::Current(1))?; //\0

                let length: usize = Integer::deser(&mut bytes)?.try_into()?;

                bytes.seek(SeekFrom::Current(1))?; //\0

                struct Val {
                    value_length: usize,
                    key: Value,
                }

                let mut keys = vec![];
                for _ in 0..length {
                    let key_length: usize = Integer::deser(&mut bytes)?.try_into()?;
                    let value_length: usize = Integer::deser(&mut bytes)?.try_into()?;

                    let key = Value::deserialise(&mut bytes, key_length)?;
                    keys.push(Val { value_length, key });
                }

                let mut kvs = HashMap::new();
                for Val { value_length, key } in keys {
                    let value = Value::deserialise(&mut bytes, value_length)?;
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
