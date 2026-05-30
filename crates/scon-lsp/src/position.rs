use tower_lsp::lsp_types::{Position, Range};

pub fn to_lsp_range(range: &scon::SourceRange) -> Range {
    Range {
        start: Position {
            line: range.start.line as u32,
            character: range.start.character as u32,
        },
        end: Position {
            line: range.end.line as u32,
            character: range.end.character as u32,
        },
    }
}

pub fn end_position(text: &str) -> Position {
    let index = scon::LineIndex::new(text);
    let position = index.utf16_position(text, text.len());
    Position {
        line: position.line as u32,
        character: position.character as u32,
    }
}

pub fn byte_at_position(
    analysis: &scon::AnalyzedDocument,
    text: &str,
    position: Position,
) -> Option<usize> {
    let parsed = analysis.parsed.as_ref()?;
    parsed.line_index.byte_for_utf16_position(
        text,
        position.line as usize,
        position.character as usize,
    )
}

pub fn contains_byte(range: &scon::SourceRange, byte: usize) -> bool {
    range.start.byte <= byte && byte <= range.end.byte
}
