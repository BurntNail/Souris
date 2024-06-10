//! A module containing a struct [`Integer`] designed to minimise size when serialised.

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

use serde_json::{Number, Value as SJValue};

use crate::{display_bytes_as_hex_array, utilities::cursor::Cursor};

///This represents whether a number is positive or negative. There are conversions to/from [`u8`]s where `1` represents negative and `0` represents positive.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum SignedState {
    #[allow(missing_docs)]
    Positive,
    #[allow(missing_docs)]
    Negative,
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

///The largest unsigned integer that can be stored using [`Integer`].
pub type BiggestInt = u128;
///The largest signed integer that can be stored using [`Integer`].
pub type BiggestIntButSigned = i128; //convenience so it's all at the top of the file
///The number of bytes required for storing one [`BiggestInt`]
const INTEGER_MAX_SIZE: usize = (BiggestInt::BITS / 8) as usize; //yes, I could >> 3, but it gets compile-time evaluated anyways and this is clearer
///The maximum size for an integer to be stored without a size before it
#[allow(clippy::cast_possible_truncation)]
pub const ONE_BYTE_MAX_SIZE: u8 = u8::MAX - (INTEGER_MAX_SIZE as u8);

///A type that represents an integer designed to be the smallest when serialised.
///
/// NB: the private `content` is always unsigned and the `signed_state` must be applied to get a valid integer out. The `TryFrom` implementations always do this.
///
/// To create an `Integer`, there are many `From` implementations for every integer type in the standard library. To get a type out, there are many `TryFrom` implementations for those same integers. These are `TryFrom` as the stored content could be too large or be have a sign and not be able to be represented by an unsigned integer.
///
/// When converting to a floating point number, precision can be lost. When converting from a floating number, it can fail if:
/// - The floating point number was too large.
/// - The floating point number had a decimal part (currently checked using [`f64::fract`], [`f64::EPSILON`] and the [`f32`] equivalents).
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Integer {
    ///whether the number is negative
    signed_state: SignedState,
    ///positive little endian bytes
    content: [u8; INTEGER_MAX_SIZE],
}

impl Integer {
    ///The minimum number of bits required to store this integer as an unsigned integer
    ///
    ///```rust
    /// use sourisdb::types::integer::Integer;
    ///
    /// let n: Integer = 257.into();
    /// let min_bits = n.unsigned_bits();
    /// assert_eq!(min_bits, 9);
    /// ```
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn unsigned_bits(&self) -> u32 {
        let x = f64::from(*self);
        if x == 0.0 {
            0
        } else {
            x.log2().ceil() as u32
        }
    }

    ///The minimum number of bytes required to store this as an unsigned integer
    ///
    /// NB: always <= `INTEGER_MAX_SIZE`
    fn min_bytes_needed(&self) -> usize {
        ((self.unsigned_bits() / 8) + 1) as usize
    }

    ///Whether the number is negative.
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.signed_state == SignedState::Negative
    }

    ///Whether the number is positive.
    #[must_use]
    pub fn is_positive(&self) -> bool {
        self.signed_state == SignedState::Positive
    }

    ///Converts the `Integer` to a [`serde_json::Value`].
    ///
    /// This can fail if the integer doesn't fit into i64 or u64 as those are the limits for [`Number`].
    #[must_use]
    pub fn to_json(self) -> Option<SJValue> {
        Some(if self.is_negative() {
            let n = i64::try_from(self).ok()?;
            SJValue::Number(Number::from(n))
        } else {
            let n = u64::try_from(self).ok()?;
            SJValue::Number(Number::from(n))
        })
    }

    ///Gets an `Integer` from a [`Number`].
    ///
    /// Can fail if the number was representing a floating point number.
    #[must_use]
    pub fn from_json(n: &Number) -> Option<Self> {
        if let Some(u) = n.as_u64() {
            Some(u.into())
        } else {
            n.as_i64().map(Into::into)
        }
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
        let ss = self.signed_state;
        let hex = display_bytes_as_hex_array(&self.content);
        let content = self.to_string();

        f.debug_struct("Integer")
            .field("signed_state", &ss)
            .field("content", &hex)
            .field("content", &content)
            .finish()
    }
}

