pub struct Cursor<'a, T> {
    backing: &'a [T],
    pos: usize,
}

impl<'a, T> Cursor<'a, T> {
    pub fn new(backing: &'a impl AsRef<[T]>) -> Self {
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

    pub fn read(&mut self, n: usize) -> Option<&'a [T]> {
        let start = self.pos;
        let end = start.checked_add(n)?;
        if end > self.backing.len() {
            return None;
        }
        self.pos = end;

        Some(&self.backing[start..end])
    }

    pub fn read_specific<const N: usize>(&mut self) -> Option<&'a [T; N]> {
        let start = self.pos;
        let end = start.checked_add(N)?;
        if end > self.backing.len() {
            return None;
        }
        self.pos = end;


        (&self.backing[start..end]).try_into().ok()
    }

    pub fn peek(&mut self, n: usize) -> Option<&'a [T]> {
        let start = self.pos;
        let end = start + n;
        if end > self.backing.len() {
            return None;
        }

        Some(&self.backing[start..end])
    }

    #[must_use]
    pub fn read_remaining(&mut self) -> &[T] {
        if self.pos >= self.backing.len() {
            &[]
        } else {
            let backup = self.pos;
            self.pos = self.backing.len();
            &self.backing[backup..]
        }
    }

    #[must_use]
    pub fn peek_remaining(&self) -> &[T] {
        if self.pos >= self.backing.len() {
            &[]
        } else {
            &self.backing[self.pos..]
        }
    }

    #[must_use]
    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn set_pos(&mut self, new: usize) {
        self.pos = new;
    }

    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.pos >= self.backing.len()
    }
}

impl<'a, T> AsRef<[T]> for Cursor<'a, T> {
    fn as_ref(&self) -> &'a [T] {
        &self.backing[self.pos..]
    }
}
    
impl<'a, T> core::iter::Iterator for Cursor<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.backing.len() {
            return None;
        }

        self.pos += 1;
        Some(&self.backing[self.pos - 1])
    }
}

#[cfg(test)]
mod tests {
    use crate::utilities::cursor::Cursor;

    #[test]
    fn test_cursor_movement() {
        let data = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut cursor = Cursor::new(&data);

        assert_eq!(cursor.read(5), Some([0, 1, 2, 3, 4].as_slice()));
        assert_eq!(cursor.pos(), 5);

        assert_eq!(cursor.peek(5), Some([5, 6, 7, 8, 9].as_slice()));
        assert_eq!(cursor.pos(), 5);

        assert_eq!(cursor.read(5), Some([5, 6, 7, 8, 9].as_slice()));
        assert_eq!(cursor.pos(), 10);
        let empty: &[i32] = &[];
        assert_eq!(cursor.peek_remaining(), empty);

        cursor.seek_backwards(4);
        assert_eq!(cursor.pos(), 6);
        assert_eq!(cursor.peek_remaining(), &[6, 7, 8, 9]);

        assert_eq!(cursor.read(2), Some([6, 7].as_slice()));

        cursor.seek(2);
        assert_eq!(cursor.pos(), 8);
        assert_eq!(cursor.read(1), Some([8].as_slice()));
        assert_eq!(cursor.pos(), 9);

        assert_eq!(cursor.read(10), None);
        assert_eq!(cursor.pos(), 9);
    }
}
