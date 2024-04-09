use std::io::{Error as IOError, Read};

pub enum Integer {
    Small([u8; 1], bool),
    Smedium([u8; 2], bool),
    Medium([u8; 4], bool),
    Large([u8; 8], bool),
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
    pub fn ser (self) -> Vec<u8> {
        let mut res = vec![];

        let disc: u8 = match &self {
            Self::Small(_, true) =>    0b0001_0001,
            Self::Small(_, false) =>   0b0001_0000,
            Self::Medium(_, true) =>   0b0010_0001,
            Self::Medium(_, false) =>  0b0010_0000,
            Self::Smedium(_, true) =>  0b0011_0001,
            Self::Smedium(_, false) => 0b0011_0000,
            Self::Large(_, true) =>    0b0100_0001,
            Self::Large(_, false) =>   0b0100_0000
        };
        res.push(disc);

        match self {
            Self::Small(b, _) => res.push(b[0]),
            Self::Smedium(b, _) => res.extend(b),
            Self::Medium(b, _) => res.extend(b),
            Self::Large(b, _) => res.extend(b)
        };

        res
    }

    pub fn deser<R: Read> (mut reader: R) -> Result<Self, IntegerSerError> {
        let mut discriminant = [0_u8];
        match reader.read(&mut discriminant)? {
            0 => return Err(IntegerSerError::NotEnoughBytes),
            1 => {},
            _ => unreachable!("Can only read 1 byte"),
        }
        let [discriminant] = discriminant;
        let is_signed = discriminant << 4 > 0;

        match discriminant >> 4 {
            0b0001 => {
                let mut bytes = [0_u8; 1];
                match reader.read(&mut bytes)? {
                    n if n < 1 => Err(IntegerSerError::NotEnoughBytes),
                    1 => Ok(Self::Small(bytes, is_signed)),
                    _ => unreachable!("Can only read 1 byte"),
                }
            },
            0b0010 => {
                let mut bytes = [0_u8; 2];
                match reader.read(&mut bytes)? {
                    n if n < 2 => Err(IntegerSerError::NotEnoughBytes),
                    2 => Ok(Self::Smedium(bytes, is_signed)),
                    _ => unreachable!("Can only read 1 byte"),
                }
            },
            0b0011 => {
                let mut bytes = [0_u8; 4];
                match reader.read(&mut bytes)? {
                    n if n < 4 => Err(IntegerSerError::NotEnoughBytes),
                    4 => Ok(Self::Medium(bytes, is_signed)),
                    _ => unreachable!("Can only read 1 byte"),
                }

            },
            0b0100 => {
                let mut bytes = [0_u8; 8];
                match reader.read(&mut bytes)? {
                    n if n < 8 => Err(IntegerSerError::NotEnoughBytes),
                    8 => Ok(Self::Large(bytes, is_signed)),
                    _ => unreachable!("Can only read 1 byte"),
                }

            },
            _ => Err(IntegerSerError::InvalidDiscriminant),
        }
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
