use crate::{utilities::cursor::Cursor, version::Version};
use alloc::{string::ToString, vec::Vec};
use core::{
    fmt::{Debug, Display, Formatter},
    num::ParseIntError,
    ops::Neg,
    str::FromStr,
};

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum Content {
    Small([u8; 1]),
    Smedium([u8; 2]),
    Medium([u8; 4]),
    Large([u8; 8]),
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum SignedState {
    Unsigned,
    SignedPositive,
    SignedNegative,
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Integer {
    signed_state: SignedState,
    content: Content,
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
    fn as_disc(&self) -> IntegerDiscriminant {
        match &self.content {
            Content::Small(_) => IntegerDiscriminant::Small,
            Content::Smedium(_) => IntegerDiscriminant::Smedium,
            Content::Medium(_) => IntegerDiscriminant::Medium,
            Content::Large(_) => IntegerDiscriminant::Large,
        }
    }

    #[must_use]
    pub fn is_zero(&self) -> bool {
        match &self.content {
            Content::Small(s) => s.iter(),
            Content::Smedium(sm) => sm.iter(),
            Content::Medium(m) => m.iter(),
            Content::Large(l) => l.iter(),
        }
        .all(|b| *b == 0)
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
        match (&self.content, self.signed_state) {
            (Content::Small(b), SignedState::SignedNegative) => {
                write!(f, "{}", -i8::from_le_bytes(*b))
            }
            (Content::Small(b), _) => write!(f, "{}", u8::from_le_bytes(*b)),
            (Content::Smedium(b), SignedState::SignedNegative) => {
                write!(f, "{}", -i16::from_le_bytes(*b))
            }
            (Content::Smedium(b), _) => write!(f, "{}", u16::from_le_bytes(*b)),
            (Content::Medium(b), SignedState::SignedNegative) => {
                write!(f, "{}", -i32::from_le_bytes(*b))
            }
            (Content::Medium(b), _) => write!(f, "{}", u32::from_le_bytes(*b)),
            (Content::Large(b), SignedState::SignedNegative) => {
                write!(f, "{}", -i64::from_le_bytes(*b))
            }
            (Content::Large(b), _) => write!(f, "{}", u64::from_le_bytes(*b)),
        }
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl Debug for Integer {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let displayed = self.to_string();
        let signed_state = self.signed_state;

        f.debug_struct("Integer")
            .field("variant", &self.as_disc())
            .field("signed_state", &signed_state)
            .field("value", &displayed)
            .finish()
    }
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
        iter_len: usize,
    ) -> Self {
        let mut last_zeroed = iter_len;
        for (i, b) in iter.enumerate().rev() {
            if b != 0 {
                break;
            }

            last_zeroed = i;
        }

        if last_zeroed < 2 {
            IntegerDiscriminant::Small
        } else if last_zeroed < 3 {
            IntegerDiscriminant::Smedium
        } else if last_zeroed < 5 {
            IntegerDiscriminant::Medium
        } else {
            IntegerDiscriminant::Large
        }
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
                let arr = n.to_le_bytes();
                Self {
                    signed_state: SignedState::Unsigned,
                    content: Content::$disc(arr),
                }
            }
        }

        impl TryFrom<Integer> for $t {
            type Error = IntegerSerError;

            fn try_from(i: Integer) -> Result<Self, Self::Error> {
                if i.signed_state == SignedState::SignedNegative {
                    return Err(IntegerSerError::WrongType);
                }

                match i.content {
                    Content::$disc(bytes) => Ok(<$t>::from_le_bytes(bytes)),
                    _ => Err(IntegerSerError::WrongType),
                }
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
                    let arr = (-n).to_le_bytes();
                    Self {
                        signed_state: SignedState::SignedNegative,
                        content: Content::$disc(arr),
                    }
                } else {
                    let arr = n.to_le_bytes();
                    Self {
                        signed_state: SignedState::SignedPositive,
                        content: Content::$disc(arr),
                    }
                }
            }
        }

        impl TryFrom<Integer> for $t {
            type Error = IntegerSerError;

            fn try_from(i: Integer) -> Result<Self, Self::Error> {
                let raw_n = match i.content {
                    Content::$disc(bytes) => <$t>::from_le_bytes(bytes),
                    _ => return Err(IntegerSerError::WrongType),
                };

                Ok(if i.signed_state == SignedState::SignedNegative {
                    -raw_n
                } else {
                    raw_n
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
                content: Content::Small([0]),
            });
        }

        let (s, signed_state) = if s.as_bytes()[0] == b'-' {
            (&s[1..], SignedState::SignedNegative)
        } else {
            (s, SignedState::Unsigned)
        };

        let biggest: u64 = s.parse()?;
        let biggest_bytes = biggest.to_le_bytes();

        Ok(if biggest < 1 << 8 {
            Self {
                signed_state,
                content: Content::Small((biggest as u8).to_le_bytes()),
            }
        } else if biggest < 1 << 16 {
            Self {
                signed_state,
                content: Content::Smedium((biggest as u16).to_le_bytes()),
            }
        } else if biggest < 1 << 32 {
            Self {
                signed_state,
                content: Content::Medium((biggest as u32).to_le_bytes()),
            }
        } else {
            Self {
                signed_state,
                content: Content::Large(biggest_bytes),
            }
        })
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
            _ => Err(IntegerSerError::InvalidIntegerSizeDiscriminant(value)),
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
            IntegerSerError::IntegerParseError(e) => Some(e),
            _ => None,
        }
    }
}

