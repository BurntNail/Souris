use std::io::{Cursor, Error as IOError, Read, Seek, SeekFrom};

#[derive(Copy, Clone, Debug)]
pub enum Version {
    V0_1_0
}

#[derive(Debug)]
pub enum VersionSerError {
    Invalid,
    NotEnoughBytes,
    IOError(IOError)
}

impl From<IOError> for VersionSerError {
    fn from(value: IOError) -> Self {
        Self::IOError(value)
    }
}

impl Version {
    pub fn to_bytes (self) -> &'static [u8] {
        match self {
            Self::V0_1_0 => b"V0_1_0"
        }
    }

    pub fn from_bytes(cursor: &mut Cursor<impl Read + AsRef<[u8]>>) -> Result<Self, VersionSerError> {
        let pos = cursor.position();
        let mut bytes = vec![];
        loop {
            let delta = cursor.position() - pos;
            let mut tmp = vec![0_u8; (6 - delta) as usize];
            if delta >= 6 {
                let delta = 6 - (delta as i64);
                cursor.seek(SeekFrom::Current(delta))?;
                
                break;
            }

            match cursor.read(&mut tmp)? {
                0 => return Err(VersionSerError::NotEnoughBytes),
                n => bytes.extend(&tmp[0..(n.max(delta as usize))])
            }
        }


        match bytes.as_slice() {
            b"V0_1_0" => Ok(Self::V0_1_0),
            _ => Err(VersionSerError::Invalid),
        }
    }
}