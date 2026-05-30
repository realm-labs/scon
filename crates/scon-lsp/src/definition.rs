use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Location, Url};

use crate::position::{byte_at_position, contains_byte, to_lsp_range};
use crate::state::WorkspaceState;

pub async fn goto_definition(
    state: &Mutex<WorkspaceState>,
    params: GotoDefinitionParams,
) -> LspResult<Option<GotoDefinitionResponse>> {
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
    let Some(reference) = analysis
        .references
        .iter()
        .find(|reference| contains_byte(&reference.range, byte))
    else {
        return Ok(None);
    };
    let Some(target) = &reference.target else {
        return Ok(None);
    };
    let target_uri = target
        .file
        .as_deref()
        .and_then(|path| Url::from_file_path(path).ok())
        .unwrap_or(uri);
    Ok(Some(GotoDefinitionResponse::Scalar(Location {
        uri: target_uri,
        range: to_lsp_range(&target.range),
    })))
}
