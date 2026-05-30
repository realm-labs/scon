use std::borrow::Cow;

use indexmap::{IndexMap, map::Entry as IndexEntry};
use rustc_hash::FxBuildHasher;

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
pub(crate) enum EvalValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<EvalValue>),
    Object(EvalObject),
}

#[derive(Clone, Debug)]
pub(crate) struct Entry {
    pub(crate) value: EvalValue,
    layer: Layer,
    kind: Kind,
}

pub(crate) type EvalObject = IndexMap<String, Entry, FxBuildHasher>;

pub(crate) struct ResolvedDocument {
    pub(crate) root: EvalObject,
}

pub(crate) fn eval_document(doc: Document, loader: &mut dyn IncludeLoader) -> Result<Value> {
    let resolved = eval_resolved_document(doc, loader)?;
    Ok(Value::Object(into_public_object(resolved.root)))
}

pub(crate) fn eval_resolved_document(
    doc: Document,
    loader: &mut dyn IncludeLoader,
) -> Result<ResolvedDocument> {
    let mut evaluator = Evaluator {
        root: EvalObject::default(),
        in_progress: vec![Vec::new()],
    };
    let Document { body, file, .. } = doc;
    evaluator.eval_contents(body, &[], file.as_deref(), loader)?;
    evaluator.in_progress.pop();
    Ok(ResolvedDocument {
        root: evaluator.root,
    })
}

struct Evaluator {
    root: EvalObject,
    in_progress: Vec<Vec<String>>,
}

impl Evaluator {
    fn eval_contents(
        &mut self,
        body: ObjectBody,
        path: &[String],
        file: Option<&std::path::Path>,
        loader: &mut dyn IncludeLoader,
    ) -> Result<()> {
        if let Some(target) = object_mut_at(&mut self.root, path) {
            target.reserve(body.spreads.len() + body.members.len());
        }

        for spread in body.spreads {
            let base_entries = {
                let base = self.lookup_completed_entry(spread.path.as_slice(), &spread.loc)?;
                let EvalValue::Object(object) = &base.value else {
                    return Err(Error::new(
                        ErrorCode::TypeMismatch,
                        "object spread target is not an object",
                    )
                    .at(spread.loc.clone())
                    .with_path(spread.path.as_slice()));
                };
                clone_object_with_layer(object, Layer::Base, Kind::OrdinaryValue)
            };
            let target = object_mut_at(&mut self.root, path).ok_or_else(|| {
                Error::new(ErrorCode::PathConflict, "target object does not exist")
            })?;
            overlay_base(target, base_entries);
        }

        for member in body.members {
            match member {
                LocalMember::Include {
                    path: include_path,
                    loc,
                    ..
                } => {
                    let included = loader.load_include(file, &include_path, loc.clone())?;
                    let Document {
                        body: included_body,
                        file: included_file,
                        ..
                    } = included;
                    self.eval_contents(included_body, path, included_file.as_deref(), loader)?;
                }
                LocalMember::Field(field) => self.eval_field(field, path, file, loader)?,
            }
        }
        Ok(())
    }

