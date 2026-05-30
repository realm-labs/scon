#![allow(clippy::result_large_err)]

mod ast;
mod de;
mod error;
mod eval;
mod format;
mod lexer;
mod limits;
mod loader;
mod parser;
mod ser;
mod value;

use std::path::Path;

pub use error::{Error, ErrorCode, Result};
pub use limits::{Limits, LoadOptions};
pub use value::Value;

pub fn from_str<T>(source: &str) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    from_value(parse_str(source)?)
}

pub fn from_file<T>(path: impl AsRef<Path>) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    from_value(parse_file(path)?)
}

pub fn to_string<T>(value: &T) -> Result<String>
where
    T: serde::Serialize,
{
    let value = to_value(value)?;
    format::to_string(&value)
}

pub fn parse_str(source: &str) -> Result<Value> {
    let doc = parser::parse_str(source, None)?;
    eval::eval_document(&doc, &mut loader::NoopLoader)
}

pub fn parse_file(path: impl AsRef<Path>) -> Result<Value> {
    parse_file_with_options(path, LoadOptions::default())
}

pub fn parse_file_with_options(path: impl AsRef<Path>, options: LoadOptions) -> Result<Value> {
    let mut loader = loader::FileLoader::new(path.as_ref(), options)?;
    let doc = loader.load_entry()?;
    eval::eval_document(&doc, &mut loader)
}

pub fn from_file_with_options<T>(path: impl AsRef<Path>, options: LoadOptions) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    from_value(parse_file_with_options(path, options)?)
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
