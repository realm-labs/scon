#![allow(dead_code)]

use std::path::PathBuf;

pub type SconPath = smallvec::SmallVec<[String; 2]>;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Span {
    pub start_byte: usize,
    pub end_byte: usize,
}

impl Span {
    pub fn new(start_byte: usize, end_byte: usize) -> Self {
        Self {
            start_byte,
            end_byte,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Location {
    pub file: Option<PathBuf>,
    pub line: usize,
    pub column: usize,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct Document {
    pub body: ObjectBody,
    pub file: Option<PathBuf>,
    pub span: Span,
}

#[derive(Clone, Debug, Default)]
pub struct ObjectBody {
    pub spreads: Vec<ObjectSpread>,
    pub members: Vec<LocalMember>,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub struct ObjectSpread {
    pub path: SconPath,
    pub path_span: Span,
    pub loc: Location,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum LocalMember {
    Include {
        path: String,
        path_span: Span,
        loc: Location,
        span: Span,
    },
    Field(Field),
}

#[derive(Clone, Debug)]
pub struct Field {
    pub path: SconPath,
    pub path_span: Span,
    pub value: AstValue,
    pub loc: Location,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub enum AstValue {
    Object {
        body: ObjectBody,
        span: Span,
    },
    Array {
        items: Vec<ArrayItem>,
        span: Span,
    },
    String {
        value: StringValue,
        span: Span,
    },
    Number {
        value: String,
        span: Span,
    },
    Bool {
        value: bool,
        span: Span,
    },
    Null {
        span: Span,
    },
    Substitution {
        path: SconPath,
        path_span: Span,
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub enum StringPart {
    Literal(String),
    Interpolation {
        path: SconPath,
        path_span: Span,
        span: Span,
    },
}

#[derive(Clone, Debug)]
pub enum StringValue {
    Literal(String),
    Parts(Vec<StringPart>),
}

#[derive(Clone, Debug)]
pub enum ArrayItem {
    Value(AstValue),
    Spread {
        path: SconPath,
        path_span: Span,
        loc: Location,
        span: Span,
    },
}
