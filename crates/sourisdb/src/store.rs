use crate::{
    utilities::cursor::Cursor,
    values::{Value, ValueSerError},
};
use alloc::{vec, vec::Vec};
use core::{
    fmt::{Display, Formatter},
    ops::{Deref, DerefMut},
};
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
    #[must_use]
    pub fn new(v: Value) -> Self {
        Self(v)
    }

    #[must_use]
    pub fn new_map() -> Self {
        Self(Value::Map(HashMap::new()))
    }

    #[must_use]
    pub fn new_array() -> Self {
        Self(Value::Array(Vec::new()))
    }

    pub fn ser(&self) -> Result<Vec<u8>, ValueSerError> {
        let mut res = vec![];

        res.extend(b"SOURISDB");
        res.extend(self.0.ser()?);

        Ok(res)
    }

    pub fn deser(bytes: &[u8]) -> Result<Self, ValueSerError> {
        let mut bytes = Cursor::new(&bytes);
        let _ = bytes.read_specific::<8>();

        let val = Value::deser(&mut bytes)?;
        Ok(Self(val))
    }

    pub fn from_json(json: &[u8]) -> Result<Self, ValueSerError> {
        let val: SJValue = serde_json::from_slice(json)?;
        Ok(Self(Value::from(val)))
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

impl Display for Store {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}
