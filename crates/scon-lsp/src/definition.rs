use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Location, Range, Url};

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

    if let Some(include) = analysis
        .includes
        .iter()
        .find(|include| contains_byte(&include.range, byte))
        && let Some(target_uri) = include
            .resolved_path
            .as_deref()
            .and_then(|path| Url::from_file_path(path).ok())
    {
        return Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri: target_uri,
            range: Range::default(),
        })));
    }

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

#[cfg(test)]
mod tests {
    use tokio::sync::Mutex;
    use tower_lsp::lsp_types::{
        GotoDefinitionParams, PartialResultParams, Position, TextDocumentIdentifier,
        TextDocumentPositionParams, WorkDoneProgressParams,
    };

    use super::*;
    use crate::state::WorkspaceState;

    #[tokio::test]
    async fn include_path_goes_to_included_file() {
        let root = std::env::temp_dir().join(format!(
            "scon-lsp-definition-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let app = root.join("app.scon");
        let base = root.join("base.scon");
        let app_uri = Url::from_file_path(&app).unwrap();
        let base_uri = Url::from_file_path(&base).unwrap();
        let state = Mutex::new(WorkspaceState::default());
        state
            .lock()
            .await
            .open_document(app_uri.clone(), "include \"./base.scon\"\n".to_string());
        state
            .lock()
            .await
            .open_document(base_uri.clone(), "x = 1\n".to_string());

        let response = goto_definition(
            &state,
            GotoDefinitionParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri: app_uri },
                    position: Position {
                        line: 0,
                        character: 12,
                    },
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            },
        )
        .await
        .unwrap()
        .unwrap();

        let GotoDefinitionResponse::Scalar(location) = response else {
            panic!("expected scalar definition response");
        };
        assert_eq!(location.uri, base_uri);
    }
}
