//! A module which contains a read-only cursor for use when deserialising variable-length structs in the middle of a larger byte sequence.
//!
//! Easily construct a cursor using [`Cursor::new`], and then use various methods to move the pointer around and read data.
//!
//! [`Cursor`] also implements [`AsRef`] for `&[T]`, which uses the remaining items.
//!
//! ```rust
//! use sourisdb::utilities::cursor::Cursor;
//!
//! let data: Vec<u8> = b"Hello, World!".to_vec();
//! let mut cursor = Cursor::new(&data); // cursor takes a reference to a vec here, but anything that implements `AsRef<[T]>` could be used.
//!
//! assert!(cursor.move_forwards(7)); //move forward 7 items
//!
//! let remaining = cursor.as_ref(); //as_ref doesn't move the pointer
//! assert_eq!(remaining, b"World!");
//!
//! assert_eq!(cursor.items_remaining(), 6);
//!```

///An immutable cursor into a borrowed slice of elements.
pub struct Cursor<'a, T> {
    backing: &'a [T],
    pos: usize,
}

impl<'a, T> Cursor<'a, T> {
    ///Create a new cursor
    pub fn new(backing: &'a impl AsRef<[T]>) -> Self {
        Self {
            backing: backing.as_ref(),
            pos: 0,
        }
    }

    ///Moves the pointer forwards by the specified offset.
    ///
    /// Returns:
    /// - `true` if the move was successful
    /// - `false` if the move was out-of-bounds
    pub fn move_forwards(&mut self, offset: usize) -> bool {
        let Some(new_pos) = self.pos.checked_add(offset) else {
            return false;
        };
        if new_pos > self.backing.len() {
            return false;
        }

        self.pos = new_pos;
        true
    }

    ///Moves the pointer backwards by the specified offset.
    ///
    /// Returns:
    /// - `true` if the move was successful
    /// - `false` if the move was out-of-bounds
    pub fn move_backwards(&mut self, offset: usize) -> bool {
        let Some(new_pos) = self.pos.checked_sub(offset) else {
            return false;
        };

        self.pos = new_pos;
        true
    }

    ///Reads a specified number of elements starting from the cursor's position. The cursor is also moved to the next position after the last element revealed.
    ///
    /// - If the elements would go out of bounds, `None` is returned, rather than a list with a different length.
    /// - If the cursor is at the end (can be checked using [`Cursor::is_finished`], `None` is **always** returned.
    pub fn read(&mut self, n: usize) -> Option<&'a [T]> {
        let start = self.pos;
        let end = start.checked_add(n)?;
        if end > self.backing.len() {
            return None;
        }
        self.pos = end;

        Some(&self.backing[start..end])
    }

    ///Reads a specified number of elements starting from the cursor's position. The cursor is also moved to the next position after the last element revealed.
    ///
    /// This method might be useful for if the number of elements is always known at compile-time. One example is shown here for retreiving an `f64` from some bytes:
    ///```rust
    /// use sourisdb::utilities::cursor::Cursor;
    ///
    /// let expected_f64: f64 = 123.456;
    /// let mut bytes = expected_f64.to_le_bytes().to_vec();
    /// bytes.insert(0, 12);
    /// bytes.push(34); //simulate extra bytes around the `f64`.
    ///
    /// let mut cursor = Cursor::new(&bytes);
    /// cursor.move_forwards(1);
    /// let found_f64 = f64::from_le_bytes(*cursor.read_exact().unwrap());
    /// assert_eq!(expected_f64, found_f64);
    /// ```
    ///
    /// - If the elements would go out of bounds, `None` is returned, rather than a list with a different length.
    /// - If the cursor is at the end (can be checked using [`Cursor::is_finished`], `None` is **always** returned.
    pub fn read_exact<const N: usize>(&mut self) -> Option<&'a [T; N]> {
        let start = self.pos;
        let end = start.checked_add(N)?;
        if end > self.backing.len() {
            return None;
        }
        self.pos = end;

        (&self.backing[start..end]).try_into().ok()
    }

    ///Peeks at a certain number of bytes - follows the exact same behaviour as [`Cursor::read`] but without changing the position of the pointer.
    #[must_use]
    pub fn peek(&self, n: usize) -> Option<&'a [T]> {
        let start = self.pos;
        let end = start.checked_add(n)?;
        if end > self.backing.len() {
            return None;
        }

        Some(&self.backing[start..end])
    }

    ///Peeks at a certain generic number of bytes.
    #[must_use]
    pub fn peek_exact<const N: usize>(&self) -> Option<&'a [T; N]> {
        let start = self.pos;
        let end = start.checked_add(N)?;
        if end > self.backing.len() {
            return None;
        }

        (&self.backing[start..end]).try_into().ok()
    }

    #[must_use]
    ///Reads all remaining bytes, and finishes the cursor.
    ///
    /// If none are left, it returns an empty slice.
    pub fn read_remaining(&mut self) -> &[T] {
        if self.pos >= self.backing.len() {
            &[]
        } else {
            let backup = self.pos;
            self.pos = self.backing.len();
            &self.backing[backup..]
        }
    }

    ///Peeks all remaining bytes, without finishing the cursor.
    ///
    /// If none are left, it returns an empty slice.
    #[must_use]
    pub fn peek_remaining(&self) -> &[T] {
        self.as_ref()
    }

    #[must_use]
    ///Returns the current zero-indexed position of the pointer in the list.
    ///
    /// NB: this will always be in the range `0..=backing.len()`
    pub fn pos(&self) -> usize {
        self.pos
    }

    #[must_use]
    ///returns the number of items remaining
    pub fn items_remaining(&self) -> usize {
        self.backing.len() - self.pos
    }

    ///Sets the position of the pointer.
    ///
    /// NB: if the position given is greater than the length of the list, the pointer will just be set to the end of the list.
    pub fn set_pos(&mut self, new: usize) {
        self.pos = self.backing.len().min(new);
    }

    #[must_use]
    ///Returns whether the cursor is finished.
    pub fn is_finished(&self) -> bool {
        self.pos >= self.backing.len()
    }
}

impl<'a, T> AsRef<[T]> for Cursor<'a, T> {
    fn as_ref(&self) -> &'a [T] {
        if self.pos >= self.backing.len() {
            &[]
        } else {
            &self.backing[self.pos..]
        }
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

#[cfg(feature = "std")]
impl<'a, T> std::io::Seek for Cursor<'a, T> {
    #[allow(
        clippy::collapsible_if,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        match pos {
            std::io::SeekFrom::Current(offset) => {
                if offset > 0 {
                    if !self.move_forwards(offset as usize) {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            "Goes out of bounds".to_string(),
                        ));
                    }
                }
            }
            std::io::SeekFrom::End(offset) => {
                if offset > 0 || !self.move_backwards((-offset) as usize) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "Goes out of bounds".to_string(),
                    ));
                }
            }
            std::io::SeekFrom::Start(offset) => {
                if !self.move_backwards(offset as usize) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "Goes out of bounds".to_string(),
                    ));
                }
            }
        }

        Ok(self.pos as u64)
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

        cursor.move_backwards(4);
        assert_eq!(cursor.pos(), 6);
        assert_eq!(cursor.peek_remaining(), &[6, 7, 8, 9]);

        assert_eq!(cursor.read(2), Some([6, 7].as_slice()));

        cursor.move_forwards(1);
        assert_eq!(cursor.pos(), 9);
        assert_eq!(cursor.read(1), Some([9].as_slice()));
        assert_eq!(cursor.pos(), 10);

        assert_eq!(cursor.read(1), None);
        assert_eq!(cursor.pos(), 10);
    }
}
