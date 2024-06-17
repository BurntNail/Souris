use alloc::{string::ToString, vec, vec::Vec};
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
        // let self_end = (self.valid_bits as f32 / 8.0).ceil() as usize;
        // let other_end = (other.valid_bits as f32 / 8.0).ceil() as usize;
        //
        // self.backing[0..self_end] == other.backing[0..other_end]
        self.to_string().eq(&other.to_string()) //TODO: make this more efficient
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
        // let end = (self.valid_bits as f32 / 8.0).ceil() as usize;
        //
        // self.backing[0..end].hash(state);
        self.to_string().hash(state); //TODO: make more efficient
    }
}

impl Bits {
    pub fn push(&mut self, bit: bool) {
        if self.valid_bits % 8 == 0 {
            self.valid_bits += 1;
            self.backing.push(u8::from(bit));
        } else {
            let interior_index = self.valid_bits % 8;
            let backing_index = self.valid_bits / 8;
            self.valid_bits += 1;
            self.backing[backing_index] |= (u8::from(bit)) << interior_index;
        }
    }

    pub fn push_many(&mut self, bits: Self) {
        let bools: Vec<bool> = bits.into();
        for bool in bools {
            self.push(bool);
        }
    }

    pub fn pop(&mut self) -> Option<bool> {
        if self.valid_bits == 0 {
            return None;
        }

        let interior_index = (self.valid_bits - 1) % 8;

        if interior_index == 0 {
            self.valid_bits -= 1;
            Some(self.backing.pop().unwrap() > 0)
        } else {
            let backing_index = (self.valid_bits - 1) / 8;
            let extracted = self.backing[backing_index] & (1 << interior_index);
            self.backing[backing_index] &= u8::MAX - (1 << interior_index);
            self.valid_bits -= 1;
            Some(extracted > 0)
        }
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
        size.extend(&self.backing);

        size
    }

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
    use alloc::string::ToString;

    use crate::utilities::bits::Bits;

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
        let mut bits = Bits::default();
        bits.push(false);
        bits.push(true);
        bits.push(true);
        bits.push(false);
        bits.push(false);
        bits.push(true);

        assert_eq!(bits.pop(), Some(true));
        assert_eq!(bits.pop(), Some(false));
        assert_eq!(bits.pop(), Some(false));
        assert_eq!(bits.pop(), Some(true));
        assert_eq!(bits.pop(), Some(true));
        assert_eq!(bits.pop(), Some(false));
        assert_eq!(bits.pop(), None);
    }
}