macro_rules! new_x {
    ($($t:ty => $name:ident),+) => {
        impl Integer {
            $(
                ///Creates an `Integer`
                #[must_use]
                pub fn $name(n: $t) -> Self {
                    <Self as From<$t>>::from(n)
                }
            )+
        }
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
                if i.unsigned_bits() > (<$t>::BITS - 1) {
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

                let raw = <$t>::from_le_bytes(out);
                let out = if i.signed_state == SignedState::Negative {
                    -raw
                } else {
                    raw
                };

                Ok(out)
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

impl From<Integer> for f64 {
    #[allow(clippy::cast_precision_loss)]
    fn from(value: Integer) -> Self {
        match value.signed_state {
            SignedState::Positive => u128::from_le_bytes(value.content) as f64,
            SignedState::Negative => (-i128::from_le_bytes(value.content)) as f64,
        }
    }
}
impl From<Integer> for f32 {
    fn from(value: Integer) -> Self {
        match value.signed_state {
            SignedState::Positive => u128::from_le_bytes(value.content) as f32,
            SignedState::Negative => (-i128::from_le_bytes(value.content)) as f32,
        }
    }
}

#[derive(Debug, Copy, Clone)]
///An error enum to represent why a conversion from floating point number to integer failed.
pub enum FloatToIntegerConversionError {
    ///Integers cannot hold any decimal parts.
    DecimalsNotSupported,
    ///Integers can only hold positive numbers up within [`BiggestInt`] and negative numbers within [`BiggestIntButSigned`].
    TooLarge,
    ///Only finite numbers are supported for conversion into integers - there's no meaningful representation for `NaN` or infinite numbers.
    NotFinite,
}
impl Display for FloatToIntegerConversionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DecimalsNotSupported => write!(
                f,
                "Floating point decimals not supported for integer values"
            ),
            Self::TooLarge => write!(f, "Floating point number was too large"),
            Self::NotFinite => write!(f, "Floating point number was not finite"),
        }
    }
}
#[cfg(feature = "std")]
impl std::error::Error for FloatToIntegerConversionError {}

impl TryFrom<f64> for Integer {
    type Error = FloatToIntegerConversionError;

    #[allow(clippy::collapsible_else_if)]
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if !value.is_finite() {
            return Err(FloatToIntegerConversionError::NotFinite);
        }
        if value.fract() > f64::EPSILON {
            return Err(FloatToIntegerConversionError::DecimalsNotSupported);
        }

        let floored = value.floor();
        if floored < 0.0 {
            if floored > i128::MIN as f64 {
                Ok((floored as i128).into())
            } else {
                Err(FloatToIntegerConversionError::TooLarge)
            }
        } else {
            if floored < u128::MAX as f64 {
                Ok((floored as u128).into())
            } else {
                Err(FloatToIntegerConversionError::TooLarge)
            }
        }
    }
}
impl TryFrom<f32> for Integer {
    type Error = FloatToIntegerConversionError;

    #[allow(clippy::collapsible_else_if)]
    fn try_from(value: f32) -> Result<Self, Self::Error> {
        if value.fract() > f32::EPSILON {
            return Err(FloatToIntegerConversionError::DecimalsNotSupported);
        }

        let floored = value.floor();
        if floored < 0.0 {
            if floored > i128::MIN as f32 {
                Ok((floored as i128).into())
            } else {
                Err(FloatToIntegerConversionError::TooLarge)
            }
        } else {
            if floored > u128::MAX as f32 {
                Ok((floored as u128).into())
            } else {
                Err(FloatToIntegerConversionError::TooLarge)
            }
        }
    }
}

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

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
///Error type for dealing with serialisation errors related to [`Integer`]s.
pub enum IntegerSerError {
    ///An invalid signed state was found - these should only be `0b1` and `0b0`
    InvalidSignedStateDiscriminant(u8),
    ///Not enough bytes were within the cursor to deserialise the integer
    NotEnoughBytes,
    ///Integers can only be turned back into rust integers that they actually fit inside - this can be caused by sign errors (eg. fitting `-12` into a `usize`) or size concerns (eg. fitting `123456789101112131415` into a `u8`)
    WrongType,
    ///Error parsing an integer from a string using the standard library.
    IntegerParseError(ParseIntError),
    ///Custom Serde error for use serialising and deserialising with `serde`.
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
    ///Serialises an integer into a signed state and bytes.
    ///
    ///Follows the following logic:
    /// - Is the integer less than or equal to [`ONE_BYTE_MAX_SIZE`]. If it is, just return it in a byte vector.
    /// - Store the number of bytes required to hold the integer.
    /// - Store the bytes of the integer, skipping leading zero bytes
    #[must_use]
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    pub fn ser(self) -> (SignedState, Vec<u8>) {
        if let Ok(smol) = u8::try_from(self) {
            if smol < ONE_BYTE_MAX_SIZE {
                return (SignedState::Positive, vec![smol]);
            }
        } else if let Ok(smol) = i8::try_from(self) {
            return if smol < 0 {
                (SignedState::Negative, vec![(-smol) as u8])
            } else {
                (SignedState::Positive, vec![smol as u8])
            };
        }

        let stored_size = self.min_bytes_needed();
        let bytes = self.content;

        let discriminant = ONE_BYTE_MAX_SIZE + stored_size as u8;

        let mut res = vec![];
        res.push(discriminant);
        res.extend(&bytes[0..stored_size]);

        (self.signed_state, res)
    }

    ///Deserialise bytes inside a [`Cursor`] into an Integer.
    ///
    /// ## Errors
    /// - Can fail with [`IntegerSerError`] if there aren't enough bytes
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
    use alloc::{format, string::ToString};
    use core::str::FromStr;

    use proptest::prelude::*;

    use crate::{
        types::integer::{BiggestInt, BiggestIntButSigned, Integer},
        utilities::cursor::Cursor,
    };

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
