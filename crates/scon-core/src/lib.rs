#![allow(clippy::result_large_err)]

mod analysis;
mod ast;
mod de;
mod error;
mod eval;
mod format;
mod limits;
mod loader;
mod parser;
mod ser;
mod source;
mod value;

use std::path::Path;

pub use analysis::{
    Analysis, Comment, CommentKind, Diagnostic, FormatOptions, ParseOptions, ParsedDocument,
    SourcePosition, SourceRange, Symbol, Utf16Position, analyze_file, analyze_source,
    diagnostic_from_error, format_source, get_path, parse_source, resolve_file, resolve_source,
};
pub use error::{Error, ErrorCode, Result};
pub use limits::{Limits, LoadOptions};
pub use source::{LineIndex, Token, TokenKind};
pub use value::Value;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn from_str<T>(source: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let doc = parser::parse_str(source, None)?;
    let resolved = eval::eval_resolved_document(doc, &mut loader::NoopLoader)?;
    de::from_resolved(&resolved)
}

pub fn from_file<T>(path: impl AsRef<Path>) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    from_file_with_options(path, LoadOptions::default())
}

pub fn to_string<T>(value: &T) -> Result<String>
where
    T: serde::Serialize,
{
    let value = to_value(value)?;
    format::to_string(&value)
}

pub fn to_string_value(value: &Value) -> Result<String> {
    format::to_string(value)
}

pub fn to_string_fragment(value: &Value) -> String {
    format::to_fragment_string(value)
}

pub fn parse_str(source: &str) -> Result<Value> {
    let doc = parser::parse_str(source, None)?;
    eval::eval_document(doc, &mut loader::NoopLoader)
}

pub fn parse_file(path: impl AsRef<Path>) -> Result<Value> {
    parse_file_with_options(path, LoadOptions::default())
}

pub fn parse_file_with_options(path: impl AsRef<Path>, options: LoadOptions) -> Result<Value> {
    let mut loader = loader::FileLoader::new(path.as_ref(), options)?;
    let doc = loader.load_entry()?;
    eval::eval_document(doc, &mut loader)
}

pub fn from_file_with_options<T>(path: impl AsRef<Path>, options: LoadOptions) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let mut loader = loader::FileLoader::new(path.as_ref(), options)?;
    let doc = loader.load_entry()?;
    let resolved = eval::eval_resolved_document(doc, &mut loader)?;
    de::from_resolved(&resolved)
}

pub fn to_value<T>(value: &T) -> Result<Value>
where
    T: serde::Serialize,
{
    value.serialize(ser::Serializer)
}

