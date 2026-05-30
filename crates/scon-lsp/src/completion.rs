use std::collections::HashSet;
use std::fs;

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
    let position = params.text_document_position.position;
    let state = state.lock().await;
    let Some(analysis) = state.analyze_uri(&uri) else {
        return Ok(None);
    };
    let byte = state
        .document_text(&uri)
        .and_then(|text| crate::position::byte_at_position(&analysis, &text, position));

    if let Some(byte) = byte {
        if analysis
            .includes
            .iter()
            .any(|include| crate::position::contains_byte(&include.range, byte))
        {
            return Ok(Some(CompletionResponse::Array(include_path_items(&uri))));
        }
        if analysis
            .references
            .iter()
            .any(|reference| crate::position::contains_byte(&reference.range, byte))
        {
            return Ok(Some(CompletionResponse::Array(path_items(&analysis))));
        }
    }

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
    for label in ["true", "false", "null"] {
        items.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..CompletionItem::default()
        });
    }
    Ok(Some(CompletionResponse::Array(items)))
}

fn path_items(analysis: &scon::AnalyzedDocument) -> Vec<CompletionItem> {
    let mut seen = HashSet::new();
    analysis
        .definitions
        .iter()
        .map(|definition| definition.path.join("."))
        .filter(|label| seen.insert(label.clone()))
        .map(|label| CompletionItem {
            label,
            kind: Some(CompletionItemKind::REFERENCE),
            ..CompletionItem::default()
        })
        .collect()
}

fn include_path_items(uri: &tower_lsp::lsp_types::Url) -> Vec<CompletionItem> {
    let Some(dir) = uri
        .to_file_path()
        .ok()
        .and_then(|path| path.parent().map(std::path::Path::to_path_buf))
    else {
        return Vec::new();
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "scon")
            {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(str::to_string)
            } else {
                None
            }
        })
        .map(|label| CompletionItem {
            label,
            kind: Some(CompletionItemKind::FILE),
            ..CompletionItem::default()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_completion_uses_definition_paths() {
        let analysis = scon::analyze_source(
            r#"
defaults {
  host = "127.0.0.1"
}
"#,
            scon::ParseOptions::default(),
        );

        let labels = path_items(&analysis)
            .into_iter()
            .map(|item| item.label)
            .collect::<Vec<_>>();
        assert!(labels.contains(&"defaults".to_string()));
        assert!(labels.contains(&"defaults.host".to_string()));
    }
}
