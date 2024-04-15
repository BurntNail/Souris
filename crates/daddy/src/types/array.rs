use crate::{
    types::integer::{Integer, IntegerSerError},
    utilities::cursor::Cursor,
    values::{Value, ValueSerError},
    version::Version,
};
use alloc::{
    boxed::Box,
    vec,
    vec::Vec,
};
use core::fmt::{Debug, Display, Formatter};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
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

impl Array {
    pub fn ser(&self, version: Version) -> Result<Vec<u8>, ArraySerError> {
        match version {
            Version::V0_1_0 => {
                let mut res: Vec<u8> = vec![];

                res.extend(Integer::usize(self.0.len()).ser(version).iter());

                for v in &self.0 {
                    let bytes = v.ser(version)?;
                    res.extend(Integer::usize(bytes.len()).ser(version).iter());
                    res.extend(bytes.iter());
                }

                Ok(res)
            }
        }
    }

    pub fn deser(bytes: &mut Cursor<u8>, version: Version) -> Result<Self, ArraySerError> {
        match version {
            Version::V0_1_0 => {
                let len: usize = Integer::deser(bytes, version)?.try_into()?;
                Ok(Self(
                    (0..len)
                        .map(|_| {
                            let len: usize = Integer::deser(bytes, version)?.try_into()?;
                            Value::deserialise(bytes, len, version)
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                ))
            }
        }
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
