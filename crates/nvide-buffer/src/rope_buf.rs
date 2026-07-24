//! Default rope buffer implementation (ropey).

use std::ops::Range;

use ropey::Rope;

use crate::{Buffer, BufferError, BufferId, EditOp, Encoding, UndoTree};

/// Rope-backed buffer with line index (via ropey) and undo.
#[derive(Debug, Clone)]
pub struct RopeBuffer {
    id: BufferId,
    text: Rope,
    encoding: Encoding,
    dirty: bool,
    version: u64,
    read_only: bool,
    undo: UndoTree,
}

impl RopeBuffer {
    pub fn new(id: BufferId) -> Self {
        Self {
            id,
            text: Rope::new(),
            encoding: Encoding::Utf8,
            dirty: false,
            version: 0,
            read_only: false,
            undo: UndoTree::new(),
        }
    }

    pub fn from_str(id: BufferId, s: &str) -> Self {
        let mut buf = Self::new(id);
        if !s.is_empty() {
            buf.text = Rope::from_str(s);
        }
        buf
    }

    pub fn id(&self) -> BufferId {
        self.id
    }

    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn set_read_only(&mut self, read_only: bool) {
        self.read_only = read_only;
    }

    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// Insert with undo recording.
    pub fn insert_tracked(&mut self, pos: usize, text: &str) -> Result<(), BufferError> {
        self.insert(pos, text)?;
        if !text.is_empty() {
            let len = text.chars().count();
            self.undo.push(vec![EditOp::Delete { pos, len }]);
        }
        Ok(())
    }

    /// Delete with undo recording.
    pub fn delete_tracked(&mut self, range: Range<usize>) -> Result<(), BufferError> {
        if range.start == range.end {
            return Ok(());
        }
        let deleted = self.slice(range.clone())?;
        self.delete(range.clone())?;
        self.undo.push(vec![EditOp::Insert {
            pos: range.start,
            text: deleted,
        }]);
        Ok(())
    }

    /// Undo one recorded group; restores prior text.
    pub fn undo(&mut self) -> Result<bool, BufferError> {
        let Some(inverse) = self.undo.pop_undo() else {
            return Ok(false);
        };
        let mut forward = Vec::with_capacity(inverse.len());
        for op in inverse {
            match op {
                EditOp::Insert { pos, text } => {
                    let len = text.chars().count();
                    self.apply_insert(pos, &text)?;
                    forward.push(EditOp::Delete { pos, len });
                }
                EditOp::Delete { pos, len } => {
                    let end = pos.checked_add(len).ok_or(BufferError::InvalidRange {
                        start: pos,
                        end: pos.saturating_add(len),
                        len: self.len_chars(),
                    })?;
                    let deleted = self.slice(pos..end)?;
                    self.apply_delete(pos..end)?;
                    forward.push(EditOp::Insert { pos, text: deleted });
                }
            }
        }
        self.undo.push_redo(forward);
        Ok(true)
    }

    /// Redo one group.
    pub fn redo(&mut self) -> Result<bool, BufferError> {
        let Some(forward) = self.undo.pop_redo() else {
            return Ok(false);
        };
        let mut inverse = Vec::with_capacity(forward.len());
        for op in forward {
            match op {
                EditOp::Insert { pos, text } => {
                    let len = text.chars().count();
                    self.apply_insert(pos, &text)?;
                    inverse.push(EditOp::Delete { pos, len });
                }
                EditOp::Delete { pos, len } => {
                    let end = pos + len;
                    let deleted = self.slice(pos..end)?;
                    self.apply_delete(pos..end)?;
                    inverse.push(EditOp::Insert { pos, text: deleted });
                }
            }
        }
        self.undo.push_undo_only(inverse);
        Ok(true)
    }

    pub fn can_undo(&self) -> bool {
        self.undo.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.undo.can_redo()
    }

    fn check_writable(&self) -> Result<(), BufferError> {
        if self.read_only {
            Err(BufferError::ReadOnly)
        } else {
            Ok(())
        }
    }

