use alloc::vec::Vec;
use core::fmt::{Display, Formatter};

#[derive(Clone, Debug, Default)]
pub struct Bits {
    backing: Vec<u8>,
    valid_bits: usize,
}

impl PartialEq for Bits {
    fn eq(&self, other: &Self) -> bool {
        self.backing[0..self.valid_bits] == other.backing[0..other.valid_bits]
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

    pub fn len(&self) -> usize {
        self.valid_bits
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
impl From<Vec<bool>> for Bits {
    fn from(value: Vec<bool>) -> Self {
        let mut bits = Bits::default();

        for bool in value {
            bits.push(bool);
        }

        bits
    }
}

impl Display for Bits {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
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
