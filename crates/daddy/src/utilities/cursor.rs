pub struct Cursor<'a> {
    backing: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(backing: &'a impl AsRef<[u8]>) -> Self {
        Self {
            backing: backing.as_ref(),
            pos: 0,
        }
    }

    ///Returns whether move was successful
    pub fn seek(&mut self, offset: usize) -> bool {
        let Some(new_pos) = self.pos.checked_add(offset) else {
            return false;
        };

        if new_pos < self.backing.len() {
            self.pos = new_pos;
            true
        } else {
            false
        }
    }

    ///Returns whether move was successful
    pub fn seek_backwards(&mut self, offset: usize) -> bool {
        let Some(new_pos) = self.pos.checked_sub(offset) else {
            return false;
        };

        self.pos = new_pos;
        true
    }

    pub fn read(&mut self, n: usize) -> Option<&'a [u8]> {
        let start = self.pos;
        let end = start.checked_add(n)?;
        if end > self.backing.len() {
            return None;
        }
        self.pos = end;

        Some(&self.backing[start..end])
    }

    pub fn peek(&mut self, n: usize) -> Option<&'a [u8]> {
        let start = self.pos;
        let end = start + n;
        if end > self.backing.len() {
            return None;
        }

        Some(&self.backing[start..=end])
    }

    #[must_use]
    pub fn position(&self) -> usize {
        self.pos
    }
}
