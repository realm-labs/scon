use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::{
    DocumentSymbol, DocumentSymbolParams, DocumentSymbolResponse, SymbolKind,
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
    Ok(Some(DocumentSymbolResponse::Nested(build_tree(
        &analysis.symbols,
        &[],
    ))))
}

fn build_tree(symbols: &[scon::Symbol], parent: &[String]) -> Vec<DocumentSymbol> {
    let depth = parent.len();
    let mut out = Vec::new();
    for symbol in symbols
        .iter()
        .filter(|symbol| symbol.path.len() == depth + 1 && symbol.path.starts_with(parent))
    {
        let range = to_lsp_range(&symbol.range);
        let children = build_tree(symbols, &symbol.path);
        out.push(DocumentSymbol {
            name: symbol.path[depth].clone(),
            detail: Some(symbol.path.join(".")),
            kind: SymbolKind::FIELD,
            tags: None,
            deprecated: None,
            range,
            selection_range: range,
            children: if children.is_empty() {
                None
            } else {
                Some(children)
            },
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_nested_document_symbols() {
        let analysis = scon::analyze_source(
            r#"
server {
  host = "127.0.0.1"
}
"#,
            scon::ParseOptions::default(),
        );

        let symbols = build_tree(&analysis.symbols, &[]);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "server");
        assert_eq!(symbols[0].children.as_ref().unwrap()[0].name, "host");
    }
}
