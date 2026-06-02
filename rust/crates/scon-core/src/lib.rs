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
    AnalyzedDocument, Comment, CommentKind, Definition, DefinitionKind, Diagnostic,
    DiagnosticRelatedInformation, DiagnosticSeverity, FileSourceStore, FormatOptions,
    IncludeReference, ParseOptions, ParsedDocument, Reference, ReferenceKind, SourcePosition,
    SourceRange, SourceStore, Symbol, Utf16Position, analyze_file, analyze_file_with_store,
    analyze_source, diagnostic_from_error, format_source, get_path, parse_source, resolve_file,
    resolve_source,
};
pub use error::{Error, ErrorCode, Result};
pub use limits::{Limits, LoadOptions};
pub use source::{LineIndex, Token, TokenKind};
pub use value::{Number, Value};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[doc(hidden)]
pub mod __private {
    use indexmap::IndexMap;
    use serde::Serialize;

    use crate::Value;

    pub fn new_object() -> IndexMap<String, Value> {
        IndexMap::new()
    }

    pub fn insert_object(object: &mut IndexMap<String, Value>, key: String, value: Value) {
        object.insert(key, value);
    }

    pub fn key_to_string<T>(key: &T) -> String
    where
        T: Serialize,
    {
        match to_value(key) {
            Value::String(value) => value,
            other => panic!(
                "scon! object keys must serialize as strings, found {}",
                type_name(&other)
            ),
        }
    }

    pub fn to_value<T>(value: &T) -> Value
    where
        T: Serialize,
    {
        crate::to_value(value).expect("scon! failed to serialize interpolated value")
    }

    fn type_name(value: &Value) -> &'static str {
        match value {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }
}

