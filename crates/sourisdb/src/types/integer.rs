use crate::utilities::cursor::Cursor;
use alloc::{string::ToString, vec::Vec};
use core::{
    fmt::{Debug, Display, Formatter},
    num::ParseIntError,
    ops::Neg,
    str::FromStr,
};


#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SignedState {
    Unsigned,
    SignedPositive,
    SignedNegative,
}

const INTEGER_MAX_SIZE: usize = 8;
const INTEGER_BITS: usize = 4;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Integer {
    signed_state: SignedState,
    content: [u8; INTEGER_MAX_SIZE],
}

impl Neg for Integer {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self {
            signed_state: match self.signed_state {
                SignedState::Unsigned => SignedState::Unsigned,
                SignedState::SignedPositive => SignedState::SignedNegative,
                SignedState::SignedNegative => SignedState::SignedPositive,
            },
            content: self.content,
        }
    }
}

impl Integer {
    fn unsigned_bits (&self) -> u32 {
        u64::from_le_bytes(self.content).ilog2()
    }
    
    ///NB: always <= 8
    fn min_bytes_needed(&self) -> usize {
        ((self.unsigned_bits() / 8) + 1) as usize
    }

    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.signed_state == SignedState::SignedNegative
    }

    #[must_use]
    pub fn is_positive(&self) -> bool {
        !self.is_negative()
    }
}

impl Display for Integer {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self.signed_state {
            SignedState::SignedNegative => {
                write!(f, "{}", -i64::from_le_bytes(self.content))
            },
            _ => {
                write!(f, "{}", u64::from_le_bytes(self.content))
            }
        }
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl Debug for Integer {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let displayed = self.to_string();
        let signed_state = self.signed_state;

        f.debug_struct("Integer")
            .field("signed_state", &signed_state)
            .field("value", &displayed)
            .finish()
    }
}

macro_rules! new_x {
    ($t:ty => $name:ident, $disc:ident) => {
        impl Integer {
            #[must_use]
            pub fn $name(n: $t) -> Self {
                Self::from(n)
            }
        }

        impl From<$t> for Integer {
            fn from(n: $t) -> Self {
                let mut content = [0_u8; INTEGER_MAX_SIZE];
                for (i, b) in n.to_le_bytes().into_iter().enumerate() {
                    content[i] = b;
                }
                Self {
                    signed_state: SignedState::Unsigned,
                    content,
                }
            }
        }

        impl TryFrom<Integer> for $t {
            type Error = IntegerSerError;

            fn try_from(i: Integer) -> Result<Self, Self::Error> {
                if i.signed_state == SignedState::SignedNegative {
                    return Err(IntegerSerError::WrongType);
                }

                if i.unsigned_bits() > <$t>::BITS {
                    return Err(IntegerSerError::WrongType);
                }
                
                let mut out = [0_u8; (<$t>::BITS / 8) as usize];
                for (i, b) in i.content.into_iter().enumerate().take((<$t>::BITS / 8) as usize) {
                    out[i] = b;
                }
                
                Ok(<$t>::from_le_bytes(out))
            }
        }
    };
    ($t:ty =>> $name:ident, $disc:ident) => {
        impl Integer {
            #[must_use]
            pub fn $name(n: $t) -> Self {
                Self::from(n)
            }
        }

        impl From<$t> for Integer {
            fn from(n: $t) -> Self {
                if n < 0 {
                    let mut content = [0_u8; INTEGER_MAX_SIZE];
                    for (i, b) in (-n).to_le_bytes().into_iter().enumerate() {
                        content[i] = b;
                    }
                    Self {
                        signed_state: SignedState::SignedNegative,
                        content,
                    }
                } else {
                    let mut content = [0_u8; INTEGER_MAX_SIZE];
                    for (i, b) in n.to_le_bytes().into_iter().enumerate() {
                        content[i] = b;
                    }   
                    Self {
                        signed_state: SignedState::SignedPositive,
                        content,
                    }
                }
            }
        }

        impl TryFrom<Integer> for $t {
            type Error = IntegerSerError;

            fn try_from(i: Integer) -> Result<Self, Self::Error> {
                let multiplier = match i.signed_state {
                    SignedState::SignedNegative => -1,
                    _ => 1
                };

                if i.unsigned_bits() > <$t>::BITS {
                    return Err(IntegerSerError::WrongType);
                }
                
                let mut out = [0_u8; (<$t>::BITS / 8) as usize];
                for (i, b) in i.content.into_iter().enumerate().take((<$t>::BITS / 8) as usize) {
                    out[i] = b;
                }
                
                Ok(<$t>::from_le_bytes(out) * multiplier)
            }
        }
    };
}

new_x!(u8 => u8, Small);
new_x!(i8 =>> i8, Small);
new_x!(u16 => u16, Smedium);
new_x!(i16 =>> i16, Smedium);
new_x!(u32 => u32, Medium);
new_x!(i32 =>> i32, Medium);
new_x!(usize => usize, Large);
new_x!(isize =>> isize, Large);
new_x!(u64 => u64, Large);
new_x!(i64 =>> i64, Large);

impl FromStr for Integer {
    type Err = IntegerSerError;

