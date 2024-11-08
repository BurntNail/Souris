use alloc::{
    string::ToString,
    vec,
    vec::{IntoIter, Vec},
};
use core::{
    fmt::{Debug, Display, Formatter},
    hash::{Hash, Hasher},
    ops::Index,
};

use crate::{
    types::integer::{Integer, IntegerSerError, SignedState},
    utilities::cursor::Cursor,
};

#[derive(Clone, Default)]
pub struct Bits {
    backing: Vec<u8>,
    valid_bits: usize,
}

impl Debug for Bits {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Bits")
            .field("backing", &self.backing)
            .field("valid_bits", &self.valid_bits)
            .field("binary", &self.to_string())
            .finish()
    }
}

impl PartialEq for Bits {
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss
    )]
    fn eq(&self, other: &Self) -> bool {
        self.valid_bits == other.valid_bits && self.get_proper_bytes().eq(&other.get_proper_bytes())
    }
}

impl Eq for Bits {}

impl Hash for Bits {
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss
    )]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.get_proper_bytes().hash(state);
    }
}

impl Bits {
    pub fn push(&mut self, bit: bool) {
        if self.valid_bits % 8 == 0 {
            self.backing.push(0);
        }

        let interior_index = self.valid_bits % 8;
        let backing_index = self.valid_bits / 8;
        self.backing[backing_index] |= (u8::from(bit)) << interior_index;

        self.valid_bits += 1;
    }

    #[must_use]
    pub fn from_binary(backing: Vec<u8>) -> Self {
        Self {
            valid_bits: backing.len() * 8,
            backing,
        }
    }

    #[inline]
    #[must_use]
    pub fn push_into_new(&self, bit: bool) -> Self {
        let mut new = self.clone();
        new.push(bit);
        new
    }

    pub fn push_many(&mut self, bits: Self) {
        let bools: Vec<bool> = bits.into();
        self.backing.reserve(bools.len() / 8);
        for bool in bools {
            self.push(bool);
        }
    }

    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn pop(&mut self) -> Option<bool> {
        if self.valid_bits == 0 {
            return None;
        }

        self.valid_bits -= 1;
        let interior_index = self.valid_bits % 8;

        if interior_index == 0 {
            Some(self.backing.pop().unwrap() > 0)
        } else {
            let backing_index = self.valid_bits / 8;
            let extracted = self.backing[backing_index] & (1 << interior_index);
            self.backing[backing_index] &= u8::MAX - (1 << interior_index);
            Some(extracted > 0)
        }
    }

    #[must_use]
    pub fn get_proper_bytes(&self) -> Vec<u8> {
        let interior_index = self.valid_bits % 8;
        if interior_index == 0 {
            return self.backing.clone();
        }

        let to_be_anded = (0..=interior_index).fold(0, |acc, i| acc | (1 << i));

        let mut bytes = self.backing.clone();
        bytes[self.backing.len() - 1] &= to_be_anded;

        bytes
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.valid_bits
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.valid_bits == 0
    }

    pub fn clear(&mut self) {
        self.valid_bits = 0;
        self.backing.clear();
    }

    #[must_use]
    pub fn ser(&self) -> Vec<u8> {
        let (_, mut size) = Integer::usize(self.valid_bits).ser();
        size.extend(self.get_proper_bytes());

        size
    }

    ///Deserialises bytes into bits
    ///
    /// # Errors
    /// - [`IntegerSerError`] if we cannot find the number of valid bits
    /// - [`IntegerSerError::NotEnoughBytes`] if we do not have enough bytes
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn deser(bytes: &mut Cursor<u8>) -> Result<Self, IntegerSerError> {
        let valid_bits: usize = Integer::deser(SignedState::Unsigned, bytes)?.try_into()?;
        let to_be_read = (valid_bits as f32 / 8.0).ceil() as usize;
        let Some(backing) = bytes.read(to_be_read).map(<[u8]>::to_vec) else {
            return Err(IntegerSerError::NotEnoughBytes);
        };

        Ok(Self {
            backing,
            valid_bits,
        })
    }
}

impl From<Bits> for Vec<bool> {
    fn from(mut value: Bits) -> Self {
        let mut v = vec![];

        while let Some(b) = value.pop() {
            v.push(b);
        }

        v.reverse();
        v
    }
}

impl<T: AsRef<[bool]>> From<T> for Bits {
    fn from(value: T) -> Self {
        let mut bits = Bits::default();

        for bool in value.as_ref() {
            bits.push(*bool);
        }

        bits
    }
}

impl FromIterator<bool> for Bits {
    fn from_iter<T: IntoIterator<Item = bool>>(iter: T) -> Self {
        let mut bits = Bits::default();

        for bit in iter {
            bits.push(bit);
        }

        bits
    }
}
impl FromIterator<Bits> for Bits {
    fn from_iter<T: IntoIterator<Item = Bits>>(iter: T) -> Self {
        let mut out_bits = Bits::default();

        for bits in iter {
            out_bits.push_many(bits);
        }

        out_bits
    }
}

