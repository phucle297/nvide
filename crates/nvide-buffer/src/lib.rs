//! Rope-backed text storage and branching undo for the core process.

use ropey::Rope;
use std::{fmt, ops::Range};

pub type Version = u64;
pub type UndoNodeId = usize;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CursorSnapshot {
    pub anchor: usize,
    pub head: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Edit {
    Insert { at: usize, text: String },
    Delete { range: Range<usize> },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EditBatch {
    pub edits: Vec<Edit>,
    pub before: CursorSnapshot,
    pub after: CursorSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EditOutcome {
    pub version: Version,
    pub cursor: CursorSnapshot,
    pub node: UndoNodeId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BufferError {
    CharOutOfBounds {
        offset: usize,
        len: usize,
    },
    ByteOutOfBounds {
        offset: usize,
        len: usize,
    },
    ByteNotCharBoundary {
        offset: usize,
    },
    LineOutOfBounds {
        line: usize,
        count: usize,
    },
    InvalidRange {
        start: usize,
        end: usize,
        len: usize,
    },
    InvalidCursor {
        cursor: CursorSnapshot,
        len: usize,
    },
    NoUndo,
    NoRedo,
    RedoChoiceRequired {
        choices: Vec<UndoNodeId>,
    },
    InvalidRedoChoice {
        node: UndoNodeId,
    },
    VersionExhausted,
}

impl fmt::Display for BufferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CharOutOfBounds { offset, len } => {
                write!(formatter, "character offset {offset} exceeds length {len}")
            }
            Self::ByteOutOfBounds { offset, len } => {
                write!(formatter, "byte offset {offset} exceeds length {len}")
            }
            Self::ByteNotCharBoundary { offset } => {
                write!(formatter, "byte offset {offset} is not a UTF-8 boundary")
            }
            Self::LineOutOfBounds { line, count } => {
                write!(formatter, "line {line} exceeds line count {count}")
            }
            Self::InvalidRange { start, end, len } => {
                write!(
                    formatter,
                    "range {start}..{end} is invalid for length {len}"
                )
            }
            Self::InvalidCursor { cursor, len } => write!(
                formatter,
                "cursor {}..{} is invalid for length {len}",
                cursor.anchor, cursor.head
            ),
            Self::NoUndo => formatter.write_str("no undo is available"),
            Self::NoRedo => formatter.write_str("no redo is available"),
            Self::RedoChoiceRequired { choices } => {
                write!(formatter, "redo branch must be one of {choices:?}")
            }
            Self::InvalidRedoChoice { node } => write!(formatter, "invalid redo branch {node}"),
            Self::VersionExhausted => formatter.write_str("buffer version is exhausted"),
        }
    }
}

impl std::error::Error for BufferError {}

pub trait Buffer {
    fn len_chars(&self) -> usize;
    fn len_bytes(&self) -> usize;
    fn line_count(&self) -> usize;
    fn version(&self) -> Version;
    fn text(&self) -> String;
    fn line(&self, line: usize) -> Result<String, BufferError>;
    fn char_to_byte(&self, offset: usize) -> Result<usize, BufferError>;
    fn byte_to_char(&self, offset: usize) -> Result<usize, BufferError>;
    fn char_to_line(&self, offset: usize) -> Result<usize, BufferError>;
    fn line_to_char(&self, line: usize) -> Result<usize, BufferError>;
    fn apply_batch(&mut self, batch: EditBatch) -> Result<EditOutcome, BufferError>;
    fn insert(
        &mut self,
        at: usize,
        text: impl Into<String>,
        before: CursorSnapshot,
        after: CursorSnapshot,
    ) -> Result<EditOutcome, BufferError>;
    fn delete(
        &mut self,
        range: Range<usize>,
        before: CursorSnapshot,
        after: CursorSnapshot,
    ) -> Result<EditOutcome, BufferError>;
    fn undo(&mut self) -> Result<EditOutcome, BufferError>;
    fn redo(&mut self, choice: Option<UndoNodeId>) -> Result<EditOutcome, BufferError>;
    fn redo_choices(&self) -> &[UndoNodeId];
}

#[derive(Clone, Debug)]
struct UndoNode {
    parent: Option<UndoNodeId>,
    children: Vec<UndoNodeId>,
    forward: Vec<Edit>,
    inverse: Vec<Edit>,
    timestamp: u64,
    before: CursorSnapshot,
    after: CursorSnapshot,
}

#[derive(Clone, Debug)]
pub struct RopeBuffer {
    rope: Rope,
    version: Version,
    nodes: Vec<UndoNode>,
    current: UndoNodeId,
    next_timestamp: u64,
}

impl Default for RopeBuffer {
    fn default() -> Self {
        Self::new("")
    }
}

impl RopeBuffer {
    pub fn new(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            version: 0,
            nodes: vec![UndoNode {
                parent: None,
                children: Vec::new(),
                forward: Vec::new(),
                inverse: Vec::new(),
                timestamp: 0,
                before: CursorSnapshot::default(),
                after: CursorSnapshot::default(),
            }],
            current: 0,
            next_timestamp: 1,
        }
    }

    pub fn undo_timestamp(&self, node: UndoNodeId) -> Option<u64> {
        self.nodes.get(node).map(|node| node.timestamp)
    }

    fn validate_cursor(rope: &Rope, cursor: CursorSnapshot) -> Result<(), BufferError> {
        let len = rope.len_chars();
        if cursor.anchor <= len && cursor.head <= len {
            Ok(())
        } else {
            Err(BufferError::InvalidCursor { cursor, len })
        }
    }

    fn apply_edits(rope: &mut Rope, edits: &[Edit]) -> Result<Vec<Edit>, BufferError> {
        let mut inverse = Vec::with_capacity(edits.len());
        for edit in edits {
            match edit {
                Edit::Insert { at, text } => {
                    let len = rope.len_chars();
                    if *at > len {
                        return Err(BufferError::CharOutOfBounds { offset: *at, len });
                    }
                    rope.insert(*at, text);
                    inverse.push(Edit::Delete {
                        range: *at..*at + text.chars().count(),
                    });
                }
                Edit::Delete { range } => {
                    let len = rope.len_chars();
                    if range.start > range.end || range.end > len {
                        return Err(BufferError::InvalidRange {
                            start: range.start,
                            end: range.end,
                            len,
                        });
                    }
                    let removed = rope.slice(range.clone()).to_string();
                    rope.remove(range.clone());
                    inverse.push(Edit::Insert {
                        at: range.start,
                        text: removed,
                    });
                }
            }
        }
        inverse.reverse();
        Ok(inverse)
    }

    fn increment_version(&mut self) -> Result<Version, BufferError> {
        self.version = self
            .version
            .checked_add(1)
            .ok_or(BufferError::VersionExhausted)?;
        Ok(self.version)
    }

    fn replay(&mut self, edits: &[Edit]) -> Result<(), BufferError> {
        let mut candidate = self.rope.clone();
        Self::apply_edits(&mut candidate, edits)?;
        self.rope = candidate;
        Ok(())
    }
}

