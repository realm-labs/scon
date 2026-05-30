#![allow(deprecated)]

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
    documents: Arc<Mutex<HashMap<Url, String>>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> LspResult<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                document_formatting_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions::default()),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "scon-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "scon-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.documents.lock().await.insert(uri.clone(), text);
        self.publish_diagnostics(uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().next() {
            self.documents.lock().await.insert(uri.clone(), change.text);
        }
        self.publish_diagnostics(uri).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.publish_diagnostics(params.text_document.uri).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents
            .lock()
            .await
            .remove(&params.text_document.uri);
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
    ) -> LspResult<Option<Vec<TextEdit>>> {
        let Some(text) = self.document_text(&params.text_document.uri).await else {
            return Ok(None);
        };
        let formatted = match scon::format_source(&text, scon::FormatOptions::default()) {
            Ok(formatted) => formatted,
            Err(_) => return Ok(None),
        };
        let range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: end_position(&text),
        };
        Ok(Some(vec![TextEdit {
            range,
            new_text: formatted,
        }]))
    }

    async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let Some(text) = self.document_text(&uri).await else {
            return Ok(None);
        };
        let word = word_at(&text, position);
        if word.is_empty() {
            return Ok(None);
        }
        let contents = HoverContents::Scalar(MarkedString::String(format!("SCON path `{word}`")));
        Ok(Some(Hover {
            contents,
            range: None,
        }))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> LspResult<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let Some(text) = self.document_text(&uri).await else {
            return Ok(None);
        };
        let word = word_at(&text, position);
        let analysis = scon::analyze_source(&text, scon::ParseOptions::default());
        let Some(symbol) = analysis
            .symbols
            .iter()
            .find(|symbol| symbol.path.last().is_some_and(|name| name == &word))
        else {
            return Ok(None);
        };
        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri,
            range: to_lsp_range(&symbol.range),
        })))
    }

    async fn completion(&self, params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let Some(text) = self.document_text(&uri).await else {
            return Ok(None);
        };
        let mut items = scon::analyze_source(&text, scon::ParseOptions::default())
            .symbols
            .into_iter()
            .filter_map(|symbol| symbol.path.last().cloned())
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

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> LspResult<Option<DocumentSymbolResponse>> {
        let Some(text) = self.document_text(&params.text_document.uri).await else {
            return Ok(None);
        };
        let symbols = scon::analyze_source(&text, scon::ParseOptions::default())
            .symbols
            .into_iter()
            .map(|symbol| SymbolInformation {
                name: symbol.path.join("."),
                kind: SymbolKind::FIELD,
                tags: None,
                location: Location {
                    uri: params.text_document.uri.clone(),
                    range: to_lsp_range(&symbol.range),
                },
                container_name: None,
                deprecated: None,
            })
            .collect();
        Ok(Some(DocumentSymbolResponse::Flat(symbols)))
    }
}

impl Backend {
    async fn document_text(&self, uri: &Url) -> Option<String> {
        if let Some(text) = self.documents.lock().await.get(uri).cloned() {
            return Some(text);
        }
        uri.to_file_path()
            .ok()
            .and_then(|path| std::fs::read_to_string(path).ok())
    }

    async fn publish_diagnostics(&self, uri: Url) {
        let Some(text) = self.document_text(&uri).await else {
            return;
        };
        let diagnostics = scon::analyze_source(&text, scon::ParseOptions::default())
            .diagnostics
            .into_iter()
            .map(to_lsp_diagnostic)
            .collect();
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| Backend {
        client,
        documents: Arc::new(Mutex::new(HashMap::new())),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}

fn to_lsp_diagnostic(diagnostic: scon::Diagnostic) -> Diagnostic {
    Diagnostic {
        range: diagnostic
            .range
            .as_ref()
            .map(to_lsp_range)
            .unwrap_or_default(),
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(format!("{:?}", diagnostic.code))),
        source: Some("scon".to_string()),
        message: diagnostic.message,
        ..Diagnostic::default()
    }
}

fn to_lsp_range(range: &scon::SourceRange) -> Range {
    Range {
        start: Position {
            line: range.start.line as u32,
            character: range.start.character as u32,
        },
        end: Position {
            line: range.end.line as u32,
            character: range.end.character as u32,
        },
    }
}

fn end_position(text: &str) -> Position {
    let mut line = 0u32;
    let mut character = 0u32;
    for ch in text.chars() {
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }
    Position { line, character }
}

fn word_at(text: &str, position: Position) -> String {
    let Some(line) = text.lines().nth(position.line as usize) else {
        return String::new();
    };
    let chars = line.chars().collect::<Vec<_>>();
    let mut index = (position.character as usize).min(chars.len());
    while index > 0 && is_word_char(chars[index - 1]) {
        index -= 1;
    }
    let start = index;
    while index < chars.len() && is_word_char(chars[index]) {
        index += 1;
    }
    chars[start..index].iter().collect()
}

fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}
