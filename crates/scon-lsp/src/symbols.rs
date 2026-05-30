use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::{
    DocumentSymbolParams, DocumentSymbolResponse, Location, SymbolInformation, SymbolKind,
};

use crate::position::to_lsp_range;
use crate::state::WorkspaceState;

pub async fn document_symbols(
    state: &Mutex<WorkspaceState>,
    params: DocumentSymbolParams,
) -> LspResult<Option<DocumentSymbolResponse>> {
    let uri = params.text_document.uri;
    let state = state.lock().await;
    let Some(analysis) = state.analyze_uri(&uri) else {
        return Ok(None);
    };
    let symbols = analysis
        .symbols
        .into_iter()
        .map(|symbol| SymbolInformation {
            name: symbol.path.join("."),
            kind: SymbolKind::FIELD,
            tags: None,
            location: Location {
                uri: symbol
                    .file
                    .as_deref()
                    .and_then(|path| tower_lsp::lsp_types::Url::from_file_path(path).ok())
                    .unwrap_or_else(|| uri.clone()),
                range: to_lsp_range(&symbol.range),
            },
            container_name: None,
            deprecated: None,
        })
        .collect();
    Ok(Some(DocumentSymbolResponse::Flat(symbols)))
}