impl Integer {
    #[must_use]
    pub fn ser(self, version: Version) -> Vec<u8> {
        match version {
            Version::V0_1_0 => {
                //disc structure:
                //2 bit: signed state
                //3 bits: original size
                //3 bits: stored size

                let original_size = self.as_disc();
                let mut at_max = [0_u8; 8];
                let stored_size = match self.content {
                    Content::Small([b]) => {
                        at_max[0] = b;
                        IntegerDiscriminant::Small
                    }
                    Content::Smedium(b) => {
                        for (i, b) in b.iter().enumerate() {
                            at_max[i] = *b;
                        }
                        IntegerDiscriminant::iterator_to_size_can_fit_in(b.into_iter(), 2)
                    }
                    Content::Medium(b) => {
                        for (i, b) in b.iter().enumerate() {
                            at_max[i] = *b;
                        }
                        IntegerDiscriminant::iterator_to_size_can_fit_in(b.into_iter(), 4)
                    }
                    Content::Large(b) => {
                        for (i, b) in b.iter().enumerate() {
                            at_max[i] = *b;
                        }
                        IntegerDiscriminant::iterator_to_size_can_fit_in(b.into_iter(), 8)
                    }
                };

                let mut res = Vec::with_capacity(1 + stored_size.bytes());
                let discriminant: u8 = (u8::from(self.signed_state) << 6)
                    | (u8::from(original_size) << 3)
                    | u8::from(stored_size);
                res.push(discriminant);
                res.extend(&at_max[0..stored_size.bytes()]);

                res
            }
        }
    }

    pub fn deser(reader: &mut Cursor<u8>, version: Version) -> Result<Self, IntegerSerError> {
        match version {
            Version::V0_1_0 => {
                let (signed_state, original, stored) = {
                    let [discriminant] = reader.read(1).ok_or(IntegerSerError::NotEnoughBytes)?
                    else {
                        unreachable!("didn't get just one byte back")
                    };
                    let discriminant = *discriminant;
                    let signed_state = SignedState::try_from((discriminant & 0b1100_0000) >> 6)?;
                    let original =
                        IntegerDiscriminant::try_from((discriminant & 0b0011_1000) >> 3)?;
                    let stored = IntegerDiscriminant::try_from(discriminant & 0b0000_0111)?;

                    (signed_state, original, stored)
                };

                let read_bytes = reader
                    .read(stored.bytes())
                    .ok_or(IntegerSerError::NotEnoughBytes)?;

                Ok(match original {
                    IntegerDiscriminant::Small => {
                        let mut bytes = [0_u8; 1];
                        for (i, b) in read_bytes.iter().copied().enumerate() {
                            bytes[i] = b;
                        }
                        Self {
                            signed_state,
                            content: Content::Small(bytes),
                        }
                    }
                    IntegerDiscriminant::Smedium => {
                        let mut bytes = [0_u8; 2];
                        for (i, b) in read_bytes.iter().copied().enumerate() {
                            bytes[i] = b;
                        }
                        Self {
                            signed_state,
                            content: Content::Smedium(bytes),
                        }
                    }
                    IntegerDiscriminant::Medium => {
                        let mut bytes = [0_u8; 4];
                        for (i, b) in read_bytes.iter().copied().enumerate() {
                            bytes[i] = b;
                        }
                        Self {
                            signed_state,
                            content: Content::Medium(bytes),
                        }
                    }
                    IntegerDiscriminant::Large => {
                        let mut bytes = [0_u8; 8];
                        for (i, b) in read_bytes.iter().copied().enumerate() {
                            bytes[i] = b;
                        }
                        Self {
                            signed_state,
                            content: Content::Large(bytes),
                        }
                    }
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{types::integer::Integer, utilities::cursor::Cursor, version::Version};
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
        fn back_to_original (i in any::<i64>(), v in any::<u8>().prop_map(|_n| Version::V0_1_0)) {
            let s = i.to_string();

            let parsed = Integer::from_str(&s).unwrap();

            let sered = parsed.ser(v);
            let got_back = Integer::deser(&mut Cursor::new(&sered), v).unwrap();
            prop_assert_eq!(parsed, got_back);

            prop_assert_eq!(i64::try_from(got_back).unwrap(), i);
        }
    }
}
