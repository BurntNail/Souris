use crate::{
    types::integer::{Integer, IntegerSerError},
    utilities::cursor::Cursor,
    version::Version,
};
use alloc::{vec, vec::Vec};
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use core::fmt::{Display, Formatter};

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Timestamp(pub NaiveDateTime);

impl Display for Timestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug)]
pub enum TSError {
    IntegerSerError(IntegerSerError),
    InvalidDateOrTime,
}

impl From<IntegerSerError> for TSError {
    fn from(value: IntegerSerError) -> Self {
        Self::IntegerSerError(value)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TSError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TSError::IntegerSerError(e) => Some(e),
            TSError::InvalidDateOrTime => None,
        }
    }
}

impl Display for TSError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            TSError::IntegerSerError(e) => write!(f, "Error de/ser-ing integer: {e:?}"),
            TSError::InvalidDateOrTime => write!(f, "Invalid date or time"),
        }
    }
}

impl Timestamp {
    #[must_use]
    pub fn ser(&self, version: Version) -> Vec<u8> {
        match version {
            Version::V0_1_0 => {
                let date = self.0.date();
                let year = date.year();
                let month = date.month();
                let day = date.day();

                let time = self.0.time();
                let hour = time.hour();
                let minute = time.minute();
                let sec = time.second();
                let nanos = time.nanosecond();

                let mut res = vec![];

                res.extend(Integer::i32(year).ser(version));
                res.extend(Integer::u32(month).ser(version));
                res.extend(Integer::u32(day).ser(version));
                res.extend(Integer::u32(hour).ser(version));
                res.extend(Integer::u32(minute).ser(version));
                res.extend(Integer::u32(sec).ser(version));
                res.extend(Integer::u32(nanos).ser(version));

                res
            }
        }
    }

    pub fn deser(bytes: &mut Cursor<u8>, version: Version) -> Result<Self, TSError> {
        match version {
            Version::V0_1_0 => {
                let year = Integer::deser(bytes, version)?.try_into()?;
                let month = Integer::deser(bytes, version)?.try_into()?;
                let day = Integer::deser(bytes, version)?.try_into()?;

                let date =
                    NaiveDate::from_ymd_opt(year, month, day).ok_or(TSError::InvalidDateOrTime)?;

                let hour = Integer::deser(bytes, version)?.try_into()?;
                let min = Integer::deser(bytes, version)?.try_into()?;
                let sec = Integer::deser(bytes, version)?.try_into()?;
                let ns = Integer::deser(bytes, version)?.try_into()?;

                let time = NaiveTime::from_hms_nano_opt(hour, min, sec, ns)
                    .ok_or(TSError::InvalidDateOrTime)?;

                Ok(Self(NaiveDateTime::new(date, time)))
            }
        }
    }
}
