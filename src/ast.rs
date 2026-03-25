use serde::Serialize;
use serde_json::Number;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Formula {
    pub lhs: String,
    pub rhs: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invariant {
    pub field: String,
    pub min: String,
    pub max: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineOp {
    pub name: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentAst {
    pub sections: Vec<Section>,
    pub meta: Vec<KeyValue>,
    pub formulas: Vec<Formula>,
    pub invariants: Vec<Invariant>,
    pub pipeline: Vec<PipelineOp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContractFormula {
    pub lhs: String,
    pub rhs: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContractInvariant {
    pub field: String,
    pub min: Number,
    pub max: Number,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Contract {
    pub meta: BTreeMap<String, String>,
    pub formulas: Vec<ContractFormula>,
    pub invariants: Vec<ContractInvariant>,
    pub pipeline: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    UnknownSection,
    InvalidSyntax,
    DuplicateSection,
    UnknownField,
    InvalidFormula,
    InvalidInvariant,
    UnknownVariable,
    UnknownOp,
    MissingMeta,
    EmptyFormula,
    MissingPipeline,
    DuplicateMetaKey,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::UnknownSection => "UNKNOWN_SECTION",
            ErrorCode::InvalidSyntax => "INVALID_SYNTAX",
            ErrorCode::DuplicateSection => "DUPLICATE_SECTION",
            ErrorCode::UnknownField => "UNKNOWN_FIELD",
            ErrorCode::InvalidFormula => "INVALID_FORMULA",
            ErrorCode::InvalidInvariant => "INVALID_INVARIANT",
            ErrorCode::UnknownVariable => "UNKNOWN_VARIABLE",
            ErrorCode::UnknownOp => "UNKNOWN_OP",
            ErrorCode::MissingMeta => "MISSING_META",
            ErrorCode::EmptyFormula => "EMPTY_FORMULA",
            ErrorCode::MissingPipeline => "MISSING_PIPELINE",
            ErrorCode::DuplicateMetaKey => "DUPLICATE_META_KEY",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileError {
    pub code: ErrorCode,
    pub message: String,
    pub line: Option<usize>,
}

impl CompileError {
    pub fn new(code: ErrorCode, message: impl Into<String>, line: Option<usize>) -> Self {
        Self {
            code,
            message: message.into(),
            line,
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(f, "{}: {} (line {})", self.code, self.message, line),
            None => write!(f, "{}: {}", self.code, self.message),
        }
    }
}

impl Error for CompileError {}

pub type Result<T> = std::result::Result<T, CompileError>;