    fn check_char_range(&self, range: &Range<usize>) -> Result<(), BufferError> {
        let len = self.len_chars();
        if range.start > range.end || range.end > len {
            return Err(BufferError::InvalidRange {
                start: range.start,
                end: range.end,
                len,
            });
        }
        Ok(())
    }

    fn bump(&mut self) {
        self.version = self.version.wrapping_add(1);
        self.dirty = true;
    }

    fn apply_insert(&mut self, pos: usize, text: &str) -> Result<(), BufferError> {
        self.check_writable()?;
        let len = self.len_chars();
        if pos > len {
            return Err(BufferError::CharOutOfBounds { index: pos, len });
        }
        if text.is_empty() {
            return Ok(());
        }
        self.text.insert(pos, text);
        self.bump();
        Ok(())
    }

    fn apply_delete(&mut self, range: Range<usize>) -> Result<(), BufferError> {
        self.check_writable()?;
        self.check_char_range(&range)?;
        if range.start == range.end {
            return Ok(());
        }
        self.text.remove(range.start..range.end);
        self.bump();
        Ok(())
    }
}

impl Buffer for RopeBuffer {
    fn len_chars(&self) -> usize {
        self.text.len_chars()
    }

    fn len_bytes(&self) -> usize {
        self.text.len_bytes()
    }

    fn line_count(&self) -> usize {
        // ropey: empty rope has 1 line
        self.text.len_lines()
    }

    fn insert(&mut self, pos: usize, text: &str) -> Result<(), BufferError> {
        self.apply_insert(pos, text)
    }

    fn delete(&mut self, range: Range<usize>) -> Result<(), BufferError> {
        self.apply_delete(range)
    }

    fn slice(&self, range: Range<usize>) -> Result<String, BufferError> {
        self.check_char_range(&range)?;
        Ok(self.text.slice(range.start..range.end).to_string())
    }

    fn line(&self, line: usize) -> Result<String, BufferError> {
        let line_count = self.line_count();
        if line >= line_count {
            return Err(BufferError::LineOutOfBounds { line, line_count });
        }
        let mut s = self.text.line(line).to_string();
        if s.ends_with('\n') {
            s.pop();
            if s.ends_with('\r') {
                s.pop();
            }
        }
        Ok(s)
    }

    fn char_to_byte(&self, char_idx: usize) -> Result<usize, BufferError> {
        let len = self.len_chars();
        if char_idx > len {
            return Err(BufferError::CharOutOfBounds {
                index: char_idx,
                len,
            });
        }
        Ok(self.text.char_to_byte(char_idx))
    }

    fn byte_to_char(&self, byte_idx: usize) -> Result<usize, BufferError> {
        let len = self.len_bytes();
        if byte_idx > len {
            return Err(BufferError::ByteOutOfBounds {
                index: byte_idx,
                len,
            });
        }
        Ok(self.text.byte_to_char(byte_idx))
    }

    fn to_string(&self) -> String {
        self.text.to_string()
    }

