use crate::{
    types::integer::{Integer, IntegerSerError, SignedState},
    utilities::cursor::Cursor,
};
use core::fmt::{Display, Formatter};
use alloc::vec::Vec;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Imaginary {
    pub real: Integer,
    pub imaginary: Integer,
}

impl Display for Imaginary {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        if self.imaginary.is_negative() {
            write!(f, "{}{}i", self.real, self.imaginary)
        } else {
            write!(f, "{}+{}i", self.real, self.imaginary)
        }
    }
}

impl Imaginary {
    #[must_use]
    pub fn ser(&self) -> (SignedState, SignedState, Vec<u8>) {
        let (re_ss, mut re_bytes) = self.real.ser();
        let (im_ss, im_bytes) = self.imaginary.ser();

        re_bytes.extend(im_bytes.iter());

        (re_ss, im_ss, re_bytes)
    }

    pub fn deser(
        real_signed_state: SignedState,
        imaginary_signed_state: SignedState,
        bytes: &mut Cursor<u8>,
    ) -> Result<Self, IntegerSerError> {
        let real = Integer::deser(real_signed_state, bytes)?;
        let imaginary = Integer::deser(imaginary_signed_state, bytes)?;

        Ok(Self { real, imaginary })
    }
}
