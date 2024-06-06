//! A module containing a struct designed to represent imaginary numbers.
//!
//! Imaginary numbers can either be represented by two integer coefficients using the [`Integer`] type, or in polar form using two `f64`s.

use crate::{
    types::integer::{Integer, IntegerSerError, SignedState},
    utilities::cursor::Cursor,
};
use alloc::vec::Vec;
use core::{
    fmt::{Display, Formatter},
    hash::Hash,
    num::FpCategory,
};
use std::f64::consts::PI;
use crate::types::integer::FloatToIntegerConversionError;
use crate::values::Value;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
///This struct represents imaginary numbers
pub enum Imaginary {
    IntegerCoefficients {
        real: Integer,
        imaginary: Integer,
    },
    PolarForm {
        modulus: f64,
        argument: f64,
    },
}

impl Hash for Imaginary {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);

        match self {
            Self::IntegerCoefficients { real, imaginary } => {
                real.hash(state);
                imaginary.hash(state);
            }
            Self::PolarForm {
                modulus,
                argument,
            } => {
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
            Self::IntegerCoefficients { real, imaginary } => {
                if imaginary.is_negative() {
                    write!(f, "{}{}i", real, imaginary)
                } else {
                    write!(f, "{}+{}i", real, imaginary)
                }
            }
            Self::PolarForm {
                modulus,
                argument,
            } => {
                write!(f, "{modulus} e^({argument}i)")
            }
        }
    }
}

impl Imaginary {
    #[must_use] pub fn to_polar_form (self) -> Self {
        match self {
            pf @ Self::PolarForm {.. } => pf,
            Self::IntegerCoefficients {real, imaginary} => {
                Self::polar_from_real_and_imaginary(real.into(), imaginary.into())
            }
        }
    }
    
    #[must_use] pub fn to_integer_coefficients(self) -> Result<Self, (Self, FloatToIntegerConversionError)> {
        match self {
            ic @ Self::IntegerCoefficients {..} => Ok(ic),
            Self::PolarForm {modulus, argument} => {
                let real = match Integer::try_from(modulus * argument.cos()) {
                    Ok(r) => r,
                    Err(e) => {
                        return Err((Self::PolarForm {modulus, argument}, e));
                    }
                }

                let imaginary = match Integer::try_from(modulus * argument.sin()) {
                    Ok(i) => i,
                    Err(e) => {
                        return Err((Self::PolarForm {modulus, argument}, e));
                    }
                }
                
                Ok(Self::IntegerCoefficients {real, imaginary})
            }
        }
    } 
    
    #[must_use] pub fn polar_from_real_and_imaginary (real: f64, imaginary: f64) -> Self {
        let modulus = real.hypot(imaginary);
        let phi = (imaginary.abs() / real.abs()).atan();
        let argument = match (
            real.is_sign_negative(),
            imaginary.is_sign_negative(),
        ) {
            (true, true) => -PI + phi,
            (true, false) => PI - phi,
            (false, true) => -phi,
            (false, false) => phi,
        };

        Imaginary::PolarForm {
            modulus,
            argument,
        }
    }
    
    #[must_use]
    pub fn ser(&self) -> (u8, Vec<u8>) {
        match self {
            Imaginary::IntegerCoefficients { real, imaginary } => {
                let (re_ss, mut re_bytes) = real.ser();
                let (im_ss, im_bytes) = imaginary.ser();

                re_bytes.extend(im_bytes.iter());

                ((u8::from(re_ss) << 1) | (u8::from(im_ss) << 2), re_bytes)
            }
            Imaginary::PolarForm {
                modulus,
                argument,
            } => {
                let mut bytes = Vec::with_capacity(16);
                bytes.extend(modulus.to_le_bytes());
                bytes.extend(argument.to_le_bytes());

                (1, bytes)
            }
        }
    }

    pub fn deser(magic_bits: u8, bytes: &mut Cursor<u8>) -> Result<Self, IntegerSerError> {
        if magic_bits & 0b0001 == 0 {
            let real_signed_state = SignedState::try_from((magic_bits & 0b0010) >> 1)?;
            let imaginary_signed_state = SignedState::try_from((magic_bits & 0b0100) >> 2)?;

            let real = Integer::deser(real_signed_state, bytes)?;
            let imaginary = Integer::deser(imaginary_signed_state, bytes)?;
            Ok(Self::IntegerCoefficients { real, imaginary })
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

            Ok(Self::PolarForm {
                modulus,
                argument,
            })
        }
    }
}