    fn version(&self) -> u64 {
        self.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Buffer;

    #[test]
    fn empty_buffer_basics() {
        let b = RopeBuffer::new(BufferId(1));
        assert_eq!(b.len_chars(), 0);
        assert!(b.is_empty());
        assert_eq!(b.line_count(), 1);
        assert_eq!(b.line(0).unwrap(), "");
        assert_eq!(b.to_string(), "");
    }

    #[test]
    fn insert_delete_slice_line() {
        let mut b = RopeBuffer::new(BufferId(1));
        b.insert(0, "hello\nworld").unwrap();
        assert_eq!(b.to_string(), "hello\nworld");
        assert_eq!(b.line_count(), 2);
        assert_eq!(b.line(0).unwrap(), "hello");
        assert_eq!(b.line(1).unwrap(), "world");
        assert_eq!(b.slice(0..5).unwrap(), "hello");
        b.delete(5..6).unwrap(); // remove newline
        assert_eq!(b.to_string(), "helloworld");
        assert_eq!(b.line_count(), 1);
    }

    #[test]
    fn char_byte_mapping() {
        let mut b = RopeBuffer::new(BufferId(1));
        b.insert(0, "aβc").unwrap(); // β is 2 bytes
        assert_eq!(b.len_chars(), 3);
        assert_eq!(b.len_bytes(), 4);
        assert_eq!(b.char_to_byte(0).unwrap(), 0);
        assert_eq!(b.char_to_byte(1).unwrap(), 1);
        assert_eq!(b.char_to_byte(2).unwrap(), 3);
        assert_eq!(b.byte_to_char(3).unwrap(), 2);
    }

    #[test]
    fn undo_restores_prior_text() {
        let mut b = RopeBuffer::from_str(BufferId(1), "abc");
        b.insert_tracked(3, "def").unwrap();
        assert_eq!(b.to_string(), "abcdef");
        assert!(b.undo().unwrap());
        assert_eq!(b.to_string(), "abc");
        b.insert_tracked(0, "X").unwrap();
        assert_eq!(b.to_string(), "Xabc");
        b.delete_tracked(1..4).unwrap();
        assert_eq!(b.to_string(), "X");
        assert!(b.undo().unwrap());
        assert_eq!(b.to_string(), "Xabc");
        assert!(b.undo().unwrap());
        assert_eq!(b.to_string(), "abc");
    }

    #[test]
    fn redo_after_undo() {
        let mut b = RopeBuffer::from_str(BufferId(1), "");
        b.insert_tracked(0, "hi").unwrap();
        b.undo().unwrap();
        assert_eq!(b.to_string(), "");
        assert!(b.redo().unwrap());
        assert_eq!(b.to_string(), "hi");
    }

    #[test]
    fn out_of_bounds() {
        let b = RopeBuffer::new(BufferId(1));
        assert!(b.slice(0..1).is_err());
        assert!(b.line(1).is_err());
        assert!(b.char_to_byte(1).is_err());
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use crate::Buffer;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// Insert then delete the inserted range restores original text.
        #[test]
        fn insert_delete_roundtrip(s in ".*{0,80}", pos in 0usize..100, t in ".*{0,40}") {
            let mut b = RopeBuffer::from_str(BufferId(1), &s);
            let len = b.len_chars();
            let pos = if len == 0 { 0 } else { pos % (len + 1) };
            let before = b.to_string();
            b.insert(pos, &t).unwrap();
            let t_len = t.chars().count();
            if t_len > 0 {
                b.delete(pos..pos + t_len).unwrap();
            }
            prop_assert_eq!(b.to_string(), before);
        }

        /// Slice of full range equals to_string.
        #[test]
        fn slice_full_eq_to_string(s in ".*{0,120}") {
            let b = RopeBuffer::from_str(BufferId(1), &s);
            let full = b.slice(0..b.len_chars()).unwrap();
            prop_assert_eq!(full, b.to_string());
            prop_assert_eq!(b.to_string(), s);
        }

        /// Tracked insert + undo restores text and version still advances.
        #[test]
        fn tracked_insert_undo(s in ".*{0,60}", t in ".+{1,20}") {
            let mut b = RopeBuffer::from_str(BufferId(1), &s);
            let before = b.to_string();
            let pos = b.len_chars() / 2;
            b.insert_tracked(pos, &t).unwrap();
            prop_assert_ne!(&b.to_string(), &before);
            prop_assert!(b.undo().unwrap());
            prop_assert_eq!(b.to_string(), before);
        }

        /// Concatenation of all lines (with `\n`) rebuilds ASCII content.
        #[test]
        fn lines_cover_content(s in "[a-zA-Z0-9 \t\n]{0,80}") {
            let b = RopeBuffer::from_str(BufferId(1), &s);
            let mut rebuilt = String::new();
            let n = b.line_count();
            for i in 0..n {
                rebuilt.push_str(&b.line(i).unwrap());
                if i + 1 < n {
                    rebuilt.push('\n');
                }
            }
            prop_assert_eq!(rebuilt, s);
        }

        /// char_to_byte / byte_to_char round-trip on valid indices.
        #[test]
        fn char_byte_roundtrip(s in ".*{0,50}") {
            let b = RopeBuffer::from_str(BufferId(1), &s);
            for ci in 0..=b.len_chars() {
                let bi = b.char_to_byte(ci).unwrap();
                prop_assert_eq!(b.byte_to_char(bi).unwrap(), ci);
            }
        }
    }
}
