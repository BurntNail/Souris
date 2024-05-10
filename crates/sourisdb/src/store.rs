use crate::{
    utilities::cursor::Cursor,
    values::{Value, ValueSerError},
};
use alloc::{vec, vec::Vec};
use core::ops::{Deref, DerefMut};
use hashbrown::HashMap;
use serde_json::Value as SJValue;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Store(pub Value);

impl From<SJValue> for Store {
    fn from(value: SJValue) -> Self {
        Self(Value::from(value))
    }
}

impl Store {
    pub fn new(v: Value) -> Self {
        Self(v)
    }

    pub fn new_map() -> Self {
        Self(Value::Map(HashMap::new()))
    }

    pub fn new_array() -> Self {
        Self(Value::Array(Vec::new()))
    }

    pub fn ser(&self) -> Result<Vec<u8>, ValueSerError> {
        let mut res = vec![];

        res.extend(b"SOURISDB");
        res.extend(self.0.ser()?);

        Ok(res)
    }

    pub fn deser(bytes: Vec<u8>) -> Result<Self, ValueSerError> {
        let mut bytes = Cursor::new(&bytes);
        let val = Value::deser(&mut bytes)?;
        Ok(Self(val))
    }
}

impl Deref for Store {
    type Target = Value;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for Store {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

//TODO: ser + deser methods + convenience to_ methods
