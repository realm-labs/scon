use std::fs;
use std::path::{Path, PathBuf};

use scon::{Number, Value};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Manifest {
    cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
struct Case {
    id: String,
    description: String,
    entry: PathBuf,
    kind: CaseKind,
    expected: PathBuf,
    tags: Vec<String>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
enum CaseKind {
    Valid,
    Invalid,
}

#[derive(Debug, Deserialize)]
struct ExpectedError {
    code: String,
}

#[test]
fn conformance_fixtures_match_rust_implementation() {
    let root = conformance_root();
    let manifest: Manifest = read_json(&root.join("manifest.json"));

    for case in manifest.cases {
        let entry = root.join(&case.entry);
        let expected = root.join(&case.expected);
        match case.kind {
            CaseKind::Valid => {
                let value = scon::parse_file(&entry).unwrap_or_else(|err| {
                    panic!(
                        "valid conformance case `{}` failed: {err:?}\n{}",
                        case.id, case.description
                    )
                });
                let actual = value_to_json(&value);
                let expected_json: serde_json::Value = read_json(&expected);
                assert_eq!(
                    actual, expected_json,
                    "valid conformance case `{}` resolved differently\n{}\ntags: {:?}",
                    case.id, case.description, case.tags
                );
            }
            CaseKind::Invalid => {
                let err = scon::parse_file(&entry).unwrap_err_or_else(|value| {
                    panic!(
                        "invalid conformance case `{}` unexpectedly resolved to {value:?}\n{}",
                        case.id, case.description
                    )
                });
                let expected_error: ExpectedError = read_json(&expected);
                assert_eq!(
                    format!("{:?}", err.code),
                    expected_error.code,
                    "invalid conformance case `{}` produced wrong error\n{}\ntags: {:?}\nmessage: {}",
                    case.id,
                    case.description,
                    case.tags,
                    err.message
                );
            }
        }
    }
}

trait UnwrapErrOrElse<T, E> {
    fn unwrap_err_or_else<F>(self, f: F) -> E
    where
        F: FnOnce(T) -> E;
}

impl<T, E> UnwrapErrOrElse<T, E> for Result<T, E> {
    fn unwrap_err_or_else<F>(self, f: F) -> E
    where
        F: FnOnce(T) -> E,
    {
        match self {
            Ok(value) => f(value),
            Err(err) => err,
        }
    }
}

fn conformance_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/conformance")
        .canonicalize()
        .expect("conformance fixture directory should exist")
}

fn read_json<T>(path: &Path) -> T
where
    T: serde::de::DeserializeOwned,
{
    let text = fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", path.display()))
}

fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(value) => serde_json::Value::Bool(*value),
        Value::Number(value) => match value {
            Number::I64(value) => serde_json::Value::Number((*value).into()),
            Number::U64(value) => serde_json::Value::Number((*value).into()),
            Number::F64(value) => serde_json::Number::from_f64(*value)
                .map(serde_json::Value::Number)
                .expect("SCON f64 numbers are finite"),
        },
        Value::String(value) => serde_json::Value::String(value.clone()),
        Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(value_to_json).collect())
        }
        Value::Object(object) => serde_json::Value::Object(
            object
                .iter()
                .map(|(key, value)| (key.clone(), value_to_json(value)))
                .collect(),
        ),
    }
}
