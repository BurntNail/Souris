//! A module containing a struct designed to represent imaginary numbers.
//!
//! Imaginary numbers can either be represented by two integer coefficients using the [`Integer`] type, or in polar form using two `f64`s.

use alloc::vec::Vec;
use core::{
    f64::consts::PI,
    fmt::{Display, Formatter},
    hash::Hash,
    num::FpCategory,
};

use crate::{
    types::integer::{FloatToIntegerConversionError, Integer, IntegerSerError, SignedState},
    utilities::cursor::Cursor,
};

#[derive(Debug, Clone, PartialEq, Copy)]
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
    ///
    ///```rust
    /// use std::f64::consts::PI;
    /// use sourisdb::types::imaginary::Imaginary;
    ///
    /// let cartesian_form = Imaginary::CartesianForm { real: 2.into(), imaginary: (-2).into() };
    /// let polar_form = cartesian_form.to_polar_form();
    ///
    /// let Imaginary::PolarForm {modulus, argument } = polar_form else {unreachable!()};
    /// let expected_modulus: f64 = 2.0 * 2.0_f64.sqrt();
    /// let expected_argument: f64 = - PI / 4.0;
    ///
    /// assert!((modulus - expected_modulus).abs() < f64::EPSILON);
    /// assert!((argument - expected_argument).abs() < f64::EPSILON);
    /// ```
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
    ///```rust
    /// use sourisdb::types::imaginary::Imaginary;
    /// let polar_form = Imaginary::PolarForm {modulus: 13.0, argument: -(5.0_f64 / 12.0_f64).atan()};
    ///
    /// let cartesian_form = polar_form.to_cartesian_form().unwrap();
    /// let Imaginary::CartesianForm {real, imaginary} = cartesian_form else {unreachable!()};
    ///
    /// let expected_real = 12.into();
    /// let expected_imaginary = (-5).into();
    ///
    /// assert_eq!(real, expected_real);
    /// assert_eq!(imaginary, expected_imaginary);
    /// ```
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
    ///
    ///```rust
    ///
    /// use std::f64::consts::PI;
    /// use sourisdb::types::imaginary::Imaginary;
    ///
    /// let polar_form = Imaginary::polar_from_cartesian(-1.0, 1.0);
    /// let Imaginary::PolarForm {modulus, argument} = polar_form else {unreachable!()};
    ///
    /// let expected_modulus = 2.0_f64.sqrt();
    /// let expected_argument = 0.75_f64 * PI;
    /// assert!((modulus - expected_modulus).abs() < f64::EPSILON);
    /// assert!((argument - expected_argument).abs() < f64::EPSILON);
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
    /// - The magic bits only contain `0` which represents that it is polar form.
    /// - The modulus and argument are directly written into the vector in that order and so the vector is guaranteed to be 16 bytes.
    ///
    /// ## Cartesian Form
    /// - The real and imaginary parts are serialised into the vector in that order using [`Integer::ser`]
    /// - The magic bits contain values inside contain the following contents when turned into a u8:
    ///
    /// `u` = unsigned integer, `sp` = signed positive integer, `sn` = signed negative integer
    /// 1. `u,u`
    /// 2. `u,sp`
    /// 3. `sp,u`
    /// 4. `u,sn`
    /// 5. `sn,u`
    /// 6. `sp,sp`
    /// 7. `sp,sn`
    /// 8. `sn, sp`
    /// 9. `sn, sn`
    #[must_use]
    pub fn ser(&self) -> (u8, Vec<u8>) {
        use SignedState::{SignedNegative as SN, SignedPositive as SP, Unsigned as U};

        match self {
            Imaginary::CartesianForm { real, imaginary } => {
                //serialise
                let (re_ss, mut re_bytes) = real.ser();
                let (im_ss, im_bytes) = imaginary.ser();

                let magic_bytes = match (re_ss, im_ss) {
                    (U, U) => 1,
                    (U, SP) => 2,
                    (SP, U) => 3,
                    (U, SN) => 4,
                    (SN, U) => 5,
                    (SP, SP) => 6,
                    (SP, SN) => 7,
                    (SN, SP) => 8,
                    (SN, SN) => 9,
                };

                re_bytes.extend(im_bytes.iter());

                (magic_bytes, re_bytes)
            }
            Imaginary::PolarForm { modulus, argument } => {
                let mut bytes = Vec::with_capacity(16);
                bytes.extend(modulus.to_le_bytes());
                bytes.extend(argument.to_le_bytes());

                (0, bytes)
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
        use SignedState::{SignedNegative as SN, SignedPositive as SP, Unsigned as U};

        if magic_bits > 0 {
            let (real_signed_state, imaginary_signed_state) = match magic_bits {
                0 => unreachable!(),
                1 => (U, U),
                2 => (U, SP),
                3 => (SP, U),
                4 => (U, SN),
                5 => (SN, U),
                6 => (SP, SP),
                7 => (SP, SN),
                8 => (SN, SP),
                9 => (SN, SN),
                _ => return Err(IntegerSerError::InvalidSignedStateDiscriminant(magic_bits)),
            };

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
