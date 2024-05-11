use crate::utilities::cursor::Cursor;
use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};
use core::{
    fmt::{Debug, Display, Formatter},
    num::ParseIntError,
    ops::{Add, Div, Mul, Sub},
    str::FromStr,
};
use num_traits::{Bounded, ConstOne, ConstZero, NumCast, One, ToPrimitive, Zero};

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum SignedState {
    Positive,
    Negative,
}

///size of the backing integer
pub type BiggestInt = u128;
pub type BiggestIntButSigned = i128; //convenience so it's all at the top of the file
///# of bytes for storing one `BiggestInt`
const INTEGER_MAX_SIZE: usize = (BiggestInt::BITS / 8) as usize; //yes, I could >> 3, but it gets compile-time evaluated and this is clearer
///max size for an integer to be stored by itself
const ONE_BYTE_MAX_SIZE: u8 = u8::MAX - (INTEGER_MAX_SIZE as u8);

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Integer {
    ///whether the number is negative
    signed_state: SignedState,
    ///positive little endian bytes
    content: [u8; INTEGER_MAX_SIZE],
}

impl Integer {
    fn unsigned_bits(&self) -> u32 {
        let x = BiggestInt::from_le_bytes(self.content);
        if x == 0 {
            0
        } else {
            x.ilog2()
        }
    }

    ///NB: always <= `INTEGER_MAX_SIZE`
    fn min_bytes_needed(&self) -> usize {
        ((self.unsigned_bits() / 8) + 1) as usize
    }

    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.signed_state == SignedState::Negative
    }

    #[must_use]
    pub fn is_positive(&self) -> bool {
        !self.is_negative()
    }
}