    fn eval_field(
        &mut self,
        field: Field,
        current_path: &[String],
        file: Option<&std::path::Path>,
        loader: &mut dyn IncludeLoader,
    ) -> Result<()> {
        let Field {
            path: field_path,
            value,
            loc,
            ..
        } = field;
        let target_path: Cow<'_, [String]> = if current_path.is_empty() {
            Cow::Borrowed(field_path.as_slice())
        } else {
            let mut target_path = Vec::with_capacity(current_path.len() + field_path.len());
            target_path.extend_from_slice(current_path);
            target_path.extend_from_slice(&field_path);
            Cow::Owned(target_path)
        };
        match value {
            AstValue::Object { body, .. } => {
                ensure_local_object(&mut self.root, target_path.as_ref(), &loc)?;
                self.in_progress.push(target_path.as_ref().to_vec());
                self.eval_contents(body, target_path.as_ref(), file, loader)?;
                self.in_progress.pop();
                Ok(())
            }
            value => {
                let evaluated = self.eval_value(value)?;
                if current_path.is_empty() {
                    insert_local_value_owned(
                        &mut self.root,
                        field_path,
                        evaluated,
                        Kind::OrdinaryValue,
                        &loc,
                    )
                } else {
                    insert_local_value(
                        &mut self.root,
                        target_path.as_ref(),
                        evaluated,
                        Kind::OrdinaryValue,
                        &loc,
                    )
                }
            }
        }
    }

    fn eval_value(&mut self, value: AstValue) -> Result<EvalValue> {
        match value {
            AstValue::Object { body, .. } => {
                let mut nested = Evaluator {
                    root: EvalObject::default(),
                    in_progress: vec![Vec::new()],
                };
                // Anonymous object values cannot contain includes because there is no file context here.
                nested.eval_contents(body, &[], None, &mut crate::loader::NoopLoader)?;
                Ok(EvalValue::Object(nested.root))
            }
            AstValue::Array { items, .. } => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    match item {
                        ArrayItem::Value(value) => out.push(self.eval_value(value)?),
                        ArrayItem::Spread { path, loc, .. } => {
                            let target = self.lookup_completed_entry(path.as_slice(), &loc)?;
                            let EvalValue::Array(values) = &target.value else {
                                return Err(Error::new(
                                    ErrorCode::TypeMismatch,
                                    "array spread target is not an array",
                                )
                                .at(loc.clone())
                                .with_path(path.as_slice()));
                            };
                            out.reserve(values.len());
                            out.extend(values.iter().cloned());
                        }
                    }
                }
                Ok(EvalValue::Array(out))
            }
            AstValue::String { value: string, .. } => match string {
                StringValue::Literal(text) => Ok(EvalValue::String(text)),
                StringValue::Parts(parts) => {
                    let mut out = String::new();
                    for part in parts {
                        match part {
                            StringPart::Literal(text) => out.push_str(&text),
                            StringPart::Interpolation { path, .. } => {
                                let value = self.lookup_completed_entry(
                                    path.as_slice(),
                                    &Location {
                                        file: None,
                                        line: 1,
                                        column: 1,
                                        span: Span::default(),
                                    },
                                )?;
                                match &value.value {
                                    EvalValue::String(text) => out.push_str(text),
                                    EvalValue::Number(text) => out.push_str(text),
                                    EvalValue::Bool(value) => {
                                        out.push_str(if *value { "true" } else { "false" })
                                    }
                                    _ => {
                                        return Err(Error::new(
                                            ErrorCode::TypeMismatch,
                                            "interpolation requires string, number, or boolean",
                                        )
                                        .with_path(path.as_slice()));
                                    }
                                }
                            }
                        }
                    }
                    Ok(EvalValue::String(out))
                }
            },
            AstValue::Number { value: text, .. } => Ok(EvalValue::Number(text)),
            AstValue::Bool { value, .. } => Ok(EvalValue::Bool(value)),
            AstValue::Null { .. } => Ok(EvalValue::Null),
            AstValue::Substitution { path, .. } => self.lookup_completed(
                path.as_slice(),
                &Location {
                    file: None,
                    line: 1,
                    column: 1,
                    span: Span::default(),
                },
            ),
        }
    }

    fn lookup_completed(&self, path: &[String], loc: &Location) -> Result<EvalValue> {
        self.lookup_completed_entry(path, loc)
            .map(|entry| entry.value.clone())
    }

    fn lookup_completed_entry(&self, path: &[String], loc: &Location) -> Result<&Entry> {
        if self.in_progress.iter().any(|p| p == path) {
            return Err(Error::new(
                ErrorCode::MissingReference,
                format!("path {:?} is not completed before use", path),
            )
            .at(loc.clone())
            .with_path(path));
        }
        lookup_entry(&self.root, path).ok_or_else(|| {
            Error::new(
                ErrorCode::MissingReference,
                format!("path {:?} is not defined before use", path),
            )
            .at(loc.clone())
            .with_path(path)
        })
    }
}

fn lookup_entry<'a>(object: &'a EvalObject, path: &[String]) -> Option<&'a Entry> {
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

