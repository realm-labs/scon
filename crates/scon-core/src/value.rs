use std::fmt;

use indexmap::IndexMap;

use crate::error::{Error, ErrorCode, Result};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Number {
    I64(i64),
    U64(u64),
    F64(f64),
}

impl Number {
    pub fn from_i64(value: i64) -> Self {
        Self::I64(value)
    }

    pub fn from_u64(value: u64) -> Self {
        Self::U64(value)
    }

    pub fn from_f64(value: f64) -> Option<Self> {
        value.is_finite().then_some(Self::F64(value))
    }

    pub fn parse(text: &str) -> Result<Self> {
        if text.contains(['.', 'e', 'E']) {
            let value = text
                .parse::<f64>()
                .map_err(|err| invalid_number(format!("invalid float literal `{text}`: {err}")))?;
            return Self::from_f64(value)
                .ok_or_else(|| invalid_number(format!("non-finite float literal `{text}`")));
        }

        if text.starts_with('-') {
            text.parse::<i64>()
                .map(Self::I64)
                .map_err(|err| invalid_number(format!("invalid signed integer `{text}`: {err}")))
        } else {
            text.parse::<u64>()
                .map(Self::U64)
                .map_err(|err| invalid_number(format!("invalid unsigned integer `{text}`: {err}")))
        }
    }

    pub fn is_i64(&self) -> bool {
        matches!(self, Self::I64(_))
    }

    pub fn is_u64(&self) -> bool {
        matches!(self, Self::U64(_))
    }

    pub fn is_f64(&self) -> bool {
        matches!(self, Self::F64(_))
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::I64(value) => Some(*value),
            Self::U64(value) => (*value).try_into().ok(),
            Self::F64(_) => None,
        }
    }

    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::I64(value) => (*value).try_into().ok(),
            Self::U64(value) => Some(*value),
            Self::F64(_) => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::I64(value) => Some(*value as f64),
            Self::U64(value) => Some(*value as f64),
            Self::F64(value) => Some(*value),
        }
    }
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::I64(value) => write!(f, "{value}"),
            Self::U64(value) => write!(f, "{value}"),
            Self::F64(value) => {
                let text = value.to_string();
                if text.contains(['.', 'e', 'E']) {
                    f.write_str(&text)
                } else {
                    write!(f, "{text}.0")
                }
            }
        }
    }
}

fn invalid_number(message: String) -> Error {
    Error::new(ErrorCode::InvalidNumber, message)
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Value>),
    Object(IndexMap<String, Value>),
}
