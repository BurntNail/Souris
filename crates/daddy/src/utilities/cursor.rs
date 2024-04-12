pub struct Cursor<'a> {
    backing: &'a [u8],
    pos: u32,
}

impl<'a> Cursor<'a> {
    ///Fails if `backing.len()` > `u32::MAX`
    pub fn new(backing: &'a impl AsRef<[u8]>) -> Option<Self> {
        let backing = backing.as_ref();
        if backing.len() as u32 > u32::MAX {
            return None;
        }

        Some(Self { backing, pos: 0 })
    }

    ///Returns whether move was successful
    pub fn seek(&mut self, offset: i64) -> bool {
        let current = self.pos as i64;

        let new_pos = current + offset;

        if (0..=self.backing.len() as i64).contains(&new_pos) {
            self.pos = new_pos as u32;
            true
        } else {
            false
        }
    }

    pub fn read(&mut self, n: u32) -> Option<&'a [u8]> {
        let start = self.pos as usize;
        if !self.seek(n as i64) {
            return None;
        }
        let end = self.pos as usize;

        Some(&self.backing[start..end])
    }

    pub fn peek(&mut self, n: usize) -> Option<&'a [u8]> {
        let start = self.pos as usize;
        let end = start + n;
        if end > self.backing.len() {
            return None;
        }

        Some(&self.backing[start..=end])
    }

    #[must_use]
    pub fn position(&self) -> u32 {
        self.pos
    }
}
