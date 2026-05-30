use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::{DocumentFormattingParams, Range, TextEdit};

use crate::position::end_position;
use crate::state::WorkspaceState;

pub async fn format_document(
    state: &Mutex<WorkspaceState>,
    params: DocumentFormattingParams,
) -> LspResult<Option<Vec<TextEdit>>> {
    let text = {
        let state = state.lock().await;
        if !state.config.format_enable {
            return Ok(None);
        }
        let Some(text) = state.document_text(&params.text_document.uri) else {
            return Ok(None);
        };
        text
    };
    let formatted = match scon::format_source(&text, scon::FormatOptions::default()) {
        Ok(formatted) => formatted,
        Err(_) => return Ok(None),
    };
    Ok(Some(vec![TextEdit {
        range: Range {
            start: Default::default(),
            end: end_position(&text),
        },
        new_text: formatted,
    }]))
}
