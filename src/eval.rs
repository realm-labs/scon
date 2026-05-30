use indexmap::IndexMap;

use crate::ast::*;
use crate::error::{Error, ErrorCode, Result};
use crate::loader::IncludeLoader;
use crate::value::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Layer {
    Base,
    Local,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    StructuralObject,
    OrdinaryValue,
}

#[derive(Clone, Debug)]
enum EvalValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Value>),
    Object(IndexMap<String, Entry>),
}

#[derive(Clone, Debug)]
struct Entry {
    value: EvalValue,
    layer: Layer,
    kind: Kind,
}

pub(crate) fn eval_document(doc: &Document, loader: &mut dyn IncludeLoader) -> Result<Value> {
    let mut evaluator = Evaluator {
        root: IndexMap::new(),
        in_progress: vec![Vec::new()],
    };
    evaluator.eval_contents(&doc.body, &[], doc.file.as_deref(), loader)?;
    evaluator.in_progress.pop();
    Ok(Value::Object(to_public_object(&evaluator.root)))
}

struct Evaluator {
    root: IndexMap<String, Entry>,
    in_progress: Vec<Vec<String>>,
}

impl Evaluator {
    fn eval_contents(
        &mut self,
        body: &ObjectBody,
        path: &[String],
        file: Option<&std::path::Path>,
        loader: &mut dyn IncludeLoader,
    ) -> Result<()> {
        for spread in &body.spreads {
            let base = self.lookup_completed(&spread.path, &spread.loc)?;
            let Value::Object(object) = base else {
                return Err(Error::new(
                    ErrorCode::TypeMismatch,
                    "object spread target is not an object",
                )
                .at(spread.loc.clone())
                .with_path(&spread.path));
            };
            let base_entries = from_public_object(object, Layer::Base, Kind::OrdinaryValue);
            let target = object_mut_at(&mut self.root, path).ok_or_else(|| {
                Error::new(ErrorCode::PathConflict, "target object does not exist")
            })?;
            overlay_base(target, base_entries);
        }

        for member in &body.members {
            match member {
                LocalMember::Include {
                    path: include_path,
                    loc,
                } => {
                    let included = loader.load_include(file, include_path, loc.clone())?;
                    self.eval_contents(&included.body, path, included.file.as_deref(), loader)?;
                }
                LocalMember::Field(field) => self.eval_field(field, path, file, loader)?,
            }
        }
        Ok(())
    }

    fn eval_field(
        &mut self,
        field: &Field,
        current_path: &[String],
        file: Option<&std::path::Path>,
        loader: &mut dyn IncludeLoader,
    ) -> Result<()> {
        let mut target_path = current_path.to_vec();
        target_path.extend(field.path.clone());
        match &field.value {
            AstValue::Object(body) => {
                ensure_local_object(&mut self.root, &target_path, &field.loc)?;
                self.in_progress.push(target_path.clone());
                self.eval_contents(body, &target_path, file, loader)?;
                self.in_progress.pop();
                Ok(())
            }
            value => {
                let evaluated = self.eval_value(value)?;
                insert_local_value(
                    &mut self.root,
                    &target_path,
                    evaluated,
                    Kind::OrdinaryValue,
                    &field.loc,
                )
            }
        }
    }

