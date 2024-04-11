use std::{
    fmt::{Debug, Display, Formatter},
    io::{Cursor, Error as IOError, Read},
};

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum IntegerContent {
    Small([u8; 1]),
    Smedium([u8; 2]),
    Medium([u8; 4]),
    Large([u8; 8]),
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum SignedState {
    Unsigned,
    SignedPositive,
    SignedNegative,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Integer {
    signed_state: SignedState,
    content: IntegerContent,
}

#[derive(Copy, Clone, Debug)]
pub(super) enum IntegerDiscriminant {
    Small,
    Smedium,
    Medium,
    Large,
}

impl IntegerDiscriminant {
    pub const fn bytes(self) -> usize {
        match self {
            IntegerDiscriminant::Small => 1,
            IntegerDiscriminant::Smedium => 2,
            IntegerDiscriminant::Medium => 4,
            IntegerDiscriminant::Large => 8,
        }
    }

    pub fn iterator_to_size_can_fit_in(
        iter: impl Iterator<Item = u8> + DoubleEndedIterator + ExactSizeIterator,
        length_minus_one: usize,
    ) -> Self {
        let mut last_zeroed = length_minus_one;
        for (i, b) in iter.enumerate().rev() {
            if b != 0 {
                break;
            } else {
                last_zeroed = i;
            }
        }

        if last_zeroed <= 1 {
            IntegerDiscriminant::Small
        } else if last_zeroed <= 2 {
            IntegerDiscriminant::Smedium
        } else if last_zeroed <= 4 {
            IntegerDiscriminant::Medium
        } else {
            IntegerDiscriminant::Large
        }
    }
}

macro_rules! new_x {
    ($t:ty => $name:ident, $disc:ident) => {
        impl Integer {
            pub fn $name(n: $t) -> Self {
                let arr = n.to_le_bytes();
                Self {
                    signed_state: SignedState::Unsigned,
                    content: IntegerContent::$disc(arr),
                }
            }
        }

        impl TryInto<$t> for Integer {
            type Error = IntegerSerError;

            fn try_into(self) -> Result<$t, Self::Error> {
                if self.signed_state != SignedState::Unsigned {
                    return Err(IntegerSerError::WrongType);
                }

                match self.content {
                    IntegerContent::$disc(bytes) => Ok(<$t>::from_le_bytes(bytes)),
                    _ => Err(IntegerSerError::WrongType),
                }
            }
        }
    };
    ($t:ty =>> $name:ident, $disc:ident) => {
        impl Integer {
            pub fn $name(n: $t) -> Self {
                if n < 0 {
                    let arr = (-n).to_le_bytes();
                    Self {
                        signed_state: SignedState::SignedNegative,
                        content: IntegerContent::$disc(arr),
                    }
                } else {
                    let arr = n.to_le_bytes();
                    Self {
                        signed_state: SignedState::SignedPositive,
                        content: IntegerContent::$disc(arr),
                    }
                }
            }
        }

        impl TryInto<$t> for Integer {
            type Error = IntegerSerError;

            fn try_into(self) -> Result<$t, Self::Error> {
                if self.signed_state == SignedState::Unsigned {
                    return Err(IntegerSerError::WrongType);
                }

                let raw_n = match self.content {
                    IntegerContent::$disc(bytes) => <$t>::from_le_bytes(bytes),
                    _ => return Err(IntegerSerError::WrongType),
                };

                Ok(if self.signed_state == SignedState::SignedPositive {
                    raw_n
                } else {
                    -raw_n
                })
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

impl Display for Integer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match (&self.content, self.signed_state) {
            (IntegerContent::Small(b), SignedState::SignedNegative) => {
                write!(f, "{}", -i8::from_le_bytes(*b))
            }
            (IntegerContent::Small(b), _) => write!(f, "{}", u8::from_le_bytes(*b)),
            (IntegerContent::Smedium(b), SignedState::SignedNegative) => {
                write!(f, "{}", -i16::from_le_bytes(*b))
            }
            (IntegerContent::Smedium(b), _) => write!(f, "{}", u16::from_le_bytes(*b)),
            (IntegerContent::Medium(b), SignedState::SignedNegative) => {
                write!(f, "{}", -i32::from_le_bytes(*b))
            }
            (IntegerContent::Medium(b), _) => write!(f, "{}", u32::from_le_bytes(*b)),
            (IntegerContent::Large(b), SignedState::SignedNegative) => {
                write!(f, "{}", -i64::from_le_bytes(*b))
            }
            (IntegerContent::Large(b), _) => write!(f, "{}", u64::from_le_bytes(*b)),
        }
    }
}

impl Debug for Integer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let displayed = self.to_string();
        let signed_state = self.signed_state;

        f.debug_struct("Integer")
            .field("variant", &self.as_disc())
            .field("signed_state", &signed_state)
            .field("value", &displayed)
            .finish()
    }
}

impl From<IntegerDiscriminant> for u8 {
    fn from(value: IntegerDiscriminant) -> Self {
        match value {
            IntegerDiscriminant::Small => 0b001,
            IntegerDiscriminant::Smedium => 0b010,
            IntegerDiscriminant::Medium => 0b011,
            IntegerDiscriminant::Large => 0b100,
        }
    }
}

impl TryFrom<u8> for IntegerDiscriminant {
    type Error = IntegerSerError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0b001 => Ok(Self::Small),
            0b010 => Ok(Self::Smedium),
            0b011 => Ok(Self::Medium),
            0b100 => Ok(Self::Large),
            _ => Err(IntegerSerError::InvalidDiscriminant),
        }
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
            _ => Err(IntegerSerError::InvalidDiscriminant),
        }
    }
}

