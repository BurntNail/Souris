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

    pub fn peek(&mut self, n: usize) -> Option<&'a [T]> {
        let start = self.pos;
        let end = start + n;
        if end > self.backing.len() {
            return None;
        }

        Some(&self.backing[start..end])
    }

    #[must_use]
    pub fn position(&self) -> usize {
        self.pos
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
        assert_eq!(cursor.position(), 5);

        assert_eq!(cursor.peek(5), Some([5, 6, 7, 8, 9].as_slice()));
        assert_eq!(cursor.position(), 5);

        assert_eq!(cursor.read(5), Some([5, 6, 7, 8, 9].as_slice()));
        assert_eq!(cursor.position(), 10);
        
        cursor.seek_backwards(4);
        assert_eq!(cursor.position(), 6);
        
        assert_eq!(cursor.read(2), Some([6, 7].as_slice()));
        
        cursor.seek(2);
        assert_eq!(cursor.position(), 8);
        assert_eq!(cursor.read(1), Some([8].as_slice()));
        assert_eq!(cursor.position(), 9);

        assert_eq!(cursor.read(10), None);
        assert_eq!(cursor.position(), 9);
    }
}
