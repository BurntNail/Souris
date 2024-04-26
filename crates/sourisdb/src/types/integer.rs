use crate::utilities::cursor::Cursor;
use alloc::{string::ToString, vec::Vec};
use core::{
    fmt::{Debug, Display, Formatter},
    num::ParseIntError,
    str::FromStr,
};

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum SignedState {
    Positive,
    Negative
}

const SIGNED_BITS: usize = 1;

///size of the backing integer
type BiggestInt = u64;
type BiggestIntButSigned = i64; //convenience so it's all at the top of the file
///# of bytes for storing one `BiggestInt`
const INTEGER_MAX_SIZE: usize = (BiggestInt::BITS / 8) as usize; //yes, I could >> 3, but it gets compile-time evaluated and this is clearer
///# of bits to store a number from 0 to `INTEGER_MAX_SIZE` in the discriminant
const INTEGER_DISCRIMINANT_BITS: usize = INTEGER_MAX_SIZE.ilog2() as usize + 1;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Integer {
    signed_state: SignedState,
    content: [u8; INTEGER_MAX_SIZE],
}

#[cfg(feature = "serde")]
impl serde::Serialize for Integer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        let s = *self;
        if self.signed_state == SignedState::Positive { //yipee i sure do love repetitive code
            match self.min_bytes_needed() {
                0..=1 => {
                    let Ok(n) = s.try_into() else {
                        unreachable!("cannot reach here as already checked # bytes")
                    };
                    serializer.serialize_u8(n)
                }
                2 => {
                    let Ok(n) = s.try_into() else {
                        unreachable!("cannot reach here as already checked # bytes")
                    };
                    serializer.serialize_u16(n)
                }
                3..=4 => {
                    let Ok(n) = s.try_into() else {
                        unreachable!("cannot reach here as already checked # bytes")
                    };
                    serializer.serialize_u32(n)
                }
                5..=8 => {
                    let Ok(n) = s.try_into() else {
                        unreachable!("cannot reach here as already checked # bytes")
                    };
                    serializer.serialize_u64(n)
                }
                // 9..=16 => {
                //     let Ok(n) = s.try_into() else {
                //         unreachable!("cannot reach here as already checked # bytes")
                //     };
                //     serializer.serialize_u128(n)
                // }
                _ => unreachable!("can't need to store > 16 bytes for serde")
            }
        } else {
            match self.min_bytes_needed() {
                0..=1 => {
                    let Ok(n) = s.try_into() else {
                        unreachable!("cannot reach here as already checked # bytes")
                    };
                    serializer.serialize_i8(n)
                }
                2 => {
                    let Ok(n) = s.try_into() else {
                        unreachable!("cannot reach here as already checked # bytes")
                    };
                    serializer.serialize_i16(n)
                }
                3..=4 => {
                    let Ok(n) = s.try_into() else {
                        unreachable!("cannot reach here as already checked # bytes")
                    };
                    serializer.serialize_i32(n)
                }
                5..=8 => {
                    let Ok(n) = s.try_into() else {
                        unreachable!("cannot reach here as already checked # bytes")
                    };
                    serializer.serialize_i64(n)
                }
                // 9..=16 => {
                //     let Ok(n) = s.try_into() else {
                //         unreachable!("cannot reach here as already checked # bytes")
                //     };
                //     serializer.serialize_i128(n)
                // }
                _ => unreachable!("can't need to store > 16 bytes for serde")
            }
        }
    }
}

#[cfg(feature = "serde")]
struct IntegerVisitor;

#[cfg(feature = "serde")]
impl<'de> serde::de::Visitor<'de> for IntegerVisitor { //yipeeeeeeeeeeee
    type Value = Integer;
    fn expecting(&self, f: &mut Formatter) -> core::fmt::Result {
        write!(f, "an integer value {} to {}", i64::MIN, u64::MAX)
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E> where E: serde::de::Error {
        Ok(Integer::from(v))
    }
    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E> where E: serde::de::Error {
        Ok(Integer::from(v))
    }
    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E> where E: serde::de::Error {
        Ok(Integer::from(v))
    }
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> where E: serde::de::Error {
        Ok(Integer::from(v))
    }
    fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E> where E: serde::de::Error {
        // Ok(Integer::from(v))

        if v > 0 && v < BiggestInt::MAX as i128 {
            Ok(Integer::from(v as BiggestInt))
        } else if v < 0 && v > BiggestIntButSigned::MIN as i128 {
            Ok(Integer::from(v as BiggestIntButSigned))
        } else {
            Err(serde::de::Error::custom(format!("Integer {v} too big to store")))
        }
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E> where E: serde::de::Error {
        Ok(Integer::from(v))
    }
    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E> where E: serde::de::Error {
        Ok(Integer::from(v))
    }
    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E> where E: serde::de::Error {
        Ok(Integer::from(v))
    }
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> where E: serde::de::Error {
        Ok(Integer::from(v))
    }
    // fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E> where E: serde::de::Error {
    //     Ok(Integer::from(v))
    // }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Integer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
        deserializer.deserialize_i128(IntegerVisitor)
    }
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

    ///NB: always <= INTEGER_MAX_SIZE
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
            _ => {
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
    ($t:ty => $name:ident) => {
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
    };
    ($t:ty =>> $name:ident) => {
        impl Integer {
            #[must_use]
            pub fn $name(n: $t) -> Self {
                Self::from(n)
            }
        }

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
    };
}

new_x!(u8 => u8);
new_x!(i8 =>> i8);
new_x!(u16 => u16);
new_x!(i16 =>> i16);
new_x!(u32 => u32);
new_x!(i32 =>> i32);
new_x!(usize => usize);
new_x!(isize =>> isize);
new_x!(u64 => u64);
new_x!(i64 =>> i64);
// new_x!(u128 => u128);
// new_x!(i128 =>> i128);

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
            SignedState::Negative => 0b1
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
    SerdeCustom(String)
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
            IntegerSerError::SerdeCustom(s) => write!(f, "Error in serde: {s}"),
        }
    }
}

