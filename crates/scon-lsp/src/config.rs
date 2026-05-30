use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub format_enable: bool,
    pub include_root: Option<PathBuf>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            format_enable: true,
            include_root: None,
        }
    }
}