pub fn from_value<T>(value: Value) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    T::deserialize(de::Deserializer::new(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use serde::{Deserialize, Serialize};
    use std::fs;

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Server {
        host: String,
        port: u16,
        tls: Option<bool>,
    }

    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct App {
        name: String,
        server: Server,
        paths: Vec<String>,
    }

    #[test]
    fn deserializes_structs_from_resolved_scon() {
        let app: App = from_str(
            r#"
            defaults {
              host = "127.0.0.1"
              port = 8080
            }

            name = "demo"
            server {
              ...${defaults}
              port = 9090
              tls = true
            }
            base_paths = ["/bin", "/usr/bin"]
            paths = [...${base_paths}, "/opt/bin"]
            "#,
        )
        .unwrap();

        assert_eq!(
            app,
            App {
                name: "demo".to_string(),
                server: Server {
                    host: "127.0.0.1".to_string(),
                    port: 9090,
                    tls: Some(true),
                },
                paths: vec![
                    "/bin".to_string(),
                    "/usr/bin".to_string(),
                    "/opt/bin".to_string()
                ],
            }
        );
    }

    #[test]
    fn serializes_and_round_trips_structs() {
        let app = App {
            name: "demo".to_string(),
            server: Server {
                host: "127.0.0.1".to_string(),
                port: 8080,
                tls: None,
            },
            paths: vec!["/bin".to_string()],
        };

        let text = to_string(&app).unwrap();
        assert!(text.contains("name = \"demo\""));
        assert!(text.ends_with('\n'));

        let round_trip: App = from_str(&text).unwrap();
        assert_eq!(round_trip, app);
    }

    #[test]
    fn parse_str_returns_value() {
        let value = parse_str(r#"a.b = 1"#).unwrap();
        let mut b = IndexMap::new();
        b.insert("b".to_string(), Value::Number("1".to_string()));
        let mut a = IndexMap::new();
        a.insert("a".to_string(), Value::Object(b));
        assert_eq!(value, Value::Object(a));
    }

    #[test]
    fn rejects_forward_references() {
        let err = parse_str(
            r#"
            url = "http://${host}"
            host = "127.0.0.1"
            "#,
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::MissingReference);
    }

    #[test]
    fn rejects_space_separated_object_members() {
        let err = parse_str(
            r#"
            a = 1 b = 2
            "#,
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::UnexpectedToken);
    }

    #[test]
    fn accepts_crlf_and_rejects_standalone_cr() {
        let value = parse_str("a = 1\r\nb = 2\r\n").unwrap();
        let Value::Object(root) = value else {
            panic!("expected object root");
        };
        assert_eq!(root["a"], Value::Number("1".to_string()));
        assert_eq!(root["b"], Value::Number("2".to_string()));

        let err = parse_str("a = 1\rb = 2").unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidCharacter);

        let err = parse_str("a = \r\n1").unwrap_err();
        assert_eq!(err.code, ErrorCode::UnexpectedToken);
    }

    #[test]
    fn format_source_preserves_comments_and_composition() {
        let formatted = format_source(
            r#"
# leading
defaults { // inline
host="127.0.0.1"
}
server {
...${defaults}
url="http://${defaults.host}"
}
"#,
            FormatOptions::default(),
        )
        .unwrap();

        assert!(formatted.contains("# leading"));
        assert!(formatted.contains("// inline"));
        assert!(formatted.contains("...${defaults}"));
        assert!(formatted.contains("url = \"http://${defaults.host}\""));
    }

    #[test]
    fn line_index_maps_utf8_and_utf16_positions() {
        let source = "name = \"\u{1F980}\"\nnext = 1\n";
        let index = LineIndex::new(source);
        let crab = '\u{1F980}';
        let crab_byte = source.find(crab).unwrap();

        let source_position = index.source_position(source, crab_byte);
        assert_eq!(source_position.line, 0);
        assert_eq!(source_position.character, 8);
        assert_eq!(index.byte_for_line_character(source, 0, 8), Some(crab_byte));

        let after_crab = crab_byte + crab.len_utf8();
        let utf16 = index.utf16_position(source, after_crab);
        assert_eq!(utf16.line, 0);
        assert_eq!(utf16.character, 10);
        assert_eq!(
            index.byte_for_utf16_position(source, 0, 10),
            Some(after_crab)
        );
        assert_eq!(index.byte_for_utf16_position(source, 0, 9), None);
    }

    #[test]
    fn parse_source_exposes_parser_backed_comments_tokens_and_symbols() {
        let parsed = parse_source(
            r#"
# leading
server {
  host = "127.0.0.1" // inline
}
"#,
            ParseOptions::default(),
        )
        .unwrap();

        assert_eq!(parsed.comments.len(), 2);
        assert!(
            parsed
                .tokens
                .iter()
                .any(|token| matches!(token.kind, TokenKind::Comment(_)))
        );
        assert!(
            parsed
                .symbols
                .iter()
                .any(|symbol| symbol.path == vec!["server"])
        );
        assert!(
            parsed
                .symbols
                .iter()
                .any(|symbol| symbol.path == vec!["server", "host"])
        );
    }

    #[test]
    fn to_string_requires_object_root() {
        let err = format::to_string(&Value::Array(vec![])).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidRootType);
    }

    #[test]
    fn parse_file_evaluates_includes_in_source_order() {
        let dir = std::env::temp_dir().join(format!(
            "scon-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("base.scon"),
            r#"
            server {
              host = ${defaults.host}
              port = 8080
            }
            "#,
        )
        .unwrap();
        fs::write(
            dir.join("app.scon"),
            r#"
            defaults {
              host = "0.0.0.0"
            }
            include "./base.scon"
            "#,
        )
        .unwrap();

        let value = parse_file(dir.join("app.scon")).unwrap();
        let Value::Object(root) = value else {
            panic!("expected object root");
        };
        let Value::Object(server) = &root["server"] else {
            panic!("expected server object");
        };
        assert_eq!(server["host"], Value::String("0.0.0.0".to_string()));

        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn serialization_rejects_non_string_map_keys_and_non_finite_floats() {
        use std::collections::BTreeMap;

        let mut map = BTreeMap::new();
        map.insert(1u8, "bad");
        assert_eq!(to_value(&map).unwrap_err().code, ErrorCode::Serde);

        let value = f64::NAN;
        assert_eq!(to_value(&value).unwrap_err().code, ErrorCode::Serde);
    }

    #[test]
    fn quoted_keys_and_interpolation_escapes_round_trip() {
        let mut root = IndexMap::new();
        root.insert(
            "read.only".to_string(),
            Value::String("${literal}".to_string()),
        );
        let text = format::to_string(&Value::Object(root)).unwrap();
        assert!(text.contains("\"read.only\""));
        assert!(text.contains("\"\\${literal}\""));

        let parsed = parse_str(&text).unwrap();
        let Value::Object(root) = parsed else {
            panic!("expected object root");
        };
        assert_eq!(root["read.only"], Value::String("${literal}".to_string()));
    }
}