    #[allow(clippy::cast_possible_truncation)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(IntegerSerError::NotEnoughBytes);
        };

        if s == "0" {
            return Ok(Self {
                signed_state: SignedState::Unsigned,
                content: [0; 8],
            });
        }

        let (s, signed_state) = if s.as_bytes()[0] == b'-' {
            (&s[1..], SignedState::SignedNegative)
        } else {
            (s, SignedState::Unsigned)
        };

        let content: u64 = s.parse()?;
        
        //TODO: use ilog2

        Ok(Self {
            signed_state,
            content: content.to_le_bytes()
        })
    }
}

impl From<SignedState> for u8 {
    fn from(value: SignedState) -> Self {
        match value {
            SignedState::Unsigned => 0b01,
            SignedState::SignedPositive => 0b10,
            SignedState::SignedNegative => 0b11,
        }
    }
}
impl TryFrom<u8> for SignedState {
    type Error = IntegerSerError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0b01 => Ok(Self::Unsigned),
            0b10 => Ok(Self::SignedPositive),
            0b11 => Ok(Self::SignedNegative),
            _ => Err(IntegerSerError::InvalidSignedStateDiscriminant(value)),
        }
    }
}

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum IntegerSerError {
    InvalidSignedStateDiscriminant(u8),
    InvalidIntegerSizeDiscriminant(u8),
    NotEnoughBytes,
    WrongType,
    IntegerParseError(ParseIntError),
}

impl From<ParseIntError> for IntegerSerError {
    fn from(value: ParseIntError) -> Self {
        Self::IntegerParseError(value)
    }
}

impl Display for IntegerSerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            IntegerSerError::InvalidSignedStateDiscriminant(b) => {
                write!(f, "Invalid signed state discriminant found: {b:#b}")
            }
            IntegerSerError::InvalidIntegerSizeDiscriminant(b) => {
                write!(f, "Invalid integer size discriminant found: {b:#b}")
            }
            IntegerSerError::NotEnoughBytes => write!(f, "Not enough bytes provided"),
            IntegerSerError::WrongType => write!(
                f,
                "Attempted to deserialise into different type than was originally serialised from"
            ),
            IntegerSerError::IntegerParseError(e) => {
                write!(f, "Error parsing from base-10 string: {e:?}")
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for IntegerSerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IntegerParseError(e) => Some(e),
            _ => None,
        }
    }
}

impl Integer {
    #[must_use]
    pub fn ser(self) -> Vec<u8> {
        //disc structure:
        //2 bit: signed state
        //3 
        //TODO: use ilog2

        let stored_size = self.min_bytes_needed();
        let bytes = self.content;
        
        let mut res = Vec::with_capacity(1 + stored_size);
        let discriminant: u8 = (u8::from(self.signed_state) << 6)
            | ((stored_size as u8) << (8 - 2 - INTEGER_BITS));
        res.push(discriminant);
        res.extend(&bytes[0..stored_size]);

        res
    }

    pub fn deser(reader: &mut Cursor<u8>) -> Result<Self, IntegerSerError> {
        let (signed_state, stored) = {
            let [discriminant] = reader.read(1).ok_or(IntegerSerError::NotEnoughBytes)? else {
                unreachable!("didn't get just one byte back")
            };
            let discriminant = *discriminant;
            let signed_state = SignedState::try_from((discriminant & 0b1100_0000) >> 6)?;
            let stored = usize::from((discriminant & 0b0011_1100) >> (8 - 2 - INTEGER_BITS));

            (signed_state, stored)
        };
        
        let mut content = [0_u8; INTEGER_MAX_SIZE];
        for (i,b) in reader
            .read(stored)
            .ok_or(IntegerSerError::NotEnoughBytes)?.iter().copied().enumerate() {
            content[i] = b;
        }
        
        Ok(Self {
            signed_state, content
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{types::integer::Integer, utilities::cursor::Cursor};
    use alloc::{format, string::ToString};
    use core::str::FromStr;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn doesnt_crash (s in "\\PC*") {
            let _ = Integer::from_str(&s);
        }

        #[test]
        fn parse_valids (i in any::<i64>()) {
            let int = Integer::from_str(&i.to_string()).unwrap();
            prop_assert_eq!(i64::try_from(int).unwrap(), i);
        }

        #[test]
        fn back_to_original (i in any::<i64>()) {
            let s = i.to_string();

            let parsed = Integer::from_str(&s).unwrap();

            let sered = parsed.ser();
            let got_back = Integer::deser(&mut Cursor::new(&sered)).unwrap();
            prop_assert_eq!(parsed, got_back);

            prop_assert_eq!(i64::try_from(got_back).unwrap(), i);
        }
        
        #[test]
        fn back_to_original_other_size (i in any::<u16>()) {
            let s = i.to_string();

            let parsed = Integer::from_str(&s).unwrap();

            let sered = parsed.ser();
            let got_back = Integer::deser(&mut Cursor::new(&sered)).unwrap();
            prop_assert_eq!(parsed, got_back);

            prop_assert_eq!(u32::try_from(got_back).unwrap(), u32::from(i));
        }
    }
}
