use std::io::{Error as IOError, Read};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Integer {
    Small([u8; 1], bool),
    Smedium([u8; 2], bool),
    Medium([u8; 4], bool),
    Large([u8; 8], bool),
}

#[derive(Copy, Clone, Debug)]
enum IntegerDiscriminant {
    Small,
    Smedium,
    Medium,
    Large
}

impl IntegerDiscriminant {
    pub const fn bytes(self) -> usize {
        match self {
            IntegerDiscriminant::Small => 1,
            IntegerDiscriminant::Smedium => 2,
            IntegerDiscriminant::Medium => 4,
            IntegerDiscriminant::Large => 8
        }
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

#[derive(Debug)]
pub enum IntegerSerError {
    InvalidDiscriminant,
    NotEnoughBytes,
    IOError(IOError)
}

impl From<IOError> for IntegerSerError {
    fn from(value: IOError) -> Self {
        Self::IOError(value)
    }
}

impl Integer {
    fn to_disc (self) -> IntegerDiscriminant {
        match self {
            Self::Small(_, _) => IntegerDiscriminant::Small,
            Self::Smedium(_, _) => IntegerDiscriminant::Smedium,
            Self::Medium(_, _) => IntegerDiscriminant::Medium,
            Self::Large(_, _) => IntegerDiscriminant::Large,
        }
    }

    pub fn ser (self) -> Vec<u8> {
        //disc structure:
        //1 bit: is signed
        //3 bits: original size
        //3 bits: stored size

        let original_size = self.to_disc();
        let mut at_max = [0_u8; 8];
        let is_signed = match self {
            Self::Small(b, s) => {
                for (i, b) in b.into_iter().enumerate() {
                    at_max[i] = b;
                }
                s as u8
            },
            Self::Smedium(b, s) => {
                for (i, b) in b.into_iter().enumerate() {
                    at_max[i] = b;
                }
                s as u8
            },
            Self::Medium(b, s) => {
                for (i, b) in b.into_iter().enumerate() {
                    at_max[i] = b;
                }
                s as u8
            },
            Self::Large(b, s) => {
                for (i, b) in b.into_iter().enumerate() {
                    at_max[i] = b;
                }
                s as u8
            },
        };

        let mut res: Vec<u8> = Vec::with_capacity(9);
        let at_max_u64 = u64::from_le_bytes(at_max);
        let stored_size = if at_max_u64 < u8::MAX as u64 {
            res.extend(&at_max[0..1]);
            IntegerDiscriminant::Small
        } else if at_max_u64 < u16::MAX as u64 {
            res.extend(&at_max[0..2]);
            IntegerDiscriminant::Smedium
        } else if at_max_u64 < u32::MAX as u64 {
            res.extend(&at_max[0..4]);
            IntegerDiscriminant::Medium
        } else {
            res.extend(at_max);
            IntegerDiscriminant::Large
        };

        let discriminant: u8 = (is_signed << 7) | (u8::from(original_size) << 4) | (u8::from(stored_size) << 1);
        res.insert(0, discriminant);

        res
    }

    pub fn deser<R: Read> (mut reader: R) -> Result<Self, IntegerSerError> {
        let mut discriminant = [0_u8];
        let (is_signed, original, stored) = match reader.read(&mut discriminant)? {
            0 => {
                return Err(IntegerSerError::NotEnoughBytes)
            },
            1 => {
                let [discriminant] = discriminant;
                let is_signed = discriminant >> 7 > 0;
                let original = IntegerDiscriminant::try_from((discriminant & 0b0111_0000) >> 4)?;
                let stored = IntegerDiscriminant::try_from((discriminant & 0b0000_1110) >> 1)?;

                (is_signed, original, stored)
            },
            _ => unreachable!("Can only read 1 byte"),
        };

        let mut tmp = [0_u8; 1];
        let mut read_bytes = Vec::with_capacity(stored.bytes());
        loop {
            match reader.read(&mut tmp)? {
                0 => if read_bytes.len() == stored.bytes() {
                    break;
                } else {
                    eprintln!("Expected to read {} bytes only read {} bytes", stored.bytes(), read_bytes.len());
                    return Err(IntegerSerError::NotEnoughBytes)
                },
                n => read_bytes.extend(&tmp[0..n])
            }
        }

        Ok(match original {
            IntegerDiscriminant::Small => {
                let mut bytes = [0_u8; 1];
                for (i, b) in read_bytes.into_iter().enumerate() {
                    bytes[i] = b;
                }
                Self::Small(bytes, is_signed)
            }
            IntegerDiscriminant::Smedium => {
                let mut bytes = [0_u8; 2];
                for (i, b) in read_bytes.into_iter().enumerate() {
                    bytes[i] = b;
                }
                Self::Smedium(bytes, is_signed)
            }
            IntegerDiscriminant::Medium => {
                let mut bytes = [0_u8; 4];
                for (i, b) in read_bytes.into_iter().enumerate() {
                    bytes[i] = b;
                }
                Self::Medium(bytes, is_signed)
            },
            IntegerDiscriminant::Large => {
                let mut bytes = [0_u8; 8];
                for (i, b) in read_bytes.into_iter().enumerate() {
                    bytes[i] = b;
                }
                Self::Large(bytes, is_signed)
            }
        })
    }
}

macro_rules! new_x {
    ($t:ty, $is_signed:expr => $name:ident, $self_ty:ident) => {
        impl Integer {
            pub fn $name (n: $t) -> Self {
                let arr = n.to_le_bytes();
                Self::$self_ty(arr, $is_signed)
            }
        }

        impl TryInto<$t> for Integer {
            type Error = ();

            fn try_into(self) -> Result<$t, Self::Error> {
                match self {
                    Self::$self_ty(bytes, $is_signed) => {
                        Ok(<$t>::from_le_bytes(bytes))
                    },
                    _ => Err(())
                }
            }
        }
    };
}

new_x!(u8, false => u8, Small);
new_x!(i8, true => i8, Small);
new_x!(u16, false => u16, Smedium);
new_x!(i16, true => i16, Smedium);
new_x!(u32, false => u32, Medium);
new_x!(i32, true => i32, Medium);
new_x!(usize, true => usize, Large);
new_x!(isize, true => isize, Large);
new_x!(u64, false => u64, Large);
new_x!(i64, true => i64, Large);