impl Display for Integer {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self.signed_state {
            SignedState::Negative => {
                write!(f, "{}", -BiggestIntButSigned::from_le_bytes(self.content))
            }
            SignedState::Positive => {
                write!(f, "{}", BiggestInt::from_le_bytes(self.content))
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
    ($($t:ty => $name:ident),+) => {
        $(
        impl Integer {
            #[must_use]
            pub fn $name(n: $t) -> Self {
                <Self as From<$t>>::from(n)
            }
        }
        )+
    };
}

macro_rules! from_signed {
    ($($t:ty),+) => {
        $(
        impl From<$t> for Integer {
            fn from(n: $t) -> Self {
                if n == 0 {
                    Self {
                        signed_state: SignedState::Positive,
                        content: [0; INTEGER_MAX_SIZE],
                    }
                } else if n < 0 {
                    let mut content = [0_u8; INTEGER_MAX_SIZE];
                    for (i, b) in (-n).to_le_bytes().into_iter().enumerate() {
                        content[i] = b;
                    }
                    Self {
                        signed_state: SignedState::Negative,
                        content,
                    }
                } else {
                    let mut content = [0_u8; INTEGER_MAX_SIZE];
                    for (i, b) in n.to_le_bytes().into_iter().enumerate() {
                        content[i] = b;
                    }
                    Self {
                        signed_state: SignedState::Positive,
                        content,
                    }
                }
            }
        }

        impl TryFrom<Integer> for $t {
            type Error = IntegerSerError;

            fn try_from(i: Integer) -> Result<Self, Self::Error> {
                let multiplier = match i.signed_state {
                    SignedState::Negative => -1,
                    _ => 1,
                };

                if i.unsigned_bits() > <$t>::BITS {
                    return Err(IntegerSerError::WrongType);
                }

                let mut out = [0_u8; (<$t>::BITS / 8) as usize];
                for (i, b) in i
                    .content
                    .into_iter()
                    .enumerate()
                    .take((<$t>::BITS / 8) as usize)
                {
                    out[i] = b;
                }

                Ok(<$t>::from_le_bytes(out) * multiplier)
            }
        }
        )+
    };
}
macro_rules! from_unsigned {
    ($($t:ty),+) => {
        $(
        impl From<$t> for Integer {
            fn from(n: $t) -> Self {
                let mut content = [0_u8; INTEGER_MAX_SIZE];
                for (i, b) in n.to_le_bytes().into_iter().enumerate() {
                    content[i] = b;
                }
                Self {
                    signed_state: SignedState::Positive,
                    content,
                }
            }
        }
        impl TryFrom<Integer> for $t {
            type Error = IntegerSerError;

            fn try_from(i: Integer) -> Result<Self, Self::Error> {
                if i.signed_state == SignedState::Negative {
                    return Err(IntegerSerError::WrongType);
                }

                if i.unsigned_bits() > <$t>::BITS {
                    return Err(IntegerSerError::WrongType);
                }

                let mut out = [0_u8; (<$t>::BITS / 8) as usize];
                for (i, b) in i
                    .content
                    .into_iter()
                    .enumerate()
                    .take((<$t>::BITS / 8) as usize)
                {
                    out[i] = b;
                }

                Ok(<$t>::from_le_bytes(out))
            }
        }
        )+
    };
}

new_x!(u8 => u8, i8 => i8, u16 => u16, i16 => i16, u32 => u32, i32 => i32, usize => usize, isize => isize, u64 => u64, i64 => i64, u128 => u128, i128 => i128);

from_signed!(i8, i16, i32, i64, isize, i128);
from_unsigned!(u8, u16, u32, u64, usize, u128);

macro_rules! integer_trait_impl {
    ($t:ident, $f:ident) => {
        impl $t<Self> for Integer {
            type Output = Self;

            fn $f(self, rhs: Self) -> Self::Output {
                let ss_to_use = match (self.signed_state, rhs.signed_state) {
                    (SignedState::Positive, SignedState::Positive) => SignedState::Positive,
                    _ => SignedState::Negative,
                };

                match ss_to_use {
                    SignedState::Positive => {
                        let Ok(lhs) = BiggestInt::try_from(self) else {
                            panic!("integer too big to fit into u128")
                        };
                        let Ok(rhs) = BiggestInt::try_from(rhs) else {
                            panic!("integer too big to fit into u128")
                        };

                        <Self as From<BiggestInt>>::from($t::$f(lhs, rhs))
                    }
                    SignedState::Negative => {
                        let Ok(lhs) = BiggestIntButSigned::try_from(self) else {
                            panic!("integer too big to fit into i128")
                        };
                        let Ok(rhs) = BiggestIntButSigned::try_from(rhs) else {
                            panic!("integer too big to fit into i128")
                        };

                        <Self as From<BiggestIntButSigned>>::from($t::$f(lhs, rhs))
                    }
                }
            }
        }
    };
}
integer_trait_impl!(Add, add);
integer_trait_impl!(Sub, sub);
integer_trait_impl!(Mul, mul);
integer_trait_impl!(Div, div);

impl Bounded for Integer {
    fn min_value() -> Self {
        <Integer as From<BiggestIntButSigned>>::from(BiggestIntButSigned::MIN)
    }

    fn max_value() -> Self {
        <Integer as From<BiggestInt>>::from(BiggestInt::MAX)
    }
}
impl ToPrimitive for Integer {
    fn to_i64(&self) -> Option<i64> {
        (*self).try_into().ok()
    }

    fn to_i128(&self) -> Option<i128> {
        (*self).try_into().ok()
    }

    fn to_u64(&self) -> Option<u64> {
        (*self).try_into().ok()
    }
    fn to_u128(&self) -> Option<u128> {
        (*self).try_into().ok()
    }
}
impl NumCast for Integer {
    #[allow(clippy::manual_map)]
    fn from<T: ToPrimitive>(n: T) -> Option<Self> {
        if let Some(i) = n.to_i128() {
            Some(<Self as From<BiggestIntButSigned>>::from(i))
        } else if let Some(u) = n.to_u128() {
            Some(<Self as From<BiggestInt>>::from(u))
        } else {
            None
        }
    }
}

impl One for Integer {
    fn one() -> Self {
        1_u128.into()
    }
}

impl ConstOne for Integer {
    const ONE: Self = Self {
        signed_state: SignedState::Positive,
        content: 1_u128.to_le_bytes(),
    };
}

impl Zero for Integer {
    fn zero() -> Self {
        0_u128.into()
    }