impl Buffer for RopeBuffer {
    fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    fn version(&self) -> Version {
        self.version
    }

    fn text(&self) -> String {
        self.rope.to_string()
    }

    fn line(&self, line: usize) -> Result<String, BufferError> {
        if line >= self.line_count() {
            return Err(BufferError::LineOutOfBounds {
                line,
                count: self.line_count(),
            });
        }
        Ok(self.rope.line(line).to_string())
    }

    fn char_to_byte(&self, offset: usize) -> Result<usize, BufferError> {
        if offset > self.len_chars() {
            return Err(BufferError::CharOutOfBounds {
                offset,
                len: self.len_chars(),
            });
        }
        Ok(self.rope.char_to_byte(offset))
    }

    fn byte_to_char(&self, offset: usize) -> Result<usize, BufferError> {
        if offset > self.len_bytes() {
            return Err(BufferError::ByteOutOfBounds {
                offset,
                len: self.len_bytes(),
            });
        }
        let char_offset = self.rope.byte_to_char(offset);
        if self.rope.char_to_byte(char_offset) == offset {
            Ok(char_offset)
        } else {
            Err(BufferError::ByteNotCharBoundary { offset })
        }
    }

    fn char_to_line(&self, offset: usize) -> Result<usize, BufferError> {
        if offset > self.len_chars() {
            return Err(BufferError::CharOutOfBounds {
                offset,
                len: self.len_chars(),
            });
        }
        Ok(self.rope.char_to_line(offset))
    }

