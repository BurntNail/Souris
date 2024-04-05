#[derive(Copy, Clone, Debug)]
pub enum Version {
    V0_1_0
}

impl Version {
    pub fn to_bytes (self) -> &'static [u8] {
        match self {
            Self::V0_1_0 => b"V0_1_0"
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes {
            b"V0_1_0" => Some(Self::V0_1_0),
            _ => None,
        }
    }
}