#[cfg(feature = "serde")]
impl serde::de::Error for IntegerSerError {
    fn custom<T>(msg: T) -> Self where T: Display {
        Self::SerdeCustom(msg.to_string())
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
        let stored_size = self.min_bytes_needed();
        let bytes = self.content;

        let mut res = Vec::with_capacity(1 + stored_size);
        let stored_size_disc = (stored_size as u8) << (8 - (INTEGER_DISCRIMINANT_BITS + SIGNED_BITS));
        let signed_state_disc = u8::from(self.signed_state) << (8 - SIGNED_BITS);

        let discriminant: u8 = signed_state_disc | stored_size_disc;
        res.push(discriminant);
        res.extend(&bytes[0..stored_size]);

        res
    }

    pub fn deser(reader: &mut Cursor<u8>) -> Result<Self, IntegerSerError> {
        const fn size_discriminant_mask() -> u8 {
            let base: u8 = (1 << INTEGER_DISCRIMINANT_BITS) - 1; //construct INTEGER_BITS bits at the end
            base << (8 - (INTEGER_DISCRIMINANT_BITS + SIGNED_BITS)) //move them forward until there's only INTEGER_BITS bits left at the front
        }
        const fn signed_discriminant_mask() -> u8 {
            let base: u8 = (1 << SIGNED_BITS) - 1; //construct INTEGER_BITS bits at the end
            base << (8 - SIGNED_BITS) //move them forward it's at the front
        }

        let (signed_state, stored) = {
            let [discriminant] = reader.read(1).ok_or(IntegerSerError::NotEnoughBytes)? else {
                unreachable!("didn't get just one byte back")
            };
            let discriminant = *discriminant;
            let signed_state = SignedState::try_from((discriminant & signed_discriminant_mask()) >> (8 - SIGNED_BITS))?;
            let stored = usize::from(
                (discriminant & size_discriminant_mask()) >> (8 - (INTEGER_DISCRIMINANT_BITS + SIGNED_BITS)),
            );

            #[allow(clippy::items_after_statements)]
            (signed_state, stored)
        };

        let mut content = [0_u8; INTEGER_MAX_SIZE];
        for (i, b) in reader
            .read(stored)
            .ok_or(IntegerSerError::NotEnoughBytes)?
            .iter()
            .copied()
            .enumerate()
        {
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
    use crate::{types::integer::Integer, utilities::cursor::Cursor};
    use alloc::{format, string::ToString};
    use core::str::FromStr;
    use proptest::prelude::*;
    use crate::types::integer::{BiggestInt, BiggestIntButSigned};

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

            let sered = parsed.ser();
            let got_back = Integer::deser(&mut Cursor::new(&sered)).unwrap();
            prop_assert_eq!(parsed, got_back);

            prop_assert_eq!(BiggestIntButSigned::try_from(got_back).unwrap(), i);
        }

        #[test]
        fn back_to_original_other_size (i in any::<u8>()) {
            let s = i.to_string();

            let parsed = Integer::from_str(&s).unwrap();

            let sered = parsed.ser();
            let got_back = Integer::deser(&mut Cursor::new(&sered)).unwrap();
            prop_assert_eq!(parsed, got_back);

            prop_assert_eq!(u32::try_from(got_back).unwrap(), u32::from(i));
        }

        // #[test]
        // fn serde_works_signed (i in any::<BiggestIntButSigned>()) {
        //     let i = Integer::from(i);
        //     let from_raw = i.to_string();
        //
        //     let to_serde = serde_json::to_string(&i).unwrap();
        //     let from_serde = serde_json::from_str(&to_serde).unwrap();
        //
        //     prop_assert_eq!(from_raw, to_serde);
        //     prop_assert_eq!(i, from_serde);
        // }

        #[test]
        fn serde_works_unsigned (i in any::<BiggestInt>()) {
            let i = Integer::from(i);
            let from_raw = i.to_string();

            let to_serde = serde_json::to_string(&i).unwrap();
            let from_serde = serde_json::from_str(&to_serde).unwrap();

            prop_assert_eq!(from_raw, to_serde);
            prop_assert_eq!(i, from_serde);
        }
    }
}
