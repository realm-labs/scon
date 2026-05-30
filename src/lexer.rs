use std::path::PathBuf;

use crate::ast::Location;
use crate::error::{Error, ErrorCode, Result};

pub(crate) fn normalize_source(source: &str, file: Option<PathBuf>) -> Result<String> {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut line = 1;
    let mut column = 1;
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                    out.push('\n');
                    line += 1;
                    column = 1;
                } else {
                    return Err(Error::new(
                        ErrorCode::InvalidCharacter,
                        "standalone CR is invalid",
                    )
                    .at(Location { file, line, column }));
                }
            }
            _ => {
                out.push(ch);
                if ch == '\n' {
                    line += 1;
                    column = 1;
                } else {
                    column += 1;
                }
            }
        }
    }
    Ok(out)
}
