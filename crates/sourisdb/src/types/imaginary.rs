//! A module containing a struct designed to represent imaginary numbers.
//!
//! Imaginary numbers can either be represented by two integer coefficients using the [`Integer`] type, or in polar form using two `f64`s.

use crate::{
    types::integer::{FloatToIntegerConversionError, Integer, IntegerSerError, SignedState},
    utilities::cursor::Cursor,
};
use alloc::vec::Vec;
use core::{
    f64::consts::PI,
    fmt::{Display, Formatter},
    hash::Hash,
    num::FpCategory,
};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
///This struct represents imaginary numbers
pub enum Imaginary {
    ///An imaginary number represented by two integer coefficients for the real and imaginary parts
    CartesianForm {
        #[allow(missing_docs)]
        real: Integer,
        #[allow(missing_docs)]
        imaginary: Integer,
    },
    ///An imaginary number represented as a polar coordinate with real on the x-axis and imaginary on the y-axis
    PolarForm {
        #[allow(missing_docs)]
        modulus: f64,
        #[allow(missing_docs)]
        argument: f64,
    },
}

impl Hash for Imaginary {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);

        match self {
            Self::CartesianForm { real, imaginary } => {
                real.hash(state);
                imaginary.hash(state);
            }
            Self::PolarForm { modulus, argument } => {
                match modulus.classify() {
                    FpCategory::Nan => 0,
                    FpCategory::Infinite => 1,
                    FpCategory::Zero => 2,
                    FpCategory::Subnormal => 3,
                    FpCategory::Normal => 4,
                }
                .hash(state);
                modulus.to_le_bytes().hash(state);

                match argument.classify() {
                    FpCategory::Nan => 0,
                    FpCategory::Infinite => 1,
                    FpCategory::Zero => 2,
                    FpCategory::Subnormal => 3,
                    FpCategory::Normal => 4,
                }
                .hash(state);
                argument.to_le_bytes().hash(state);
            }
        }
    }
}

impl Display for Imaginary {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::CartesianForm { real, imaginary } => {
                if imaginary.is_negative() {
                    write!(f, "{real}{imaginary}i")
                } else {
                    write!(f, "{real}+{imaginary}i")
                }
            }
            Self::PolarForm { modulus, argument } => {
                write!(f, "{modulus} e^({argument}i)")
            }
        }
    }
}

impl Imaginary {
    ///Converts an imaginary number into polar form.
    ///
    /// - If the number was already in polar form, then this just returns the `self`.
    /// - If the number was in integer coefficient form, then it uses [`Imaginary::polar_from_cartesian`].
    #[must_use]
    pub fn to_polar_form(self) -> Self {
        match self {
            pf @ Self::PolarForm { .. } => pf,
            Self::CartesianForm { real, imaginary } => {
                Self::polar_from_cartesian(real.into(), imaginary.into())
            }
        }
    }

    ///Converts an imaginary number into cartesian integer form.
    ///
    /// - If the number was already in integer form, then this just returns the `self` in the [`Ok`] side.
    /// - If the number wasn't, it tries to get the real and imaginary parts in floating-point form. It then uses [`Integer::try_from`] to try to convert from the [`f64`]s to [`Integer`]s. If either part fails to convert, it returns the polar form in the [`Err`] side.
    ///
    /// ## Errors
    ///
    /// This can fail if one of two situations occurs:
    /// - The floating point number cannot fit into an [`Integer`].
    /// - The floating point number has any decimal part - this is checked by running [`f64::fract`] and then comparing with [`f64::EPSILON`].
    pub fn to_cartesian_form(self) -> Result<Self, (Self, FloatToIntegerConversionError)> {
        match self {
            ic @ Self::CartesianForm { .. } => Ok(ic),
            Self::PolarForm { modulus, argument } => {
                let real = match Integer::try_from(modulus * argument.cos()) {
                    Ok(r) => r,
                    Err(e) => {
                        return Err((Self::PolarForm { modulus, argument }, e));
                    }
                };

                let imaginary = match Integer::try_from(modulus * argument.sin()) {
                    Ok(i) => i,
                    Err(e) => {
                        return Err((Self::PolarForm { modulus, argument }, e));
                    }
                };

                Ok(Self::CartesianForm { real, imaginary })
            }
        }
    }

    #[must_use]
    ///Converts cartesian coordinates to polar coordinates. The argument is in the range \[-π,π\]
    pub fn polar_from_cartesian(real: f64, imaginary: f64) -> Self {
        let modulus = real.hypot(imaginary);
        let phi = (imaginary.abs() / real.abs()).atan();
        let argument = match (real.is_sign_negative(), imaginary.is_sign_negative()) {
            (true, true) => -PI + phi,
            (true, false) => PI - phi,
            (false, true) => -phi,
            (false, false) => phi,
        };

        Imaginary::PolarForm { modulus, argument }
    }

    ///Serialises the floating point number into 4 magic bits and bytes.
    ///
    /// The 4 magic bits are kept inside the range `0b0000_0000` to `0b0000_1111`.
    ///
    /// ## Polar Form
    /// - The magic bits only contain `0b0001` which represents that it is polar form.
    /// - The modulus and argument are directly written into the vector in that order and so the vector is guaranteed to be 16 bytes.
    ///
    /// ## Cartesian Form
    /// - The magic bits contain values inside `0b0110` - the first `1` represents the sign of the imaginary part and the second `1` represents the sign of the real part.
    /// - The real and imaginary parts are serialised into the vector in that order using [`Integer::ser`]
    #[must_use]
    pub fn ser(&self) -> (u8, Vec<u8>) {
        match self {
            Imaginary::CartesianForm { real, imaginary } => {
                let (re_ss, mut re_bytes) = real.ser();
                let (im_ss, im_bytes) = imaginary.ser();

                re_bytes.extend(im_bytes.iter());

                ((u8::from(re_ss) << 1) | (u8::from(im_ss) << 2), re_bytes)
            }
            Imaginary::PolarForm { modulus, argument } => {
                let mut bytes = Vec::with_capacity(16);
                bytes.extend(modulus.to_le_bytes());
                bytes.extend(argument.to_le_bytes());

                (1, bytes)
            }
        }
    }

    ///Deserialises 4 magic bits (contained within `0b0000_1111`) and bytes contained within a [`Cursor`] into an imaginary number.
    ///
    /// ## Errors
    /// - If there aren't enough bytes, it will fail with [`IntegerSerError::NotEnoughBytes`].
    /// - If it fails to deserialise the [`Integer`], it will fail using [`IntegerSerError`].
    /// - If it fails to deserialise the [`SignedState`], it will fail using [`IntegerSerError::InvalidSignedStateDiscriminant`].
    pub fn deser(magic_bits: u8, bytes: &mut Cursor<u8>) -> Result<Self, IntegerSerError> {
        if magic_bits & 0b0001 == 0 {
            let real_signed_state = SignedState::try_from((magic_bits & 0b0010) >> 1)?;
            let imaginary_signed_state = SignedState::try_from((magic_bits & 0b0100) >> 2)?;

            let real = Integer::deser(real_signed_state, bytes)?;
            let imaginary = Integer::deser(imaginary_signed_state, bytes)?;
            Ok(Self::CartesianForm { real, imaginary })
        } else {
            let modulus = f64::from_le_bytes(
                *bytes
                    .read_specific()
                    .ok_or(IntegerSerError::NotEnoughBytes)?,
            );
            let argument = f64::from_le_bytes(
                *bytes
                    .read_specific()
                    .ok_or(IntegerSerError::NotEnoughBytes)?,
            );

            Ok(Self::PolarForm { modulus, argument })
        }
    }
}
