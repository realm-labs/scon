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

    if let Some(diagnostic) = analysis.diagnostics.iter().find(|diagnostic| {
        diagnostic
            .range
            .as_ref()
            .is_some_and(|range| contains_byte(range, byte))
    }) {
        return Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(format!(
                "{:?}: {}",
                diagnostic.code, diagnostic.message
            ))),
            range: diagnostic.range.as_ref().map(crate::position::to_lsp_range),
        }));
    }

    if let Some(reference) = analysis
        .references
        .iter()
        .find(|reference| contains_byte(&reference.range, byte))
    {
        let preview = reference
            .target
            .as_ref()
            .and_then(|target| value_preview(&analysis, &target.path))
            .map(|preview| format!(" = {preview}"))
            .unwrap_or_default();
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
                target + &preview
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
            "SCON field `{}`{}",
            definition.path.join("."),
            value_preview(&analysis, &definition.path)
                .map(|preview| format!(" = {preview}"))
                .unwrap_or_default()
        ))),
        range: None,
    }))
}

fn value_preview(analysis: &scon::AnalyzedDocument, path: &[String]) -> Option<String> {
    let value = analysis.value.as_ref()?;
    let value = scon::get_path(value, &path.join(".")).ok()?;
    Some(format!(
        "{} {}",
        value_type(value),
        scon::to_string_fragment(value)
    ))
}

fn value_type(value: &scon::Value) -> &'static str {
    match value {
        scon::Value::Null => "null",
        scon::Value::Bool(_) => "bool",
        scon::Value::Number(_) => "number",
        scon::Value::String(_) => "string",
        scon::Value::Array(_) => "array",
        scon::Value::Object(_) => "object",
    }
}
