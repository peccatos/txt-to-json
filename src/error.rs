use serde::Serialize;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ErrorKind {
    UnknownSection,
    InvalidSyntax,
    InvalidFormula,
    InvalidInvariant,
    UnknownVariable,
    UnknownOp,
    MissingMeta,
    MissingFormula,
    DuplicateMetaKey,
    IoError,
}

impl ErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorKind::UnknownSection => "UnknownSection",
            ErrorKind::InvalidSyntax => "InvalidSyntax",
            ErrorKind::InvalidFormula => "InvalidFormula",
            ErrorKind::InvalidInvariant => "InvalidInvariant",
            ErrorKind::UnknownVariable => "UnknownVariable",
            ErrorKind::UnknownOp => "UnknownOp",
            ErrorKind::MissingMeta => "MissingMeta",
            ErrorKind::MissingFormula => "MissingFormula",
            ErrorKind::DuplicateMetaKey => "DuplicateMetaKey",
            ErrorKind::IoError => "IoError",
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompileError {
    pub kind: ErrorKind,
    pub message: String,
    pub line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
}

impl CompileError {
    pub fn new(
        kind: ErrorKind,
        message: impl Into<String>,
        line: usize,
        column: Option<usize>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            line,
            column,
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.column {
            Some(column) => write!(
                f,
                "{}: {} (line {}, column {})",
                self.kind, self.message, self.line, column
            ),
            None => write!(f, "{}: {} (line {})", self.kind, self.message, self.line),
        }
    }
}

impl Error for CompileError {}

impl From<std::io::Error> for CompileError {
    fn from(err: std::io::Error) -> Self {
        Self::new(ErrorKind::IoError, err.to_string(), 0, None)
    }
}

pub type Result<T> = std::result::Result<T, CompileError>;
