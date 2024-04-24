use crate::{
    types::integer::{Integer, IntegerSerError},
    utilities::cursor::Cursor,
    values::{Value, ValueSerError},
};
use alloc::{boxed::Box, vec, vec::Vec};
use core::fmt::{Debug, Display, Formatter};

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Array(pub Vec<Value>);

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum ArraySerError {
    ValueSerError(Box<ValueSerError>),
    IntegerSerError(IntegerSerError),
}

impl From<ValueSerError> for ArraySerError {
    fn from(value: ValueSerError) -> Self {
        Self::ValueSerError(Box::new(value))
    }
}
impl From<IntegerSerError> for ArraySerError {
    fn from(value: IntegerSerError) -> Self {
        Self::IntegerSerError(value)
    }
}

impl Display for ArraySerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ValueSerError(e) => write!(f, "Error de/ser-ing value: {e:?}"),
            Self::IntegerSerError(e) => write!(f, "Error de/ser-ing integer: {e:?}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ArraySerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ArraySerError::ValueSerError(e) => Some(e),
            ArraySerError::IntegerSerError(e) => Some(e),
        }
    }
}

impl Array {
    pub fn ser(&self) -> Result<Vec<u8>, ArraySerError> {
        let mut res: Vec<u8> = vec![];

        res.extend(Integer::usize(self.0.len()).ser().iter());

        for v in &self.0 {
            let bytes = v.ser()?;
            res.extend(bytes.iter());
        }

        Ok(res)
    }

    pub fn deser(bytes: &mut Cursor<u8>) -> Result<Self, ArraySerError> {
        let len: usize = Integer::deser(bytes)?.try_into()?;
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            v.push(Value::deserialise(bytes)?);
        }
        Ok(Self(v)) //yes, could use FP and `map` etc, but this makes it easier to ensure no screwery with control flow
    }
}

impl Display for Array {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self.0.len() {
            0 => write!(f, "[]"),
            1 => write!(f, "[{}]", self.0[0]),
            _ => {
                write!(f, "[{}", self.0[0])?;
                for v in self.0.iter().skip(1) {
                    write!(f, ", {v}")?;
                }
                write!(f, "]")
            }
        }
    }
}