    fn line_to_char(&self, line: usize) -> Result<usize, BufferError> {
        if line >= self.line_count() {
            return Err(BufferError::LineOutOfBounds {
                line,
                count: self.line_count(),
            });
        }
        Ok(self.rope.line_to_char(line))
    }

    fn apply_batch(&mut self, batch: EditBatch) -> Result<EditOutcome, BufferError> {
        Self::validate_cursor(&self.rope, batch.before)?;
        if batch.edits.is_empty() {
            Self::validate_cursor(&self.rope, batch.after)?;
            return Ok(EditOutcome {
                version: self.version,
                cursor: batch.after,
                node: self.current,
            });
        }
        if self.version == Version::MAX {
            return Err(BufferError::VersionExhausted);
        }
        let next_timestamp = self
            .next_timestamp
            .checked_add(1)
            .ok_or(BufferError::VersionExhausted)?;

        let mut candidate = self.rope.clone();
        let inverse = Self::apply_edits(&mut candidate, &batch.edits)?;
        Self::validate_cursor(&candidate, batch.after)?;

        let node = self.nodes.len();
        let timestamp = self.next_timestamp;
        self.next_timestamp = next_timestamp;
        self.nodes.push(UndoNode {
            parent: Some(self.current),
            children: Vec::new(),
            forward: batch.edits,
            inverse,
            timestamp,
            before: batch.before,
            after: batch.after,
        });
        self.nodes[self.current].children.push(node);
        self.current = node;
        self.rope = candidate;
        let version = self.increment_version()?;
        Ok(EditOutcome {
            version,
            cursor: batch.after,
            node,
        })
    }

    fn insert(
        &mut self,
        at: usize,
        text: impl Into<String>,
        before: CursorSnapshot,
        after: CursorSnapshot,
    ) -> Result<EditOutcome, BufferError> {
        self.apply_batch(EditBatch {
            edits: vec![Edit::Insert {
                at,
                text: text.into(),
            }],
            before,
            after,
        })
    }

    fn delete(
        &mut self,
        range: Range<usize>,
        before: CursorSnapshot,
        after: CursorSnapshot,
    ) -> Result<EditOutcome, BufferError> {
        self.apply_batch(EditBatch {
            edits: vec![Edit::Delete { range }],
            before,
            after,
        })
    }

    fn undo(&mut self) -> Result<EditOutcome, BufferError> {
        if self.version == Version::MAX {
            return Err(BufferError::VersionExhausted);
        }
        let node = self.current;
        let parent = self.nodes[node].parent.ok_or(BufferError::NoUndo)?;
        let inverse = self.nodes[node].inverse.clone();
        let cursor = self.nodes[node].before;
        self.replay(&inverse)?;
        self.current = parent;
        let version = self.increment_version()?;
        Ok(EditOutcome {
            version,
            cursor,
            node: parent,
        })
    }

    fn redo(&mut self, choice: Option<UndoNodeId>) -> Result<EditOutcome, BufferError> {
        if self.version == Version::MAX {
            return Err(BufferError::VersionExhausted);
        }
        let choices = &self.nodes[self.current].children;
        let node = match (choices.as_slice(), choice) {
            ([], _) => return Err(BufferError::NoRedo),
            ([only], None) => *only,
            ([..], None) => {
                return Err(BufferError::RedoChoiceRequired {
                    choices: choices.clone(),
                })
            }
            (_, Some(node)) if choices.contains(&node) => node,
            (_, Some(node)) => return Err(BufferError::InvalidRedoChoice { node }),
        };
        let forward = self.nodes[node].forward.clone();
        let cursor = self.nodes[node].after;
        self.replay(&forward)?;
        self.current = node;
        let version = self.increment_version()?;
        Ok(EditOutcome {
            version,
            cursor,
            node,
        })
    }

