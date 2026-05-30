use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use tower_lsp::lsp_types::Url;

use crate::config::ServerConfig;

#[derive(Debug, Default)]
pub struct DocumentStore {
    open_documents: HashMap<Url, String>,
}

impl DocumentStore {
    pub fn open(&mut self, uri: Url, text: String) {
        self.open_documents.insert(uri, text);
    }

    pub fn change(&mut self, uri: Url, text: String) {
        self.open_documents.insert(uri, text);
    }

    pub fn close(&mut self, uri: &Url) {
        self.open_documents.remove(uri);
    }

    pub fn text(&self, uri: &Url) -> Option<String> {
        if let Some(text) = self.open_documents.get(uri) {
            return Some(text.clone());
        }
        uri.to_file_path()
            .ok()
            .and_then(|path| fs::read_to_string(path).ok())
    }

    fn text_for_path(&self, path: &Path) -> std::io::Result<Option<String>> {
        if let Ok(uri) = Url::from_file_path(path)
            && let Some(text) = self.open_documents.get(&uri)
        {
            return Ok(Some(text.clone()));
        }
        match fs::read_to_string(path) {
            Ok(text) => Ok(Some(text)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }
}

impl scon::SourceStore for DocumentStore {
    fn read_source(&self, path: &Path) -> std::io::Result<Option<String>> {
        self.text_for_path(path)
    }
}

#[derive(Debug, Default)]
pub struct WorkspaceState {
    pub config: ServerConfig,
    documents: DocumentStore,
    include_graph: HashMap<Url, HashSet<Url>>,
    reverse_include_graph: HashMap<Url, HashSet<Url>>,
    diagnostics: HashMap<Url, Vec<scon::Diagnostic>>,
}

impl WorkspaceState {
    pub fn open_document(&mut self, uri: Url, text: String) {
        self.documents.open(uri, text);
    }

    pub fn change_document(&mut self, uri: Url, text: String) {
        self.documents.change(uri, text);
    }

    pub fn close_document(&mut self, uri: &Url) {
        self.documents.close(uri);
        self.include_graph.remove(uri);
        self.diagnostics.remove(uri);
        for includes in self.reverse_include_graph.values_mut() {
            includes.remove(uri);
        }
    }

    pub fn document_text(&self, uri: &Url) -> Option<String> {
        self.documents.text(uri)
    }

    pub fn analyze_uri(&self, uri: &Url) -> Option<scon::AnalyzedDocument> {
        let text = self.document_text(uri)?;
        if let Ok(path) = uri.to_file_path() {
            let options = scon::LoadOptions {
                include_root: self.config.include_root.clone(),
                ..scon::LoadOptions::default()
            };
            Some(scon::analyze_file_with_store(
                path,
                options,
                &self.documents,
            ))
        } else {
            Some(scon::analyze_source(&text, scon::ParseOptions::default()))
        }
    }

    pub fn refresh_analysis(&mut self, uri: &Url) -> Option<scon::AnalyzedDocument> {
        let analysis = self.analyze_uri(uri)?;
        self.update_include_graph(uri, &analysis);
        self.diagnostics
            .insert(uri.clone(), analysis.diagnostics.clone());
        Some(analysis)
    }

    pub fn dependent_uris(&self, uri: &Url) -> Vec<Url> {
        self.reverse_include_graph
            .get(uri)
            .map(|uris| uris.iter().cloned().collect())
            .unwrap_or_default()
    }

    fn update_include_graph(&mut self, uri: &Url, analysis: &scon::AnalyzedDocument) {
        if let Some(previous) = self.include_graph.remove(uri) {
            for included in previous {
                if let Some(reverse) = self.reverse_include_graph.get_mut(&included) {
                    reverse.remove(uri);
                }
            }
        }

        let includes = analysis
            .includes
            .iter()
            .filter_map(|include| include.resolved_path.as_deref())
            .filter_map(path_to_uri)
            .collect::<HashSet<_>>();

        for included in &includes {
            self.reverse_include_graph
                .entry(included.clone())
                .or_default()
                .insert(uri.clone());
        }
        self.include_graph.insert(uri.clone(), includes);
    }
}

fn path_to_uri(path: &Path) -> Option<Url> {
    Url::from_file_path(PathBuf::from(path)).ok()
}
