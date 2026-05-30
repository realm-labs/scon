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

#[cfg(test)]
mod tests {
    use tokio::sync::Mutex;
    use tower_lsp::lsp_types::{
        DocumentFormattingParams, FormattingOptions, TextDocumentIdentifier, Url,
        WorkDoneProgressParams,
    };

    use super::*;

    #[tokio::test]
    async fn formatting_returns_parseable_full_document_edit() {
        let uri = Url::parse("file:///tmp/app.scon").unwrap();
        let state = Mutex::new(WorkspaceState::default());
        state.lock().await.open_document(
            uri.clone(),
            r#"
defaults {
host="127.0.0.1"
}
"#
            .to_string(),
        );

        let edits = format_document(
            &state,
            DocumentFormattingParams {
                text_document: TextDocumentIdentifier { uri },
                options: FormattingOptions::default(),
                work_done_progress_params: WorkDoneProgressParams::default(),
            },
        )
        .await
        .unwrap()
        .unwrap();

        assert_eq!(edits.len(), 1);
        scon::parse_source(&edits[0].new_text, scon::ParseOptions::default()).unwrap();
    }
}
