use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::{Hover, HoverContents, HoverParams, MarkedString};

use crate::position::{byte_at_position, contains_byte};
use crate::state::WorkspaceState;

pub async fn hover(state: &Mutex<WorkspaceState>, params: HoverParams) -> LspResult<Option<Hover>> {
    let uri = params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;
    let state = state.lock().await;
    let Some(text) = state.document_text(&uri) else {
        return Ok(None);
    };
    let Some(analysis) = state.analyze_uri(&uri) else {
        return Ok(None);
    };
    let Some(byte) = byte_at_position(&analysis, &text, position) else {
        return Ok(None);
    };

    if let Some(reference) = analysis
        .references
        .iter()
        .find(|reference| contains_byte(&reference.range, byte))
    {
        let target = reference
            .target
            .as_ref()
            .map(|target| format!(" -> `{}`", target.path.join(".")))
            .unwrap_or_default();
        return Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(format!(
                "SCON {:?} `{}`{}",
                reference.kind,
                reference.path.join("."),
                target
            ))),
            range: None,
        }));
    }

    let Some(definition) = analysis
        .definitions
        .iter()
        .find(|definition| contains_byte(&definition.range, byte))
    else {
        return Ok(None);
    };
    Ok(Some(Hover {
        contents: HoverContents::Scalar(MarkedString::String(format!(
            "SCON field `{}`",
            definition.path.join(".")
        ))),
        range: None,
    }))
}