/// Construct a [`Value`] from a SCON-like literal.
///
/// ```
/// # let port = 8080u64;
/// let value = scon::scon!({
///     name: "demo",
///     "read.only": true,
///     port: port,
///     paths: ["/bin", "/usr/bin"],
///     tls: null,
/// });
/// # assert!(matches!(value, scon::Value::Object(_)));
/// ```
///
/// Identifier object keys are stringified, string literal keys are accepted
/// directly, and parenthesized keys are serialized dynamically. Interpolated
/// Rust expressions must implement [`serde::Serialize`]. Serialization failures
/// panic, matching the behavior of `serde_json::json!`.
#[macro_export]
macro_rules! scon {
    (@array [$($elems:expr,)*]) => {
        vec![$($elems,)*]
    };

    (@array [$($elems:expr),*]) => {
        vec![$($elems),*]
    };

    (@array [$($elems:expr,)*] null $($rest:tt)*) => {
        $crate::scon!(@array [$($elems,)* $crate::scon!(@value null)] $($rest)*)
    };

    (@array [$($elems:expr,)*] true $($rest:tt)*) => {
        $crate::scon!(@array [$($elems,)* $crate::scon!(@value true)] $($rest)*)
    };

    (@array [$($elems:expr,)*] false $($rest:tt)*) => {
        $crate::scon!(@array [$($elems,)* $crate::scon!(@value false)] $($rest)*)
    };

    (@array [$($elems:expr,)*] [$($array:tt)*] $($rest:tt)*) => {
        $crate::scon!(@array [$($elems,)* $crate::scon!(@value [$($array)*])] $($rest)*)
    };

    (@array [$($elems:expr,)*] {$($object:tt)*} $($rest:tt)*) => {
        $crate::scon!(@array [$($elems,)* $crate::scon!(@value {$($object)*})] $($rest)*)
    };

    (@array [$($elems:expr,)*] $next:expr, $($rest:tt)*) => {
        $crate::scon!(@array [$($elems,)* $crate::scon!(@value $next),] $($rest)*)
    };

    (@array [$($elems:expr,)*] $last:expr) => {
        $crate::scon!(@array [$($elems,)* $crate::scon!(@value $last)])
    };

    (@array [$($elems:expr),*] , $($rest:tt)*) => {
        $crate::scon!(@array [$($elems,)*] $($rest)*)
    };

    (@object $object:ident () () ()) => {};

    (@object $object:ident [$($key:tt)+] ($value:expr) , $($rest:tt)*) => {
        $crate::__private::insert_object(
            &mut $object,
            $crate::scon!(@key $($key)+),
            $value,
        );
        $crate::scon!(@object $object () ($($rest)*) ($($rest)*));
    };

    (@object $object:ident [$($key:tt)+] ($value:expr) $unexpected:tt $($rest:tt)*) => {
        compile_error!(concat!("unexpected token in scon! object: ", stringify!($unexpected)));
    };

    (@object $object:ident [$($key:tt)+] ($value:expr)) => {
        $crate::__private::insert_object(
            &mut $object,
            $crate::scon!(@key $($key)+),
            $value,
        );
    };

    (@object $object:ident ($($key:tt)+) (: null $($rest:tt)*) $copy:tt) => {
        $crate::scon!(@object $object [$($key)+] ($crate::scon!(@value null)) $($rest)*);
    };

    (@object $object:ident ($($key:tt)+) (: true $($rest:tt)*) $copy:tt) => {
        $crate::scon!(@object $object [$($key)+] ($crate::scon!(@value true)) $($rest)*);
    };

    (@object $object:ident ($($key:tt)+) (: false $($rest:tt)*) $copy:tt) => {
        $crate::scon!(@object $object [$($key)+] ($crate::scon!(@value false)) $($rest)*);
    };

    (@object $object:ident ($($key:tt)+) (: [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
        $crate::scon!(@object $object [$($key)+] ($crate::scon!(@value [$($array)*])) $($rest)*);
    };

    (@object $object:ident ($($key:tt)+) (: {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
        $crate::scon!(@object $object [$($key)+] ($crate::scon!(@value {$($map)*})) $($rest)*);
    };

    (@object $object:ident ($($key:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => {
        $crate::scon!(@object $object [$($key)+] ($crate::scon!(@value $value)) , $($rest)*);
    };

    (@object $object:ident ($($key:tt)+) (: $value:expr) $copy:tt) => {
        $crate::scon!(@object $object [$($key)+] ($crate::scon!(@value $value)));
    };

    (@object $object:ident ($($key:tt)+) (:) $copy:tt) => {
        compile_error!("missing value in scon! object");
    };

    (@object $object:ident ($($key:tt)+) () $copy:tt) => {
        compile_error!("missing colon and value in scon! object");
    };

    (@object $object:ident () (: $($rest:tt)*) ($colon:tt $($copy:tt)*)) => {
        compile_error!("unexpected colon in scon! object");
    };

    (@object $object:ident ($($key:tt)*) (, $($rest:tt)*) ($comma:tt $($copy:tt)*)) => {
        compile_error!("unexpected comma in scon! object key");
    };

    (@object $object:ident () (($key:expr) : $($rest:tt)*) $copy:tt) => {
        $crate::scon!(@object $object (($key)) (: $($rest)*) (: $($rest)*));
    };

    (@object $object:ident ($($key:tt)*) (: $($unexpected:tt)+) $copy:tt) => {
        compile_error!("invalid expression in scon! object value");
    };

    (@object $object:ident ($($key:tt)*) ($tt:tt $($rest:tt)*) $copy:tt) => {
        $crate::scon!(@object $object ($($key)* $tt) ($($rest)*) ($($rest)*));
    };

    (@key $key:ident) => {
        stringify!($key).to_string()
    };

    (@key ($key:expr)) => {
        $crate::__private::key_to_string(&$key)
    };

    (@key $key:literal) => {
        $crate::__private::key_to_string(&$key)
    };

    (@value null) => {
        $crate::Value::Null
    };

    (@value true) => {
        $crate::Value::Bool(true)
    };

    (@value false) => {
        $crate::Value::Bool(false)
    };

    (@value []) => {
        $crate::Value::Array(vec![])
    };

    (@value [ $($tt:tt)+ ]) => {
        $crate::Value::Array($crate::scon!(@array [] $($tt)+))
    };

    (@value {}) => {
        $crate::Value::Object($crate::__private::new_object())
    };

    (@value { $($tt:tt)+ }) => {
        $crate::Value::Object({
            let mut object = $crate::__private::new_object();
            $crate::scon!(@object object () ($($tt)+) ($($tt)+));
            object
        })
    };

    (@value $other:expr) => {
        $crate::__private::to_value(&$other)
    };

    ($($scon:tt)+) => {
        $crate::scon!(@value $($scon)+)
    };
}

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
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};

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
        b.insert("b".to_string(), Value::Number(Number::from_u64(1)));
        let mut a = IndexMap::new();
        a.insert("a".to_string(), Value::Object(b));
        assert_eq!(value, Value::Object(a));
    }

    #[test]
    fn parses_numbers_into_explicit_number_variants() {
        let value = parse_str(
            r#"
            zero = 0
            max = 18446744073709551615
            negative = -9223372036854775808
            decimal = 1.25
            exponent = 1e2
            "#,
        )
        .unwrap();
        let Value::Object(root) = value else {
            panic!("expected object root");
        };

        assert_eq!(root["zero"], Value::Number(Number::from_u64(0)));
        assert_eq!(root["max"], Value::Number(Number::from_u64(u64::MAX)));
        assert_eq!(root["negative"], Value::Number(Number::from_i64(i64::MIN)));
        assert_eq!(root["decimal"], Value::Number(Number::F64(1.25)));
        assert_eq!(root["exponent"], Value::Number(Number::F64(100.0)));
    }

    #[test]
    fn rejects_numbers_outside_the_supported_model() {
        let err = parse_str("too_large = 18446744073709551616").unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidNumber);

        let err = parse_str("too_small = -9223372036854775809").unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidNumber);

        let err = parse_str("too_large_float = 1e9999").unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidNumber);
    }

    #[test]
    fn deserializes_numbers_with_checked_integer_conversions() {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Ports {
            small: u8,
            signed: i16,
            ratio: f32,
        }

        let ports: Ports = from_str(
            r#"
            small = 255
            signed = -128
            ratio = 1.5
            "#,
        )
        .unwrap();
        assert_eq!(
            ports,
            Ports {
                small: 255,
                signed: -128,
                ratio: 1.5,
            }
        );

        assert_eq!(
            from_str::<Ports>("small = 256\nsigned = 0\nratio = 1.0")
                .unwrap_err()
                .code,
            ErrorCode::Serde
        );
        assert_eq!(
            from_str::<Ports>("small = -1\nsigned = 0\nratio = 1.0")
                .unwrap_err()
                .code,
            ErrorCode::Serde
        );
    }

    #[test]
    fn serializes_numbers_into_explicit_number_variants() {
        assert_eq!(
            to_value(&-1i64).unwrap(),
            Value::Number(Number::from_i64(-1))
        );
        assert_eq!(to_value(&1u64).unwrap(), Value::Number(Number::from_u64(1)));
        assert_eq!(
            to_value(&1.25f64).unwrap(),
            Value::Number(Number::F64(1.25))
        );
        assert_eq!(
            to_string_fragment(&Value::Number(Number::F64(1.0))),
            "1.0\n"
        );
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
        assert_eq!(root["a"], Value::Number(Number::from_u64(1)));
        assert_eq!(root["b"], Value::Number(Number::from_u64(2)));

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
    fn format_source_uses_ast_trivia_golden_output() {
        let formatted = format_source(
            r#"
# leading
include "./base.scon"
defaults { // inline
host="127.0.0.1"
}
server {
...${defaults}
paths=[...${base_paths},"/opt/bin"]
url="http://${defaults.host}" // url
}
"#,
            FormatOptions::default(),
        )
        .unwrap();

        assert_eq!(
            formatted,
            r#"# leading
include "./base.scon"
defaults { // inline
  host = "127.0.0.1"
}
server {
  ...${defaults}
  paths = [
    ...${base_paths},
    "/opt/bin"
  ]
  url = "http://${defaults.host}" // url
}
"#
        );
    }

    #[test]
    fn format_source_round_trips_resolved_value() {
        let source = r#"
defaults {
host = "127.0.0.1"
}
base_paths = ["/bin", "/usr/bin"]
server {
...${defaults}
paths = [...${base_paths}, "/opt/bin"]
url = "http://${defaults.host}"
}
"#;

        let formatted = format_source(source, FormatOptions::default()).unwrap();
        parse_source(&formatted, ParseOptions::default()).unwrap();
        assert_eq!(parse_str(source).unwrap(), parse_str(&formatted).unwrap());
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
    fn analyze_source_exposes_definitions_references_and_targets() {
        let analysis = analyze_source(
            r#"
defaults {
  host = "127.0.0.1"
}
base_paths = ["/bin"]
server {
  ...${defaults}
  paths = [...${base_paths}]
  host_copy = ${defaults.host}
  url = "http://${defaults.host}"
}
"#,
            ParseOptions::default(),
        );

        assert!(analysis.diagnostics.is_empty());
        assert!(analysis.value.is_some());
        assert!(
            analysis
                .definitions
                .iter()
                .any(|definition| definition.path == vec!["defaults", "host"])
        );
        assert!(analysis.references.iter().any(|reference| {
            reference.kind == ReferenceKind::ObjectSpread
                && reference.path == vec!["defaults"]
                && reference.target.is_some()
        }));
        assert!(analysis.references.iter().any(|reference| {
            reference.kind == ReferenceKind::ArraySpread
                && reference.path == vec!["base_paths"]
                && reference.target.is_some()
        }));
        assert!(analysis.references.iter().any(|reference| {
            reference.kind == ReferenceKind::Interpolation
                && reference.path == vec!["defaults", "host"]
                && reference.target.is_some()
        }));
    }

    #[test]
    fn analyze_file_can_use_source_store_for_unsaved_includes() {
        struct MemoryStore {
            sources: HashMap<PathBuf, String>,
        }

        impl SourceStore for MemoryStore {
            fn read_source(&self, path: &Path) -> std::io::Result<Option<String>> {
                Ok(self.sources.get(path).cloned())
            }
        }

        let root = tempfile::tempdir().unwrap();
        let root_path = root.path().canonicalize().unwrap();
        let app = root_path.join("app.scon");
        let base = root_path.join("base.scon");
        let mut sources = HashMap::new();
        sources.insert(
            app.clone(),
            r#"
defaults {
  host = "0.0.0.0"
}
include "./base.scon"
"#
            .to_string(),
        );
        sources.insert(
            base.clone(),
            r#"
server {
  host = ${defaults.host}
}
"#
            .to_string(),
        );

        let store = MemoryStore { sources };
        let analysis = analyze_file_with_store(&app, LoadOptions::default(), &store);

        assert!(analysis.diagnostics.is_empty());
        assert!(analysis.value.is_some());
        assert_eq!(analysis.includes.len(), 1);
        assert_eq!(analysis.includes[0].resolved_path, Some(base));
    }

    #[test]
    fn analyze_source_reports_reference_diagnostic_at_reference_span() {
        let analysis = analyze_source("a = ${missing}\n", ParseOptions::default());
        let diagnostic = analysis.diagnostics.first().unwrap();
        assert_eq!(diagnostic.code, ErrorCode::MissingReference);
        let range = diagnostic.range.as_ref().unwrap();
        assert_eq!(range.start.character, 6);
        assert_eq!(range.end.character, 13);
    }

    #[test]
    fn to_string_requires_object_root() {
        let err = format::to_string(&Value::Array(vec![])).unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidRootType);
    }

    #[test]
    fn parse_file_evaluates_includes_in_source_order() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("base.scon"),
            r#"
            server {
              host = ${defaults.host}
              port = 8080
            }
            "#,
        )
        .unwrap();
        fs::write(
            dir.path().join("app.scon"),
            r#"
            defaults {
              host = "0.0.0.0"
            }
            include "./base.scon"
            "#,
        )
        .unwrap();

        let value = parse_file(dir.path().join("app.scon")).unwrap();
        let Value::Object(root) = value else {
            panic!("expected object root");
        };
        let Value::Object(server) = &root["server"] else {
            panic!("expected server object");
        };
        assert_eq!(server["host"], Value::String("0.0.0.0".to_string()));
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

    #[test]
    fn scon_macro_builds_nested_values() {
        let port = 8080u64;
        let dynamic_key = String::from("read.only");

        let value = scon!({
            name: "demo",
            "enabled": true,
            (dynamic_key): false,
            port: port,
            paths: ["/bin", "/usr/bin",],
            server: {
                host: "127.0.0.1",
                tls: null,
            },
        });

        let Value::Object(root) = value else {
            panic!("expected object root");
        };
        assert_eq!(root["name"], Value::String("demo".to_string()));
        assert_eq!(root["enabled"], Value::Bool(true));
        assert_eq!(root["read.only"], Value::Bool(false));
        assert_eq!(root["port"], Value::Number(Number::from_u64(8080)));
        assert_eq!(
            root["paths"],
            Value::Array(vec![
                Value::String("/bin".to_string()),
                Value::String("/usr/bin".to_string()),
            ])
        );

        let Value::Object(server) = &root["server"] else {
            panic!("expected server object");
        };
        assert_eq!(server["host"], Value::String("127.0.0.1".to_string()));
        assert_eq!(server["tls"], Value::Null);
    }

    #[test]
    fn scon_macro_builds_empty_arrays_and_objects() {
        assert_eq!(scon!([]), Value::Array(vec![]));
        assert_eq!(scon!({}), Value::Object(IndexMap::new()));
    }

    #[test]
    fn scon_macro_formats_fragment() {
        let value = scon!({
            list: [1u64, 2u64],
        });

        assert_eq!(
            to_string_fragment(&value),
            "{\n  list = [\n    1,\n    2,\n  ]\n}\n"
        );
    }

    #[test]
    fn scon_macro_uses_rust_numeric_semantics() {
        assert_eq!(scon!(1), Value::Number(Number::from_i64(1)));
        assert_eq!(scon!(1u64), Value::Number(Number::from_u64(1)));
        assert_eq!(scon!(-1i64), Value::Number(Number::from_i64(-1)));
        assert_eq!(
            scon!(1.25f64),
            Value::Number(Number::from_f64(1.25).unwrap())
        );
    }

    #[test]
    #[should_panic(expected = "scon! object keys must serialize as strings")]
    fn scon_macro_rejects_non_string_dynamic_keys() {
        let _ = scon!({
            (1u64): "bad",
        });
    }

    #[test]
    #[should_panic(expected = "scon! failed to serialize interpolated value")]
    fn scon_macro_rejects_non_finite_float_values() {
        let _ = scon!(f64::NAN);
    }
}
