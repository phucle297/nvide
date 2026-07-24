//! Rope-backed buffer API for NVide (ADR-0005).
//!
//! Core code depends on the [`Buffer`] trait; [`RopeBuffer`] is the default
//! implementation. Undo restores prior text after edits.

mod rope_buf;
mod undo;

pub use rope_buf::RopeBuffer;
pub use undo::{EditOp, UndoTree};

use std::ops::Range;

/// Stable buffer identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BufferId(pub u64);

/// Text encoding for the buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Encoding {
    #[default]
    Utf8,
}

/// Core buffer operations. All editor code should depend on this trait, not a
/// concrete rope type (ADR-0005).
pub trait Buffer {
    /// Total length in Unicode scalar values (chars).
    fn len_chars(&self) -> usize;

    /// True when the buffer has no characters.
    fn is_empty(&self) -> bool {
        self.len_chars() == 0
    }

    /// Total length in bytes (UTF-8).
    fn len_bytes(&self) -> usize;

    /// Number of lines (always ≥ 1 for an empty buffer: one empty line).
    fn line_count(&self) -> usize;

    /// Insert `text` at character index `pos`.
    fn insert(&mut self, pos: usize, text: &str) -> Result<(), BufferError>;

    /// Delete the half-open character range `[start, end)`.
    fn delete(&mut self, range: Range<usize>) -> Result<(), BufferError>;

    /// Return the UTF-8 slice for the character range.
    fn slice(&self, range: Range<usize>) -> Result<String, BufferError>;

    /// Return the contents of `line` (0-based), without the trailing newline.
    fn line(&self, line: usize) -> Result<String, BufferError>;

    /// Map a character index to a byte index.
    fn char_to_byte(&self, char_idx: usize) -> Result<usize, BufferError>;

    /// Map a byte index to a character index.
    fn byte_to_char(&self, byte_idx: usize) -> Result<usize, BufferError>;

    /// Full buffer text (avoid on multi-GB files; fine for tests/prototype).
    fn to_string(&self) -> String;

    /// Monotonic content version (bumped on every edit).
    fn version(&self) -> u64;
}

/// Errors from buffer operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BufferError {
    #[error("character index {index} out of bounds (len {len})")]
    CharOutOfBounds { index: usize, len: usize },
    #[error("byte index {index} out of bounds (len {len})")]
    ByteOutOfBounds { index: usize, len: usize },
    #[error("line {line} out of bounds (line_count {line_count})")]
    LineOutOfBounds { line: usize, line_count: usize },
    #[error("invalid range {start}..{end} (len {len})")]
    InvalidRange {
        start: usize,
        end: usize,
        len: usize,
    },
    #[error("buffer is read-only")]
    ReadOnly,
}
