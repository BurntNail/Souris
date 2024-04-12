use crate::utilities::cursor::Cursor;

#[derive(Copy, Clone, Debug)]
pub enum Version {
    V0_1_0,
}

#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
pub enum VersionSerError {
    Invalid,
    NotEnoughBytes,
}

impl Version {
    #[must_use]
    pub fn to_bytes(self) -> &'static [u8] {
        match self {
            Self::V0_1_0 => b"V0_1_0",
        }
    }

    pub fn from_bytes(cursor: &mut Cursor) -> Result<Self, VersionSerError> {
        match cursor.read(6).ok_or(VersionSerError::NotEnoughBytes)? {
            b"V0_1_0" => Ok(Self::V0_1_0),
            _ => Err(VersionSerError::Invalid),
        }
    }
}
