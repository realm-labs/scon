use std::collections::HashSet;

use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse,
};

use crate::state::WorkspaceState;

pub async fn complete(
    state: &Mutex<WorkspaceState>,
    params: CompletionParams,
) -> LspResult<Option<CompletionResponse>> {
    let uri = params.text_document_position.text_document.uri;
    let state = state.lock().await;
    let Some(analysis) = state.analyze_uri(&uri) else {
        return Ok(None);
    };
    let mut seen = HashSet::new();
    let mut items = analysis
        .symbols
        .into_iter()
        .filter_map(|symbol| symbol.path.last().cloned())
        .filter(|label| seen.insert(label.clone()))
        .map(|label| CompletionItem {
            label,
            kind: Some(CompletionItemKind::FIELD),
            ..CompletionItem::default()
        })
        .collect::<Vec<_>>();
    items.push(CompletionItem {
        label: "include".to_string(),
        kind: Some(CompletionItemKind::KEYWORD),
        ..CompletionItem::default()
    });
    Ok(Some(CompletionResponse::Array(items)))
}
