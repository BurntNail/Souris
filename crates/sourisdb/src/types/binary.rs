use crate::{
    display_bytes_as_hex_array,
    types::{
        binary::{
            lz::{lz, un_lz},
            rle::{rle, un_rle},
        },
        integer::{Integer, IntegerSerError, SignedState},
    },
    utilities::cursor::Cursor,
    values::ValueTy,
};
use alloc::vec::Vec;
use core::fmt::{Debug, Display, Formatter};
use serde_json::{Map as SJMap, Number, Value as SJValue};

pub mod lz;
pub mod rle;

#[derive(Debug, Copy, Clone)]
pub enum BinaryCompression {
    Nothing,
    RunLengthEncoding,
    LempelZiv, //LZ77, not LZSS from https://go-compression.github.io/algorithms/lz/
}

impl From<BinaryCompression> for u8 {
    fn from(compression: BinaryCompression) -> Self {
        match compression {
            BinaryCompression::Nothing => 0,
            BinaryCompression::RunLengthEncoding => 1,
            BinaryCompression::LempelZiv => 2,
        }
    }
}

impl TryFrom<u8> for BinaryCompression {
    type Error = BinarySerError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Nothing),
            1 => Ok(Self::RunLengthEncoding),
            2 => Ok(Self::LempelZiv),
            _ => Err(BinarySerError::NoCompressionTypeFound(value)),
        }
    }
}

#[derive(Debug)]
pub enum BinarySerError {
    NoCompressionTypeFound(u8),
    Integer(IntegerSerError),
    NotEnoughBytes,
}

impl Display for BinarySerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoCompressionTypeFound(v) => {
                write!(f, "Invalid compression discriminant found: {v}")
            }
            Self::Integer(i) => write!(f, "Error parsing integer: {i}"),
            Self::NotEnoughBytes => write!(f, "Not enough bytes to deserialize."),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BinarySerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NoCompressionTypeFound(_) | Self::NotEnoughBytes => None,
            Self::Integer(i) => Some(i),
        }
    }
}

impl From<IntegerSerError> for BinarySerError {
    fn from(value: IntegerSerError) -> Self {
        Self::Integer(value)
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BinaryData(pub Vec<u8>);

impl Debug for BinaryData {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BinaryData")
            .field("data", &display_bytes_as_hex_array(&self.0))
            .finish()
    }
}
impl Display for BinaryData {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", &display_bytes_as_hex_array(&self.0))
    }
}
impl BinaryData {
    #[must_use]
    pub fn to_json(self, add_souris_types: bool) -> SJValue {
        let mut obj = SJMap::new();
        if add_souris_types {
            obj.insert(
                "souris_type".into(),
                SJValue::Number(Number::from(u8::from(ValueTy::Binary))),
            );
        }

        obj.insert(
            "bytes".into(),
            SJValue::Array(
                self.0
                    .into_iter()
                    .map(|n| SJValue::Number(Number::from(n)))
                    .collect(),
            ),
        );

        SJValue::Object(obj)
    }

    #[must_use]
    pub fn ser(&self) -> (BinaryCompression, Vec<u8>) {
        let vanilla = {
            let mut backing = Integer::usize(self.0.len()).ser().1;
            backing.extend(&self.0);
            backing
        };
        let rle = rle(self.0.clone());
        let lz = lz(&self.0);

        if vanilla.len() <= rle.len() && vanilla.len() <= lz.len() {
            (BinaryCompression::Nothing, vanilla)
        } else if rle.len() <= lz.len() {
            (BinaryCompression::RunLengthEncoding, rle)
        } else {
            (BinaryCompression::LempelZiv, lz)
        }
    }

    ///Uncompresses bytes using the specified method.
    ///
    /// # Errors
    /// - [`IntegerSerError`] if we cannot deserialise the length
    /// - [`BinarySerError::NotEnoughBytes`] if there are not enough bytes
    /// - [`BinarySerError`] if there are any issues with the Run-Length-Encoding
    pub fn deser(
        compression: BinaryCompression,
        cursor: &mut Cursor<u8>,
    ) -> Result<Self, BinarySerError> {
        Ok(match compression {
            BinaryCompression::Nothing => {
                let length = Integer::deser(SignedState::Unsigned, cursor)?.try_into()?;
                Self(
                    cursor
                        .read(length)
                        .ok_or(BinarySerError::NotEnoughBytes)?
                        .to_vec(),
                )
            }
            BinaryCompression::RunLengthEncoding => Self(un_rle(cursor)?),
            BinaryCompression::LempelZiv => Self(un_lz(cursor)?),
        })
    }
}