    fn is_zero(&self) -> bool {
        self.content.iter().all(|x| *x == 0)
    }
}

impl ConstZero for Integer {
    const ZERO: Self = Self {
        signed_state: SignedState::Positive,
        content: [0; (BiggestInt::BITS / 8) as usize],
    };
}

#[cfg(feature = "serde")]
impl serde::Serialize for Integer {
    fn serialize<S>(&self, serialiser: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = *self;
        if self.signed_state == SignedState::Positive {
            serialiser.serialize_u128(s.try_into().map_err(serde::ser::Error::custom)?)
        } else {
            serialiser.serialize_i128(s.try_into().map_err(serde::ser::Error::custom)?)
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Integer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        struct IntegerVisitor;

        impl<'de> serde::de::Visitor<'de> for IntegerVisitor {
            type Value = Integer;

            fn expecting(&self, f: &mut Formatter) -> core::fmt::Result {
                write!(
                    f,
                    "An integer between {} and {}",
                    BiggestInt::MAX,
                    BiggestIntButSigned::MIN
                )
            }

            fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(<Integer as From<i8>>::from(v))
            }
            fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(<Integer as From<i16>>::from(v))
            }
            fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(<Integer as From<i32>>::from(v))
            }
            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(<Integer as From<i64>>::from(v))
            }
            fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(<Integer as From<i128>>::from(v))
            }

            fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(<Integer as From<u8>>::from(v))
            }
            fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(<Integer as From<u16>>::from(v))
            }
            fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(<Integer as From<u32>>::from(v))
            }
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(<Integer as From<u64>>::from(v))
            }
            fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(<Integer as From<u128>>::from(v))
            }
        }

        deserializer.deserialize_any(IntegerVisitor)
    }
}

impl FromStr for Integer {
    type Err = IntegerSerError;

    #[allow(clippy::cast_possible_truncation)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(IntegerSerError::NotEnoughBytes);
        };

        if s == "0" {
            return Ok(Self {
                signed_state: SignedState::Positive,
                content: [0; INTEGER_MAX_SIZE],
            });
        }

        let (s, signed_state) = if s.as_bytes()[0] == b'-' {
            (&s[1..], SignedState::Negative)
        } else {
            (s, SignedState::Positive)
        };

        let content: BiggestInt = s.parse()?;

        Ok(Self {
            signed_state,
            content: content.to_le_bytes(),
        })
    }
}

