use tokio::sync::Mutex;
use tower_lsp::Client;
use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Url};

use crate::position::to_lsp_range;
use crate::state::WorkspaceState;

pub async fn publish(client: &Client, state: &Mutex<WorkspaceState>, uri: Url) {
    let (diagnostics, dependents) = {
        let mut state = state.lock().await;
        let diagnostics = state
            .refresh_analysis(&uri)
            .map(|analysis| analysis.diagnostics)
            .unwrap_or_default();
        (diagnostics, state.dependent_uris(&uri))
    };
    client
        .publish_diagnostics(
            uri,
            diagnostics.into_iter().map(to_lsp_diagnostic).collect(),
            None,
        )
        .await;

    for dependent in dependents {
        let diagnostics = {
            let mut state = state.lock().await;
            state
                .refresh_analysis(&dependent)
                .map(|analysis| analysis.diagnostics)
                .unwrap_or_default()
        };
        client
            .publish_diagnostics(
                dependent,
                diagnostics.into_iter().map(to_lsp_diagnostic).collect(),
                None,
            )
            .await;
    }
}

pub async fn clear(client: &Client, state: &Mutex<WorkspaceState>, uri: Url) {
    state.lock().await.close_document(&uri);
    client.publish_diagnostics(uri, Vec::new(), None).await;
}

fn to_lsp_diagnostic(diagnostic: scon::Diagnostic) -> Diagnostic {
    Diagnostic {
        range: diagnostic
            .range
            .as_ref()
            .map(to_lsp_range)
            .unwrap_or_default(),
        severity: Some(match diagnostic.severity {
            scon::DiagnosticSeverity::Error => DiagnosticSeverity::ERROR,
            scon::DiagnosticSeverity::Warning => DiagnosticSeverity::WARNING,
            scon::DiagnosticSeverity::Information => DiagnosticSeverity::INFORMATION,
            scon::DiagnosticSeverity::Hint => DiagnosticSeverity::HINT,
        }),
        code: Some(NumberOrString::String(format!("{:?}", diagnostic.code))),
        source: Some("scon".to_string()),
        message: diagnostic.message,
        ..Diagnostic::default()
    }
}
