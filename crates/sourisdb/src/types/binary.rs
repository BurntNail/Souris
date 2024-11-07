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
use core::ops::{Deref, DerefMut};
use lz4_flex::block::DecompressError;
use serde_json::{Map as SJMap, Number, Value as SJValue};
use crate::types::binary::huffman::{huffman, un_huffman};
use crate::utilities::huffman::HuffmanSerError;

pub mod lz;
pub mod rle;
mod huffman;

#[derive(Debug, Copy, Clone)]
pub enum BinaryCompression {
    Nothing,
    RunLengthEncoding,
    LempelZiv,
    Huffman,
}

impl From<BinaryCompression> for u8 {
    fn from(compression: BinaryCompression) -> Self {
        match compression {
            BinaryCompression::Nothing => 0,
            BinaryCompression::RunLengthEncoding => 1,
            BinaryCompression::LempelZiv => 2,
            BinaryCompression::Huffman => 3,
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
            3 => Ok(Self::Huffman),
            _ => Err(BinarySerError::NoCompressionTypeFound(value)),
        }
    }
}

#[derive(Debug)]
pub enum BinarySerError {
    NoCompressionTypeFound(u8),
    Integer(IntegerSerError),
    NotEnoughBytes,
    LzFlex(DecompressError),
    Huffman(HuffmanSerError)
}

impl Display for BinarySerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoCompressionTypeFound(v) => {
                write!(f, "Invalid compression discriminant found: {v}")
            }
            Self::Integer(i) => write!(f, "Error parsing integer: {i}"),
            Self::NotEnoughBytes => write!(f, "Not enough bytes to deserialize."),
            Self::LzFlex(e) => write!(f, "Error decompressing LZ: {e}"),
            Self::Huffman(e) => write!(f, "Error decompressing huffman: {e}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for BinarySerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NoCompressionTypeFound(_) | Self::NotEnoughBytes => None,
            Self::Integer(i) => Some(i),
            Self::LzFlex(e) => Some(e),
            Self::Huffman(e) => Some(e),
        }
    }
}

impl From<IntegerSerError> for BinarySerError {
    fn from(value: IntegerSerError) -> Self {
        Self::Integer(value)
    }
}
impl From<DecompressError> for BinarySerError {
    fn from(value: DecompressError) -> Self {
        Self::LzFlex(value)
    }
}
impl From<HuffmanSerError> for BinarySerError {
    fn from(value: HuffmanSerError) -> Self {
        Self::Huffman(value)
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BinaryData(pub Vec<u8>);

impl<T: AsRef<[u8]>> From<T> for BinaryData {
    fn from(value: T) -> Self {
        let slice = value.as_ref();
        let vec = slice.to_vec();
        Self(vec)
    }
}

impl From<BinaryData> for Vec<u8> {
    fn from(value: BinaryData) -> Self {
        value.0
    }
}

impl Deref for BinaryData {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for BinaryData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

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
    #[allow(clippy::missing_panics_doc)]
    pub fn ser(&self) -> (BinaryCompression, Vec<u8>) {
        let vanilla = {
            let mut backing = Integer::usize(self.0.len()).ser().1;
            backing.extend(&self.0);
            backing
        };
        let rle = rle(&self.0);
        let lz = lz(&self.0);
        let huffman = huffman(&self.0);

        [
            (BinaryCompression::Nothing, vanilla),
            (BinaryCompression::RunLengthEncoding, rle),
            (BinaryCompression::LempelZiv, lz),
            (BinaryCompression::Huffman, huffman)
        ]
        .into_iter()
        .min_by_key(|(_, v)| v.len())
        .unwrap()
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
            BinaryCompression::Huffman => Self(un_huffman(cursor)?)
        })
    }
}

#[cfg(test)]
const CASES: &[&[u8]] = &[
    &[
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0xFF,
    ],
    &[],
    &[0],
    &[0x12, 0x12, 0x12, 0xDE, 0xAD, 0xBE, 0xEF],
    &[0xAB; 10_000],
];