impl IntoIterator for Bits {
    type Item = bool;
    type IntoIter = IntoIter<bool>;

    fn into_iter(self) -> Self::IntoIter {
        let v: Vec<bool> = self.into();
        v.into_iter()
    }
}

impl Display for Bits {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        if self.valid_bits == 0 {
            return write!(f, "");
        }

        for i in 0..(self.valid_bits - 1) {
            let interior_index = i % 8;
            let backing_index = i / 8;

            if (self.backing[backing_index] & (1 << interior_index)) > 0 {
                write!(f, "1")?;
            } else {
                write!(f, "0")?;
            }
        }

        let interior_index = (self.valid_bits - 1) % 8;
        let backing_index = (self.valid_bits - 1) / 8;

        if (self.backing[backing_index] & (1 << interior_index)) > 0 {
            write!(f, "1")
        } else {
            write!(f, "0")
        }
    }
}

impl<T: Into<usize>> Index<T> for Bits {
    type Output = bool;

    fn index(&self, index: T) -> &<Self as Index<T>>::Output {
        static TRUE: bool = true;
        static FALSE: bool = false;

        let index = index.into();
        assert!(
            index < self.valid_bits,
            "attempted to get index {index} into bits length {}",
            self.valid_bits
        );

        let interior_index = index % 8;
        let backing_index = index / 8;

        if self.backing[backing_index] & (1 << interior_index) > 0 {
            &TRUE
        } else {
            &FALSE
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::utilities::bits::Bits;
    use alloc::{format, string::ToString};
    #[allow(unused_imports)]
    use proptest::{prop_assert, prop_assert_eq, prop_assert_ne};

    #[test]
    fn test_display() {
        let mut bits = Bits::default();
        bits.push(false);
        bits.push(true);
        bits.push(true);
        bits.push(false);
        bits.push(false);
        bits.push(true);

        let disp = bits.to_string();
        assert_eq!(disp, "011001");
    }

    #[test]
    fn test_pop() {
        let mut bits: Bits = Bits::default();
        bits.push(false);
        bits.push(true);
        bits.push(true);
        bits.push(false);
        bits.push(false);
        bits.push(true);
        bits.push(true);
        bits.push(false);
        bits.push(true);

        assert_eq!(bits.pop(), Some(true));
        assert_eq!(bits.pop(), Some(false));
        assert_eq!(bits.pop(), Some(true));

        bits.push(false);
        bits.push(false);
        bits.push(false);
        bits.push(true);
        bits.push(false);
        assert_eq!(bits.pop(), Some(false));
        assert_eq!(bits.pop(), Some(true));
        assert_eq!(bits.pop(), Some(false));
        assert_eq!(bits.pop(), Some(false));
        assert_eq!(bits.pop(), Some(false));

        assert_eq!(bits.pop(), Some(true));
        assert_eq!(bits.pop(), Some(false));
        assert_eq!(bits.pop(), Some(false));
        assert_eq!(bits.pop(), Some(true));
        assert_eq!(bits.pop(), Some(true));
        assert_eq!(bits.pop(), Some(false));
        assert_eq!(bits.pop(), None);
    }

    proptest::proptest! {
        #[test]
        fn test_partialeq (a: u32, b: u32, a_bits in 0..=32_usize, b_bits in 0..=32_usize) {
            let a_bytes = a.to_le_bytes().to_vec();
            let a_bits = Bits {
                backing: a_bytes,
                valid_bits: a_bits
            };

            let b_bytes = b.to_le_bytes().to_vec();
            let b_bits = Bits {
                backing: b_bytes,
                valid_bits: b_bits
            };

            if a == b {
                prop_assert!(a_bits.eq(&b_bits));
            } else {
                prop_assert!(a_bits.ne(&b_bits));
            }
        }

        #[cfg(feature = "std")]
        #[test]
        fn test_hash (a: u8, b: u8, bits in 0..=8_usize) {
            let a_bytes = a.to_le_bytes().to_vec();
            let a_bits = Bits {
                backing: a_bytes,
                valid_bits: bits
            };

            let b_bytes = b.to_le_bytes().to_vec();
            let b_bits = Bits {
                backing: b_bytes,
                valid_bits: bits
            };


            let mut a_hasher = std::hash::DefaultHasher::new(); //going with long names to make it easier with features etc
            let mut b_hasher = std::hash::DefaultHasher::new();

            std::hash::Hash::hash(&a_bits, &mut a_hasher);
            std::hash::Hash::hash(&b_bits, &mut b_hasher);

            let a_hash = std::hash::Hasher::finish(&a_hasher);
            let b_hash = std::hash::Hasher::finish(&b_hasher);

            if a_bits == b_bits {
                prop_assert_eq!(a_hash, b_hash);
            } else {
                prop_assert_ne!(a_hash, b_hash);
            }
        }
    }
}
