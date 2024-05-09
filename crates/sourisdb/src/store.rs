use hashbrown::HashMap;
use crate::{
    values::{Value},
};
use alloc::vec::Vec;
use serde_json::{Value as SJValue};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Store(pub Value);

impl From<SJValue> for Store {
    fn from(value: SJValue) -> Self {
        Self(Value::from(value))
    }
}

impl Store {
    pub fn new (v: Value) -> Self {
        Self(v)
    }

    pub fn new_map () -> Self {
        Self(Value::Map(HashMap::new()))
    }

    pub fn new_array () -> Self {
        Self(Value::Array(Vec::new()))
    }

    pub fn as_char (&self) -> Option<char> {
        if let Value::Ch(c) = self.0 {
            Some(c)
        } else {
            None
        }
    }
}

//TODO: ser + deser methods + convenience to_ methods