#[derive(Debug)]
pub enum IntegerSerError {
    InvalidDiscriminant,
    NotEnoughBytes,
    WrongType,
    IOError(IOError),
}

impl From<IOError> for IntegerSerError {
    fn from(value: IOError) -> Self {
        Self::IOError(value)
    }
}

impl Integer {
    fn as_disc(&self) -> IntegerDiscriminant {
        match &self.content {
            IntegerContent::Small(_) => IntegerDiscriminant::Small,
            IntegerContent::Smedium(_) => IntegerDiscriminant::Smedium,
            IntegerContent::Medium(_) => IntegerDiscriminant::Medium,
            IntegerContent::Large(_) => IntegerDiscriminant::Large,
        }
    }

    pub fn ser(self) -> Vec<u8> {
        //disc structure:
        //2 bit: signed state
        //3 bits: original size
        //3 bits: stored size

        let original_size = self.as_disc();
        let mut at_max = [0_u8; 8];
        let stored_size = match self.content {
            IntegerContent::Small([b]) => {
                at_max[0] = b;
                IntegerDiscriminant::Small
            }
            IntegerContent::Smedium(b) => {
                for (i, b) in b.iter().enumerate() {
                    at_max[i] = *b;
                }
                IntegerDiscriminant::iterator_to_size_can_fit_in(b.into_iter(), b.len() - 1)
            }
            IntegerContent::Medium(b) => {
                for (i, b) in b.iter().enumerate() {
                    at_max[i] = *b;
                }
                IntegerDiscriminant::iterator_to_size_can_fit_in(b.into_iter(), b.len() - 1)
            }
            IntegerContent::Large(b) => {
                for (i, b) in b.iter().enumerate() {
                    at_max[i] = *b;
                }
                IntegerDiscriminant::iterator_to_size_can_fit_in(b.into_iter(), b.len() - 1)
            }
        };

        let mut res = Vec::with_capacity(1 + stored_size.bytes());
        let discriminant: u8 = (u8::from(self.signed_state) << 6)
            | (u8::from(original_size) << 3)
            | u8::from(stored_size);
        res.push(discriminant);
        res.extend(&at_max[0..stored_size.bytes()]);

        // println!("Storing {self} from {original_size:?} in {stored_size:?}");

        res
    }

    pub fn deser(reader: &mut Cursor<impl Read + AsRef<[u8]>>) -> Result<Self, IntegerSerError> {
        let mut discriminant = [0_u8];
        let (signed_state, original, stored) = match reader.read(&mut discriminant)? {
            0 => return Err(IntegerSerError::NotEnoughBytes),
            1 => {
                let [discriminant] = discriminant;
                let signed_state = SignedState::try_from((discriminant & 0b1100_0000) >> 6)?;
                let original = IntegerDiscriminant::try_from((discriminant & 0b0011_1000) >> 3)?;
                let stored = IntegerDiscriminant::try_from(discriminant & 0b0000_0111)?;

                (signed_state, original, stored)
            }
            _ => unreachable!("Can only read 1 byte"),
        };

        let mut tmp = [0_u8; 1];
        let mut read_bytes = Vec::with_capacity(stored.bytes());
        loop {
            if read_bytes.len() == stored.bytes() {
                break;
            }

            match reader.read(&mut tmp)? {
                0 => return Err(IntegerSerError::NotEnoughBytes),
                n => read_bytes.extend(&tmp[0..n]),
            };
        }

        Ok(match original {
            IntegerDiscriminant::Small => {
                let mut bytes = [0_u8; 1];
                for (i, b) in read_bytes.into_iter().enumerate() {
                    bytes[i] = b;
                }
                Self {
                    signed_state,
                    content: IntegerContent::Small(bytes),
                }
            }
            IntegerDiscriminant::Smedium => {
                let mut bytes = [0_u8; 2];
                for (i, b) in read_bytes.into_iter().enumerate() {
                    bytes[i] = b;
                }
                Self {
                    signed_state,
                    content: IntegerContent::Smedium(bytes),
                }
            }
            IntegerDiscriminant::Medium => {
                let mut bytes = [0_u8; 4];
                for (i, b) in read_bytes.into_iter().enumerate() {
                    bytes[i] = b;
                }
                Self {
                    signed_state,
                    content: IntegerContent::Medium(bytes),
                }
            }
            IntegerDiscriminant::Large => {
                let mut bytes = [0_u8; 8];
                for (i, b) in read_bytes.into_iter().enumerate() {
                    bytes[i] = b;
                }
                Self {
                    signed_state,
                    content: IntegerContent::Large(bytes),
                }
            }
        })
    }
}
