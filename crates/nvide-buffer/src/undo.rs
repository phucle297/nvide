//! Undo tree (linear stack for Phase 0; branching layout reserved).

/// Inverse-op stack that restores prior text after edits.
#[derive(Debug, Default, Clone)]
pub struct UndoTree {
    /// Past states (most recent at the end). Each entry is the inverse of a
    /// committed edit (or a coalesced group).
    undo_stack: Vec<Vec<EditOp>>,
    /// Redo stack.
    redo_stack: Vec<Vec<EditOp>>,
}

/// A single inverse operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditOp {
    /// Inverse of delete: re-insert text at `pos`.
    Insert { pos: usize, text: String },
    /// Inverse of insert: delete `len` chars at `pos`.
    Delete { pos: usize, len: usize },
}

impl UndoTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Record a group of inverse ops (call after applying the forward edit).
    pub fn push(&mut self, inverse: Vec<EditOp>) {
        if inverse.is_empty() {
            return;
        }
        self.undo_stack.push(inverse);
        self.redo_stack.clear();
    }

    /// Pop the last inverse group for undo. Returns `None` if empty.
    pub fn pop_undo(&mut self) -> Option<Vec<EditOp>> {
        self.undo_stack.pop()
    }

    /// After applying undo inverses, push the redo forward ops.
    pub fn push_redo(&mut self, forward: Vec<EditOp>) {
        if !forward.is_empty() {
            self.redo_stack.push(forward);
        }
    }

    /// Pop redo group.
    pub fn pop_redo(&mut self) -> Option<Vec<EditOp>> {
        self.redo_stack.pop()
    }

    /// After applying redo, push inverses back onto undo.
    pub fn push_undo_only(&mut self, inverse: Vec<EditOp>) {
        if !inverse.is_empty() {
            self.undo_stack.push(inverse);
        }
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}