fn object_mut_at<'a>(object: &'a mut EvalObject, path: &[String]) -> Option<&'a mut EvalObject> {
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

fn ensure_local_object(object: &mut EvalObject, path: &[String], loc: &Location) -> Result<()> {
    let Some((first, rest)) = path.split_first() else {
        return Ok(());
    };
    if rest.is_empty() {
        match object.get_mut(first) {
            None => {
                object.insert(
                    first.clone(),
                    Entry {
                        value: EvalValue::Object(EvalObject::default()),
                        layer: Layer::Local,
                        kind: Kind::StructuralObject,
                    },
                );
            }
            Some(entry) if entry.layer == Layer::Base => {
                if !matches!(entry.value, EvalValue::Object(_)) {
                    entry.value = EvalValue::Object(EvalObject::default());
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
        value: EvalValue::Object(EvalObject::default()),
        layer: Layer::Local,
        kind: Kind::StructuralObject,
    });
    if !matches!(child.value, EvalValue::Object(_)) {
        if child.layer == Layer::Base {
            child.value = EvalValue::Object(EvalObject::default());
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
    object: &mut EvalObject,
    path: &[String],
    value: EvalValue,
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
            value,
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

fn insert_local_value_owned(
    object: &mut EvalObject,
    path: SconPath,
    value: EvalValue,
    kind: Kind,
    loc: &Location,
) -> Result<()> {
    if path.len() == 1 {
        let mut iter = path.into_iter();
        let key = iter.next().expect("path length checked");
        let entry = Entry {
            value,
            layer: Layer::Local,
            kind,
        };
        match object.entry(key) {
            IndexEntry::Vacant(vacant) => {
                vacant.insert(entry);
                Ok(())
            }
            IndexEntry::Occupied(mut occupied) if occupied.get().layer == Layer::Base => {
                overlay_replace(occupied.get_mut(), entry);
                Ok(())
            }
            IndexEntry::Occupied(occupied) => {
                let path = vec![occupied.key().clone()];
                Err(Error::new(ErrorCode::DuplicateKey, "duplicate local key")
                    .at(loc.clone())
                    .with_path(&path))
            }
        }
    } else {
        insert_local_value(object, path.as_slice(), value, kind, loc)
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

fn overlay_base(left: &mut EvalObject, right: EvalObject) {
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

fn clone_object_with_layer(object: &EvalObject, layer: Layer, kind: Kind) -> EvalObject {
    let mut out = EvalObject::with_capacity_and_hasher(object.len(), FxBuildHasher);
    for (key, value) in object {
        out.insert(
            key.clone(),
            Entry {
                value: clone_value_with_layer(&value.value, layer, kind),
                layer,
                kind,
            },
        );
    }
    out
}

fn clone_value_with_layer(value: &EvalValue, layer: Layer, kind: Kind) -> EvalValue {
    match value {
        EvalValue::Null => EvalValue::Null,
        EvalValue::Bool(value) => EvalValue::Bool(*value),
        EvalValue::Number(value) => EvalValue::Number(value.clone()),
        EvalValue::String(value) => EvalValue::String(value.clone()),
        EvalValue::Array(value) => EvalValue::Array(value.clone()),
        EvalValue::Object(value) => EvalValue::Object(clone_object_with_layer(value, layer, kind)),
    }
}

fn into_public_object(object: EvalObject) -> IndexMap<String, Value> {
    let mut out = IndexMap::with_capacity(object.len());
    for (key, value) in object {
        out.insert(key, entry_into_public(value));
    }
    out
}

fn entry_into_public(entry: Entry) -> Value {
    eval_value_into_public(entry.value)
}

fn eval_value_into_public(value: EvalValue) -> Value {
    match value {
        EvalValue::Null => Value::Null,
        EvalValue::Bool(value) => Value::Bool(value),
        EvalValue::Number(value) => Value::Number(value),
        EvalValue::String(value) => Value::String(value),
        EvalValue::Array(values) => {
            Value::Array(values.into_iter().map(eval_value_into_public).collect())
        }
        EvalValue::Object(value) => Value::Object(into_public_object(value)),
    }
}