    fn eval_value(&mut self, value: &AstValue) -> Result<Value> {
        match value {
            AstValue::Object(body) => {
                let mut nested = Evaluator {
                    root: IndexMap::new(),
                    in_progress: vec![Vec::new()],
                };
                // Anonymous object values cannot contain includes because there is no file context here.
                nested.eval_contents(body, &[], None, &mut crate::loader::NoopLoader)?;
                Ok(Value::Object(to_public_object(&nested.root)))
            }
            AstValue::Array(items) => {
                let mut out = Vec::new();
                for item in items {
                    match item {
                        ArrayItem::Value(value) => out.push(self.eval_value(value)?),
                        ArrayItem::Spread { path, loc } => {
                            let target = self.lookup_completed(path, loc)?;
                            let Value::Array(values) = target else {
                                return Err(Error::new(
                                    ErrorCode::TypeMismatch,
                                    "array spread target is not an array",
                                )
                                .at(loc.clone())
                                .with_path(path));
                            };
                            out.extend(values);
                        }
                    }
                }
                Ok(Value::Array(out))
            }
            AstValue::String(parts) => {
                let mut out = String::new();
                for part in parts {
                    match part {
                        StringPart::Literal(text) => out.push_str(text),
                        StringPart::Interpolation(path) => {
                            let value = self.lookup_completed(
                                path,
                                &Location {
                                    file: None,
                                    line: 1,
                                    column: 1,
                                },
                            )?;
                            match value {
                                Value::String(text) => out.push_str(&text),
                                Value::Number(text) => out.push_str(&text),
                                Value::Bool(value) => {
                                    out.push_str(if value { "true" } else { "false" })
                                }
                                _ => {
                                    return Err(Error::new(
                                        ErrorCode::TypeMismatch,
                                        "interpolation requires string, number, or boolean",
                                    )
                                    .with_path(path));
                                }
                            }
                        }
                    }
                }
                Ok(Value::String(out))
            }
            AstValue::Number(text) => Ok(Value::Number(text.clone())),
            AstValue::Bool(value) => Ok(Value::Bool(*value)),
            AstValue::Null => Ok(Value::Null),
            AstValue::Substitution(path) => self.lookup_completed(
                path,
                &Location {
                    file: None,
                    line: 1,
                    column: 1,
                },
            ),
        }
    }

    fn lookup_completed(&self, path: &[String], loc: &Location) -> Result<Value> {
        if self.in_progress.iter().any(|p| p == path) {
            return Err(Error::new(
                ErrorCode::MissingReference,
                format!("path {:?} is not completed before use", path),
            )
            .at(loc.clone())
            .with_path(path));
        }
        lookup_entry(&self.root, path)
            .map(entry_to_public)
            .ok_or_else(|| {
                Error::new(
                    ErrorCode::MissingReference,
                    format!("path {:?} is not defined before use", path),
                )
                .at(loc.clone())
                .with_path(path)
            })
    }
}

fn lookup_entry<'a>(object: &'a IndexMap<String, Entry>, path: &[String]) -> Option<&'a Entry> {
    let (first, rest) = path.split_first()?;
    let entry = object.get(first)?;
    if rest.is_empty() {
        return Some(entry);
    }
    match &entry.value {
        EvalValue::Object(child) => lookup_entry(child, rest),
        _ => None,
    }
}

fn object_mut_at<'a>(
    object: &'a mut IndexMap<String, Entry>,
    path: &[String],
) -> Option<&'a mut IndexMap<String, Entry>> {
    if path.is_empty() {
        return Some(object);
    }
    let (first, rest) = path.split_first()?;
    let entry = object.get_mut(first)?;
    match &mut entry.value {
        EvalValue::Object(child) => object_mut_at(child, rest),
        _ => None,
    }
}

fn ensure_local_object(
    object: &mut IndexMap<String, Entry>,
    path: &[String],
    loc: &Location,
) -> Result<()> {
    let Some((first, rest)) = path.split_first() else {
        return Ok(());
    };
    if rest.is_empty() {
        match object.get_mut(first) {
            None => {
                object.insert(
                    first.clone(),
                    Entry {
                        value: EvalValue::Object(IndexMap::new()),
                        layer: Layer::Local,
                        kind: Kind::StructuralObject,
                    },
                );
            }
            Some(entry) if entry.layer == Layer::Base => {
                if !matches!(entry.value, EvalValue::Object(_)) {
                    entry.value = EvalValue::Object(IndexMap::new());
                }
                entry.layer = Layer::Local;
                entry.kind = Kind::StructuralObject;
            }
            Some(entry)
                if entry.layer == Layer::Local
                    && entry.kind == Kind::StructuralObject
                    && matches!(entry.value, EvalValue::Object(_)) => {}
            Some(_) => {
                return Err(Error::new(
                    ErrorCode::PathConflict,
                    "path conflicts with an existing local value",
                )
                .at(loc.clone())
                .with_path(path));
            }
        }
        return Ok(());
    }

    let child = object.entry(first.clone()).or_insert_with(|| Entry {
        value: EvalValue::Object(IndexMap::new()),
        layer: Layer::Local,
        kind: Kind::StructuralObject,
    });
    if !matches!(child.value, EvalValue::Object(_)) {
        if child.layer == Layer::Base {
            child.value = EvalValue::Object(IndexMap::new());
            child.layer = Layer::Local;
            child.kind = Kind::StructuralObject;
        } else {
            return Err(
                Error::new(ErrorCode::PathConflict, "path conflicts with scalar value")
                    .at(loc.clone())
                    .with_path(path),
            );
        }
    }
    let EvalValue::Object(child_object) = &mut child.value else {
        unreachable!();
    };
    ensure_local_object(child_object, rest, loc)
}

