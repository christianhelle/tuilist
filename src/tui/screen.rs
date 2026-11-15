//! The cursor's shape: a bar while typing, a block in vim's other modes.

/// The cursor's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cursor {
    Default,
    Bar,
    Block,
}
