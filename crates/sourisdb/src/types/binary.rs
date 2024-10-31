use crate::{
    display_bytes_as_hex_array,
    types::integer::{Integer, IntegerSerError, SignedState},
    utilities::cursor::Cursor,
    values::ValueTy,
};
use alloc::{vec, vec::Vec};
use core::fmt::{Debug, Display, Formatter};
use itertools::Itertools;
use serde_json::{Map as SJMap, Number, Value as SJValue};

#[derive(Debug, Copy, Clone)]
pub enum BinaryCompression {
    Nothing,
    RunLengthEncoding,
    XORedRunLengthEncoding,
}

impl From<BinaryCompression> for u8 {
    fn from(compression: BinaryCompression) -> Self {
        match compression {
            BinaryCompression::Nothing => 0,
            BinaryCompression::RunLengthEncoding => 1,
            BinaryCompression::XORedRunLengthEncoding => 2,
        }
    }
}

impl TryFrom<u8> for BinaryCompression {
    type Error = BinarySerError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Nothing),
            1 => Ok(Self::RunLengthEncoding),
            2 => Ok(Self::XORedRunLengthEncoding),
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
    pub fn rle(&self) -> Vec<u8> {
        let mut output = vec![];

        let mut bytes = self.0.clone();
        bytes.reverse();

        let mut current_count = 1;
        let Some(mut current) = bytes.pop() else {
            return output;
        };

        while let Some(byte) = bytes.pop() {
            if current != byte {
                output.push(current_count);
                output.push(current);

                current_count = 0;
                current = byte;
            }
            current_count += 1;

            if current_count == u8::MAX {
                output.push(current_count);
                output.push(current);

                current_count = 0;
            }
        }
        if current_count != 0 {
            output.push(current_count);
            output.push(current);
        }

        output
    }

    fn un_rle(len: usize, cursor: &mut Cursor<u8>) -> Result<Self, BinarySerError> {
        //complicated version that's like 70% slower lol
        /*        let mut alloc_size = 0;
                let mut to_be_added = Vec::with_capacity(len);

                for _ in 0..len {
                    let [count, byte] = cursor.read_exact().copied().ok_or(BinarySerError::NotEnoughBytes)?;
                    alloc_size += count as usize;
                    to_be_added.push((count, byte));
                }

                let mut output = Vec::with_capacity(alloc_size);

                for (count, byte) in to_be_added {
                    (0..count).for_each(|_| output.push(byte));
                }
        */

        let mut output = vec![];
        for (count, byte) in cursor
            .read(len * 2)
            .ok_or(BinarySerError::NotEnoughBytes)?
            .iter()
            .copied()
            .tuples()
        {
            (0..count).for_each(|_| output.push(byte));
        }

        Ok(Self(output))
    }

    #[must_use]
    pub fn ser(&self) -> (BinaryCompression, Vec<u8>) {
        let vanilla = {
            let mut backing = Integer::usize(self.0.len()).ser().1;
            backing.extend(&self.0);
            backing
        };
        let rle = {
            let rle = self.rle();

            let mut out = Integer::usize(rle.len() / 2).ser().1;
            out.extend(&rle);
            out
        };
        // let rle_xor = {
        //TODO eventually
        // };

        if vanilla.len() <= rle.len() {
            (BinaryCompression::Nothing, vanilla)
        } else {
            (BinaryCompression::RunLengthEncoding, rle)
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
        let len = Integer::deser(SignedState::Unsigned, cursor)?.try_into()?;

        Ok(match compression {
            BinaryCompression::Nothing => Self(
                cursor
                    .read(len)
                    .ok_or(BinarySerError::NotEnoughBytes)?
                    .to_vec(),
            ),
            BinaryCompression::RunLengthEncoding => Self::un_rle(len, cursor)?,
            BinaryCompression::XORedRunLengthEncoding => {
                unimplemented!()
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rle_specific_cases() {
        const CASES: &[&[u8]] = &[
            &[
                0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0xFF,
            ],
            &[],
            &[0],
            &[0x12, 0x12, 0x12, 0xDE, 0xAD, 0xBE, 0xEF],
        ];

        for case in CASES {
            let vec = case.to_vec();
            let bd = BinaryData(vec.clone());

            let encoded = {
                let rle = bd.rle(); //forcing RLE to ensure it works
                let mut out = Integer::usize(rle.len() / 2).ser().1;
                out.extend(&rle);
                out
            };

            let mut cursor = Cursor::new(&encoded);
            let BinaryData(decoded) =
                BinaryData::deser(BinaryCompression::RunLengthEncoding, &mut cursor).unwrap();

            assert_eq!(decoded, vec);
        }
    }
}