    fn redo_choices(&self) -> &[UndoNodeId] {
        &self.nodes[self.current].children
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cursor(offset: usize) -> CursorSnapshot {
        CursorSnapshot {
            anchor: offset,
            head: offset,
        }
    }

    #[test]
    fn lines_and_utf8_boundaries_are_strict() -> Result<(), BufferError> {
        let buffer = RopeBuffer::new("a\r\nβ\n");
        assert_eq!(buffer.line_count(), 3);
        assert_eq!(buffer.line(0)?, "a\r\n");
        assert_eq!(buffer.line(1)?, "β\n");
        assert_eq!(buffer.line(2)?, "");
        assert_eq!(buffer.char_to_line(3)?, 1);
        assert_eq!(buffer.line_to_char(2)?, 5);
        assert_eq!(buffer.char_to_byte(3)?, 3);
        assert_eq!(buffer.byte_to_char(3)?, 3);
        assert_eq!(
            buffer.byte_to_char(4),
            Err(BufferError::ByteNotCharBoundary { offset: 4 })
        );
        Ok(())
    }

    #[test]
    fn batch_is_atomic_and_one_undo_node() -> Result<(), BufferError> {
        let mut buffer = RopeBuffer::new("abc");
        let outcome = buffer.apply_batch(EditBatch {
            edits: vec![
                Edit::Insert {
                    at: 1,
                    text: "X".to_owned(),
                },
                Edit::Delete { range: 2..3 },
            ],
            before: cursor(1),
            after: cursor(2),
        })?;
        assert_eq!(buffer.text(), "aXc");
        assert_eq!(outcome.version, 1);
        assert_eq!(buffer.undo()?.cursor, cursor(1));
        assert_eq!(buffer.text(), "abc");

        let before = (
            buffer.text(),
            buffer.version(),
            buffer.redo_choices().to_vec(),
        );
        assert!(buffer
            .apply_batch(EditBatch {
                edits: vec![Edit::Delete { range: 0..99 }],
                before: cursor(0),
                after: cursor(0),
            })
            .is_err());
        assert_eq!(
            before,
            (
                buffer.text(),
                buffer.version(),
                buffer.redo_choices().to_vec()
            )
        );
        Ok(())
    }

    #[test]
    fn undo_branches_require_an_explicit_choice() -> Result<(), BufferError> {
        let mut buffer = RopeBuffer::new("");
        let first = buffer.insert(0, "a", cursor(0), cursor(1))?.node;
        buffer.undo()?;
        let second = buffer.insert(0, "b", cursor(0), cursor(1))?.node;
        buffer.undo()?;
        assert_eq!(
            buffer.redo(None),
            Err(BufferError::RedoChoiceRequired {
                choices: vec![first, second]
            })
        );
        assert_eq!(buffer.redo(Some(first))?.version, 5);
        assert_eq!(buffer.text(), "a");
        Ok(())
    }

    #[test]
    fn generated_edit_roundtrips() -> Result<(), BufferError> {
        let mut state = 7_u64;
        for _case in 0..128 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let len = (state as usize % 24) + 1;
            let original = (0..len)
                .map(|index| if index % 5 == 0 { 'λ' } else { 'a' })
                .collect::<String>();
            let mut buffer = RopeBuffer::new(&original);
            let at = state as usize % (buffer.len_chars() + 1);
            let inserted = if state & 1 == 0 { "🦀" } else { "xy" };
            let after = at + inserted.chars().count();
            buffer.insert(at, inserted, cursor(at), cursor(after))?;
            buffer.undo()?;
            assert_eq!(buffer.text(), original);
            buffer.redo(None)?;
            assert_ne!(buffer.text(), original);
        }
        Ok(())
    }

    #[test]
    fn timestamps_are_monotonic() -> Result<(), BufferError> {
        let mut buffer = RopeBuffer::default();
        buffer.insert(0, "a", cursor(0), cursor(1))?;
        buffer.insert(1, "b", cursor(1), cursor(2))?;
        assert!(buffer.undo_timestamp(1) < buffer.undo_timestamp(2));
        Ok(())
    }

    #[test]
    fn version_exhaustion_is_atomic() -> Result<(), BufferError> {
        let mut buffer = RopeBuffer::default();
        buffer.insert(0, "a", cursor(0), cursor(1))?;
        buffer.version = Version::MAX;
        let before = (buffer.text(), buffer.current);
        assert_eq!(buffer.undo(), Err(BufferError::VersionExhausted));
        assert_eq!(before, (buffer.text(), buffer.current));
        Ok(())
    }
}
