use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Limits {
    pub max_file_size: usize,
    pub max_include_depth: usize,
    pub max_include_files: usize,
    pub max_array_length: usize,
    pub max_object_depth: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_file_size: 16 * 1024 * 1024,
            max_include_depth: 64,
            max_include_files: 1024,
            max_array_length: 1_000_000,
            max_object_depth: 512,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct LoadOptions {
    pub include_root: Option<PathBuf>,
    pub limits: Limits,
}
