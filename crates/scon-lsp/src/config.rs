use std::path::PathBuf;

use serde_json::Value;

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub format_enable: bool,
    pub include_root: Option<PathBuf>,
    pub diagnostics_resolve_on_change: bool,
    pub max_file_size: Option<usize>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            format_enable: true,
            include_root: None,
            diagnostics_resolve_on_change: true,
            max_file_size: None,
        }
    }
}

impl ServerConfig {
    pub fn apply_json(&mut self, settings: &Value) {
        let scon = settings.get("scon").unwrap_or(settings);
        if let Some(value) = scon
            .pointer("/format/enable")
            .and_then(Value::as_bool)
            .or_else(|| scon.get("format.enable").and_then(Value::as_bool))
        {
            self.format_enable = value;
        }
        if let Some(value) = scon
            .pointer("/diagnostics/resolveOnChange")
            .and_then(Value::as_bool)
            .or_else(|| {
                scon.get("diagnostics.resolveOnChange")
                    .and_then(Value::as_bool)
            })
        {
            self.diagnostics_resolve_on_change = value;
        }
        if let Some(value) = scon
            .get("includeRoot")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        {
            self.include_root = Some(PathBuf::from(value));
        }
        if let Some(value) = scon.get("includeRoot").and_then(Value::as_str)
            && value.is_empty()
        {
            self.include_root = None;
        }
        if let Some(value) = scon.get("maxFileSize").and_then(Value::as_u64) {
            self.max_file_size = usize::try_from(value).ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_nested_and_flat_json_settings() {
        let mut config = ServerConfig::default();
        config.apply_json(&serde_json::json!({
            "scon": {
                "format": { "enable": false },
                "diagnostics.resolveOnChange": false,
                "includeRoot": "/tmp/scon",
                "maxFileSize": 1024
            }
        }));

        assert!(!config.format_enable);
        assert!(!config.diagnostics_resolve_on_change);
        assert_eq!(config.include_root, Some(PathBuf::from("/tmp/scon")));
        assert_eq!(config.max_file_size, Some(1024));
    }
}
