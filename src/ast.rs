use std::path::PathBuf;

pub type SconPath = smallvec::SmallVec<[String; 2]>;

#[derive(Clone, Debug)]
pub struct Location {
    pub file: Option<PathBuf>,
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug)]
pub struct Document {
    pub body: ObjectBody,
    pub file: Option<PathBuf>,
}

#[derive(Clone, Debug, Default)]
pub struct ObjectBody {
    pub spreads: Vec<ObjectSpread>,
    pub members: Vec<LocalMember>,
}

#[derive(Clone, Debug)]
pub struct ObjectSpread {
    pub path: SconPath,
    pub loc: Location,
}

#[derive(Clone, Debug)]
pub enum LocalMember {
    Include { path: String, loc: Location },
    Field(Field),
}

#[derive(Clone, Debug)]
pub struct Field {
    pub path: SconPath,
    pub value: AstValue,
    pub loc: Location,
}

#[derive(Clone, Debug)]
pub enum AstValue {
    Object(ObjectBody),
    Array(Vec<ArrayItem>),
    String(StringValue),
    Number(String),
    Bool(bool),
    Null,
    Substitution(SconPath),
}

#[derive(Clone, Debug)]
pub enum StringPart {
    Literal(String),
    Interpolation(SconPath),
}

#[derive(Clone, Debug)]
pub enum StringValue {
    Literal(String),
    Parts(Vec<StringPart>),
}

#[derive(Clone, Debug)]
pub enum ArrayItem {
    Value(AstValue),
    Spread { path: SconPath, loc: Location },
}
