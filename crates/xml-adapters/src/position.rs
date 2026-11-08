use std::fmt;

/// A place in the XML source text.
///
/// `line` and `column` count from 1 (columns in characters, as `roxmltree`
/// does); `offset` is the byte offset from 0 of the `<` of the start tag.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Position {
    /// Line, from 1.
    pub line: u32,
    /// Column in characters, from 1.
    pub column: u32,
    /// Byte offset into the input, from 0.
    pub offset: usize,
}

impl Position {
    pub(crate) fn of(node: roxmltree::Node<'_, '_>) -> Self {
        let offset = node.range().start;
        let at = node.document().text_pos_at(offset);
        Position {
            line: at.row,
            column: at.col,
            offset,
        }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}
