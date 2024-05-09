use crate::{
    types::integer::{Integer, IntegerSerError, SignedState},
    utilities::cursor::Cursor,
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
    pub fn ser(&self) -> (SignedState, Vec<u8>) {
        let date = self.0.date();
        let (year_ss, year) = Integer::from(date.year()).ser();
        let (_, month) = Integer::from(date.month()).ser();
        let (_, day) = Integer::from(date.day()).ser();

        let time = self.0.time();
        let (_, hour) = Integer::from(time.hour()).ser();
        let (_, minute) = Integer::from(time.minute()).ser();
        let (_, sec) = Integer::from(time.second()).ser();
        let (_, nanos) = Integer::from(time.nanosecond()).ser();

        let mut res = vec![];

        res.extend(year.iter());
        res.extend(month.iter());
        res.extend(day.iter());
        res.extend(hour.iter());
        res.extend(minute.iter());
        res.extend(sec.iter());
        res.extend(nanos.iter());

        (year_ss, res)
    }

    pub fn deser(year_signed_state: SignedState, bytes: &mut Cursor<u8>) -> Result<Self, TSError> {
        let year = Integer::deser(year_signed_state, bytes)?.try_into()?;
        let month = Integer::deser(SignedState::Positive, bytes)?.try_into()?;
        let day = Integer::deser(SignedState::Positive, bytes)?.try_into()?;

        let date = NaiveDate::from_ymd_opt(year, month, day).ok_or(TSError::InvalidDateOrTime)?;

        let hour = Integer::deser(SignedState::Positive, bytes)?.try_into()?;
        let min = Integer::deser(SignedState::Positive, bytes)?.try_into()?;
        let sec = Integer::deser(SignedState::Positive, bytes)?.try_into()?;
        let ns = Integer::deser(SignedState::Positive, bytes)?.try_into()?;

        let time =
            NaiveTime::from_hms_nano_opt(hour, min, sec, ns).ok_or(TSError::InvalidDateOrTime)?;

        Ok(Self(NaiveDateTime::new(date, time)))
    }
}
