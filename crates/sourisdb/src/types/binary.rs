use core::fmt::{Debug, Display, Formatter};
use crate::display_bytes_as_hex_array;
use serde_json::{Value as SJValue, Map as SJMap, Number};
use crate::types::integer::{Integer, IntegerSerError, SignedState};
use crate::utilities::bits::Bits;
use crate::utilities::cursor::Cursor;
use crate::values::ValueTy;

pub enum BinaryCompression {
    Nothing,
    RunLengthEncoding,
    XORedRunLengthEncoding
}

impl Into<u8> for BinaryCompression {
    fn into(self) -> u8 {
        match self {
            Self::Nothing => 0,
            Self::RunLengthEncoding => 1,
            Self::XORedRunLengthEncoding => 2
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
            _ => Err(BinarySerError::NoCompressionTypeFound(value))
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
            Self::NoCompressionTypeFound(v) => write!(f, "Invalid compression discriminant found: {v}"),
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
pub struct BinaryData (Vec<u8>);

impl Debug for BinaryData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
    pub fn to_json (self, add_souris_types: bool) -> SJValue {
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
                self.0.into_iter()
                    .map(|n| SJValue::Number(Number::from(n)))
                    .collect(),
            ),
        );

        SJValue::Object(obj)
    }

    fn rle (&self) -> Vec<u8> {
        let mut output = vec![];

        if self.0.is_empty() {
            return output;
        }

            let mut bits = {
                let mut cloned = self.0.clone();
                cloned.reverse(); //reverse it to get the first bits at the end where I can easily pop them
                Bits::from(cloned)
            };

            let Some(mut current_bit) = bits.pop() else {
                unreachable!()
            };
            let mut current_count: u8 = 1;

            while let Some(found_bit) = bits.pop() {
                if current_count == 0b1111_1110 || found_bit != current_bit {
                    let to_be_pushed = current_count + (current_bit as u8);
                    output.push(to_be_pushed);
                    current_count = 0;
                }

                current_bit = found_bit;
                current_count += 1;
            }

        output

    }

    pub fn ser (&self) -> (BinaryCompression, Vec<u8>) {
        let vanilla = {
            let mut backing = Integer::usize(self.0.len()).ser().1;
            backing.extend(&self.0);
            backing
        };
        let rle = {
            let rle = self.rle();

            let mut out = Integer::usize(rle.len()).ser().1;
            out.extend(&rle);
            out
        };
        let rle_xor = {
            //TODO someday when i get internet access and time
        };

        if vanilla.len() < rle.len() {
            (BinaryCompression::Nothing, vanilla)
        } else {
            (BinaryCompression::RunLengthEncoding, rle)
        }
    }

    pub fn deser (compression: BinaryCompression, cursor: &mut Cursor<u8>) -> Result<Self, BinarySerError> {
        let len = Integer::deser(SignedState::Unsigned, cursor)?.try_into()?;
        let bytes = cursor.read(len).ok_or(BinarySerError::NotEnoughBytes)?;

        Ok(match compression {
            BinaryCompression::Nothing => {
                Self(bytes.to_vec())
            }
            BinaryCompression::RunLengthEncoding => {
                let mut output = Bits::default();

                for byte in bytes {
                    let bit = if byte & 0b0000_0001 > 0 { true } else { false };
                    let len = byte & 0b1111_1110;

                    (0..len).into_iter().for_each(|_| output.push(bit));
                }

                Self(output.get_proper_bytes())
            },
            BinaryCompression::XORedRunLengthEncoding => {
                todo!("when i get internet access and time :)")
            }
        })
    }
}