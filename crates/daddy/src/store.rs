use core::panic;
use std::{
    collections::{HashMap, VecDeque},
    io::Error as IOError,
    ops::{Index, IndexMut},
};

use crate::{
    values::{Value, ValueFailure},
    version::Version,
};

#[derive(Debug)]
pub struct Store {
    version: Version,
    kvs: HashMap<Value, Value>,
}

#[derive(Debug)]
pub enum StoreFailure {
    ValueError(ValueFailure),
    IO(IOError),
    CouldntFindKey,
}

impl From<ValueFailure> for StoreFailure {
    fn from(value: ValueFailure) -> Self {
        Self::ValueError(value)
    }
}
impl From<IOError> for StoreFailure {
    fn from(value: IOError) -> Self {
        Self::IO(value)
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
    ///     8 bytes: content_length
    ///     8 bytes: lookup
    ///     length bytes: content
    ///
    /// values:
    ///     see value serialisations lol
    ///     NB: same order as keys
    ///     NB: lookups absolute to the whole file
    pub fn ser(self) -> Result<Vec<u8>, StoreFailure> {
        let mut res = vec![];
        res.extend(b"DADDYSTORE".iter());
        res.push(0);
        res.extend(self.version.to_bytes().iter());
        res.push(0);

        let length = self.kvs.len();
        res.extend(b"SIZE".iter());
        res.push(0);
        res.extend(length.to_le_bytes().iter());
        res.push(0);

        let (keys, values) = {
            let mut start = 0;

            let mut keys: Vec<(Vec<u8>, usize, usize)> = vec![];
            let mut values: Vec<u8> = vec![];

            for (k, v) in self.kvs.into_iter() {
                let ser_key = k.serialise()?;
                let ser_value = v.serialise()?;

                keys.push((ser_key, ser_value.len(), start));
                start += ser_value.len();

                values.extend(ser_value.iter());
            }

            (keys, values)
        };

        let mut ser_keys: Vec<(usize, usize, usize, Vec<u8>)> = vec![];
        for (ser_key, value_length, value_lookup) in keys {
            let key_length = ser_key.len();

            ser_keys.push((key_length, value_length, value_lookup, ser_key));
        }

        let lookup_modifier = res.len()
            + ser_keys
                .iter()
                .map(|(_, _, _, v)| 24 + v.len())
                .sum::<usize>();
        for (key_length, value_length, value_lookup, key_ser) in ser_keys {
            res.extend(key_length.to_le_bytes().iter());
            res.extend(value_length.to_le_bytes().iter());
            res.extend((value_lookup + lookup_modifier).to_le_bytes().iter());
            res.extend(key_ser);
        }

        res.extend(values);

        Ok(res)
    }

    pub fn deser(bytes: Vec<u8>) -> Result<Self, StoreFailure> {
        let mut bytes = VecDeque::from(bytes);

        bytes.skip_front(10); //title
        bytes.skip_front(1); //\0

        let version = Version::from_bytes(&bytes.take_to_vec(6).unwrap()).unwrap();

        match version {
            Version::V0_1_0 => {
                bytes.skip_front(1); //\0
                bytes.skip_front(4); //size
                bytes.skip_front(1); //\0

                let length: [u8; 8] = bytes.take_to_vec(8).unwrap().try_into().unwrap();
                let length = usize::from_le_bytes(length);

                bytes.skip_front(1); //\0

                struct Val {
                    value_length: usize,
                    key: Value,
                }

                let mut keys = Vec::with_capacity(length);
                for _ in 0..length {
                    let key_length: [u8; 8] = bytes.take_to_vec(8).unwrap().try_into().unwrap();
                    let key_length = usize::from_le_bytes(key_length);

                    let value_length: [u8; 8] = bytes.take_to_vec(8).unwrap().try_into().unwrap();
                    let value_length = usize::from_le_bytes(value_length);

                    bytes.skip_front(8); //value lookup, but not used if we're getting the whole thing

                    let key = bytes.take_to_vec(key_length).unwrap();
                    let key = Value::deserialise(&key)?;
                    keys.push(Val { value_length, key });
                }

                let mut kvs = HashMap::new();
                for Val { value_length, key } in keys {
                    let value: Vec<u8> = bytes.take_to_vec(value_length).unwrap();
                    let value = Value::deserialise(&value)?;

                    kvs.insert(key, value);
                }

                Ok(Self { version, kvs })
            }
        }
    }

    pub fn deser_specific(bytes: &[u8], key: Value) -> Result<Value, StoreFailure> {
        match Version::from_bytes(&bytes[11..17]).unwrap() {
            Version::V0_1_0 => {
                let length: [u8; 8] = bytes[23..31].try_into().unwrap();
                let length = usize::from_le_bytes(length);

                let mut lookup_length: Option<(usize, usize)> = None;
                let mut start: usize = 32;

                let serialised_key = key.serialise()?;

                for _ in 0..length {
                    let key_length: [u8; 8] = bytes[start..(start + 8)].try_into().unwrap();
                    let key_length: usize = usize::from_le_bytes(key_length);
                    start += 8;

                    let value_length: [u8; 8] = bytes[start..(start + 8)].try_into().unwrap();
                    let value_length: usize = usize::from_le_bytes(value_length);
                    start += 8;

                    let lookup: [u8; 8] = bytes[start..(start + 8)].try_into().unwrap();
                    let lookup: usize = usize::from_le_bytes(lookup);
                    start += 8;

                    let found_key = &bytes[start..(start + key_length)];
                    start += key_length;
                    if key_length != serialised_key.len() {
                        continue;
                    }

                    if serialised_key == found_key {
                        lookup_length = Some((lookup, value_length));
                        break;
                    }
                }

                let Some((lookup, value_length)) = lookup_length else {
                    return Err(StoreFailure::CouldntFindKey);
                };

                Ok(Value::deserialise(&bytes[lookup..(lookup + value_length)])?)
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