impl From<SignedState> for u8 {
    fn from(value: SignedState) -> Self {
        match value {
            SignedState::Positive => 0b0,
            SignedState::Negative => 0b1,
        }
    }
}
impl TryFrom<u8> for SignedState {
    type Error = IntegerSerError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0b0 => Ok(Self::Positive),
            0b1 => Ok(Self::Negative),
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
    SerdeCustom(String),
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
                write!(f, "Error parsing from base-10 string: {e}")
            }
            IntegerSerError::SerdeCustom(s) => write!(f, "Error in serde: {s}"),
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
    pub fn ser(self) -> (SignedState, Vec<u8>) {
        if let Some(smol) = self.to_i8() {
            return if smol < 0 {
                (SignedState::Negative, vec![(-smol) as u8])
            } else {
                (SignedState::Positive, vec![smol as u8])
            };
        } else if let Some(pos_smol) = self.to_u8() {
            if pos_smol < ONE_BYTE_MAX_SIZE {
                return (SignedState::Positive, vec![pos_smol]);
            }
        }

        let stored_size = self.min_bytes_needed();
        let bytes = self.content;

        let discriminant = ONE_BYTE_MAX_SIZE + stored_size as u8;

        let mut res = vec![];
        res.push(discriminant);
        res.extend(&bytes[0..stored_size]);

        (self.signed_state, res)
    }

    pub fn deser(
        signed_state: SignedState,
        reader: &mut Cursor<u8>,
    ) -> Result<Self, IntegerSerError> {
        let Some(first_byte) = reader.next().copied() else {
            return Err(IntegerSerError::NotEnoughBytes);
        };

        if first_byte <= ONE_BYTE_MAX_SIZE {
            let mut content = [0; INTEGER_MAX_SIZE];
            content[0] = first_byte;

            return Ok(Self {
                signed_state,
                content,
            });
        }

        let bytes_stored = first_byte - ONE_BYTE_MAX_SIZE;
        let Some(bytes_stored) = reader.read(bytes_stored as usize) else {
            return Err(IntegerSerError::NotEnoughBytes);
        };

        let mut content = [0; INTEGER_MAX_SIZE];
        for (i, b) in bytes_stored.iter().copied().enumerate() {
            content[i] = b;
        }

        Ok(Self {
            signed_state,
            content,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        types::integer::{BiggestInt, BiggestIntButSigned, Integer},
        utilities::cursor::Cursor,
    };
    use alloc::{format, string::ToString};
    use core::str::FromStr;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn doesnt_crash (s in "\\PC*") {
            let _ = Integer::from_str(&s);
        }

        #[test]
        fn parse_valids (i in any::<i32>()) {
            let int = Integer::from_str(&i.to_string()).unwrap();
            prop_assert_eq!(i32::try_from(int).unwrap(), i);
        }

        #[test]
        fn back_to_original (i in any::<BiggestIntButSigned>()) {
            let s = i.to_string();

            let parsed = Integer::from_str(&s).unwrap();

            let (s, sered) = parsed.ser();
            let got_back = Integer::deser(s, &mut Cursor::new(&sered)).unwrap();
            prop_assert_eq!(parsed, got_back);

            prop_assert_eq!(BiggestIntButSigned::try_from(got_back).unwrap(), i);
        }

        #[test]
        fn back_to_original_other_size (i in any::<u8>()) {
            let s = i.to_string();

            let parsed = Integer::from_str(&s).unwrap();

            let (s, sered) = parsed.ser();
            let got_back = Integer::deser(s, &mut Cursor::new(&sered)).unwrap();
            prop_assert_eq!(parsed, got_back);

            prop_assert_eq!(u32::try_from(got_back).unwrap(), u32::from(i));
        }

        #[test]
        #[cfg(feature = "serde")]
        fn serde_works_signed (raw_i in any::<BiggestIntButSigned>()) {
            let i = Integer::from(raw_i);
            let from_raw = i.to_string();

            let to_serde = serde_json::to_string(&i).unwrap();
            let from_serde = match serde_json::from_str(&to_serde) {
                Ok(f) => f,
                Err(e) => {
                    let e = e.to_string();
                    return if e.contains("invalid type") { //dealt with in Value impl
                        Ok(())
                    } else {
                        panic!("{e:?}");
                    };
                }
            };

            eprintln!("{i:?} {from_serde:?}");

            prop_assert_eq!(from_raw, to_serde);
            prop_assert_eq!(i, from_serde);
        }

        #[test]
        #[cfg(feature = "serde")]
        fn serde_works_unsigned (i in any::<BiggestInt>()) {
            let i = Integer::from(i);
            let from_raw = i.to_string();

            let to_serde = serde_json::to_string(&i).unwrap();
            let from_serde = match serde_json::from_str(&to_serde) {
                Ok(f) => f,
                Err(e) => {
                    let e = e.to_string();
                    return if e.contains("invalid type") { //floats dealt with in Value impl
                        Ok(())
                    } else {
                        panic!("{e:?}")
                    };
                }
            };

            prop_assert_eq!(from_raw, to_serde);
            prop_assert_eq!(i, from_serde);
        }
    }
}