fn insert_local_value(
    object: &mut IndexMap<String, Entry>,
    path: &[String],
    value: Value,
    kind: Kind,
    loc: &Location,
) -> Result<()> {
    let Some((first, rest)) = path.split_first() else {
        return Err(
            Error::new(ErrorCode::InvalidRootType, "cannot replace root object").at(loc.clone()),
        );
    };
    if rest.is_empty() {
        let entry = Entry {
            value: from_public_value(value),
            layer: Layer::Local,
            kind,
        };
        match object.get_mut(first) {
            None => {
                object.insert(first.clone(), entry);
                Ok(())
            }
            Some(existing) if existing.layer == Layer::Base => {
                overlay_replace(existing, entry);
                Ok(())
            }
            Some(_) => Err(Error::new(ErrorCode::DuplicateKey, "duplicate local key")
                .at(loc.clone())
                .with_path(path)),
        }
    } else {
        ensure_local_object(object, &path[..path.len() - rest.len()], loc)?;
        let child = object.get_mut(first).unwrap();
        let EvalValue::Object(child_object) = &mut child.value else {
            return Err(Error::new(ErrorCode::PathConflict, "path conflict")
                .at(loc.clone())
                .with_path(path));
        };
        insert_local_value(child_object, rest, value, kind, loc)
    }
}

fn overlay_replace(left: &mut Entry, right: Entry) {
    match (&mut left.value, right.value) {
        (EvalValue::Object(left_object), EvalValue::Object(right_object)) => {
            overlay_base(left_object, right_object);
            left.layer = Layer::Local;
            left.kind = right.kind;
        }
        (_, value) => {
            left.value = value;
            left.layer = Layer::Local;
            left.kind = right.kind;
        }
    }
}

fn overlay_base(left: &mut IndexMap<String, Entry>, right: IndexMap<String, Entry>) {
    for (key, incoming) in right {
        match left.get_mut(&key) {
            None => {
                left.insert(key, incoming);
            }
            Some(existing) if existing.layer == Layer::Base => {
                match (&mut existing.value, incoming.value) {
                    (EvalValue::Object(left_object), EvalValue::Object(right_object)) => {
                        overlay_base(left_object, right_object);
                    }
                    (_, value) => {
                        existing.value = value;
                    }
                }
            }
            Some(existing) => {
                if let (EvalValue::Object(left_object), EvalValue::Object(right_object)) =
                    (&mut existing.value, incoming.value)
                {
                    overlay_base(left_object, right_object);
                }
            }
        }
    }
}

fn from_public_object(
    object: IndexMap<String, Value>,
    layer: Layer,
    kind: Kind,
) -> IndexMap<String, Entry> {
    object
        .into_iter()
        .map(|(key, value)| {
            (
                key,
                Entry {
                    value: from_public_value(value),
                    layer,
                    kind,
                },
            )
        })
        .collect()
}

fn from_public_value(value: Value) -> EvalValue {
    match value {
        Value::Null => EvalValue::Null,
        Value::Bool(value) => EvalValue::Bool(value),
        Value::Number(value) => EvalValue::Number(value),
        Value::String(value) => EvalValue::String(value),
        Value::Array(value) => EvalValue::Array(value),
        Value::Object(value) => {
            EvalValue::Object(from_public_object(value, Layer::Local, Kind::OrdinaryValue))
        }
    }
}

fn entry_to_public(entry: &Entry) -> Value {
    match &entry.value {
        EvalValue::Null => Value::Null,
        EvalValue::Bool(value) => Value::Bool(*value),
        EvalValue::Number(value) => Value::Number(value.clone()),
        EvalValue::String(value) => Value::String(value.clone()),
        EvalValue::Array(value) => Value::Array(value.clone()),
        EvalValue::Object(value) => Value::Object(to_public_object(value)),
    }
}

fn to_public_object(object: &IndexMap<String, Entry>) -> IndexMap<String, Value> {
    object
        .iter()
        .map(|(key, value)| (key.clone(), entry_to_public(value)))
        .collect()
}
