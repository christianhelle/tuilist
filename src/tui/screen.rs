//! The cursor's shape: a bar while typing, a block in vim's other modes.

/// The cursor's shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cursor {
    Default,
    Bar,
    Block,
}

#[cfg(test)]
mod tests {
    use super::Cursor;

    #[test]
    fn the_cursor_shapes_are_distinct() {
        assert_ne!(Cursor::Default, Cursor::Bar);
        assert_ne!(Cursor::Bar, Cursor::Block);
    }
}
