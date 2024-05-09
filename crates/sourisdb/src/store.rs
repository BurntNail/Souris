use crate::{
    values::{Value},
};
use serde_json::{Value as SJValue};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Store(pub Value);

impl From<SJValue> for Store {
    fn from(value: SJValue) -> Self {
        Self(Value::from(value))
    }
}