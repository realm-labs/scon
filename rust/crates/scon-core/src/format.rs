use crate::error::{Error, ErrorCode, Result};
use crate::value::Value;

pub(crate) fn to_string(value: &Value) -> Result<String> {
    let Value::Object(object) = value else {
        return Err(Error::new(
            ErrorCode::InvalidRootType,
            "SCON document root must be an object",
        ));
    };
    let mut out = String::new();
    write_object_body(&mut out, object, 0);
    out.push('\n');
    Ok(out)
}

pub(crate) fn to_fragment_string(value: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, value, 0);
    out.push('\n');
    out
}

fn write_object_body(out: &mut String, object: &indexmap::IndexMap<String, Value>, indent: usize) {
    for (key, value) in object {
        write_indent(out, indent);
        out.push_str(&format_key(key));
        out.push_str(" = ");
        write_value(out, value, indent);
        out.push('\n');
    }
}

fn write_value(out: &mut String, value: &Value, indent: usize) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(value) => out.push_str(&value.to_string()),
        Value::String(value) => write_string(out, value),
        Value::Array(values) => {
            if values.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push_str("[\n");
            for value in values {
                write_indent(out, indent + 2);
                write_value(out, value, indent + 2);
                out.push_str(",\n");
            }
            write_indent(out, indent);
            out.push(']');
        }
        Value::Object(object) => {
            if object.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{\n");
            write_object_body(out, object, indent + 2);
            write_indent(out, indent);
            out.push('}');
        }
    }
}

fn write_indent(out: &mut String, indent: usize) {
    for _ in 0..indent {
        out.push(' ');
    }
}

fn write_string(out: &mut String, value: &str) {
    out.push('"');
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000c}' => out.push_str("\\f"),
            '$' if chars.peek() == Some(&'{') => out.push_str("\\$"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04X}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
}

fn format_key(key: &str) -> String {
    if is_identifier(key) {
        key.to_string()
    } else {
        let mut out = String::new();
        write_string(&mut out, key);
        out
    }
}

fn is_identifier(key: &str) -> bool {
    if matches!(key, "include" | "true" | "false" | "null") {
        return false;
    }
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}
