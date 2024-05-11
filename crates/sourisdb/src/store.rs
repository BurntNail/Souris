use crate::{
    utilities::cursor::Cursor,
    values::{Value, ValueSerError, ValueTy},
};
use alloc::{vec, vec::Vec, string::String};
use core::{
    fmt::{Display, Formatter},
    ops::{Deref, DerefMut},
};
use hashbrown::HashMap;
use serde_json::{Error as SJError, Value as SJValue};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Store(HashMap<String, Value>);

impl Store {
    #[must_use]
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn ser(&self) -> Result<Vec<u8>, StoreSerError> {
        let mut res = vec![];

        res.extend(b"SOURISDB");
        res.extend(Value::Map(self.0.clone()).ser()?);

        Ok(res)
    }

    pub fn deser(bytes: &[u8]) -> Result<Self, StoreSerError> {
        let mut bytes = Cursor::new(&bytes);
        let _ = bytes.read_specific::<8>();

        let val = Value::deser(&mut bytes)?;
        let ty = val.as_ty();
        let Some(map) = val.to_map() else {
            return Err(StoreSerError::ExpectedMap(ty));
        };
        Ok(Self(map))
    }

    pub fn from_json(json: &[u8]) -> Result<Self, StoreSerError> {
        let val: SJValue = serde_json::from_slice(json)?;
        let map = match Value::from(val) {
            Value::Map(m) => m,
            v => {
                let mut map = HashMap::new();
                map.insert("JSON".into(), v);
                map
            }
        };

        Ok(Self(map))
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
pub enum StoreSerError {
    ExpectedMap(ValueTy),
    Value(ValueSerError),
    SerdeJson(SJError),
}

impl Display for StoreSerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            StoreSerError::ExpectedMap(t) => write!(
                f,
                "Expected to find a map when deserialising, found {t:?} instead"
            ),
            StoreSerError::Value(e) => write!(f, "Error with values: {e}"),
            StoreSerError::SerdeJson(e) => write!(f, "Error with serde_json: {e}"),
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

#[cfg(feature = "std")]
impl std::error::Error for StoreSerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Value(e) => Some(e),
            Self::SerdeJson(e) => Some(e),
            _ => None,
        }
    }
}
