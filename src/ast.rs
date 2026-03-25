use serde::Serialize;
use serde_json::Number;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Section {
    pub name: String,
    #[serde(skip_serializing)]
    pub line: usize,
    #[serde(skip_serializing)]
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    #[serde(skip_serializing)]
    pub line: usize,
    #[serde(skip_serializing)]
    pub column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
}

impl BinaryOperator {
    pub const fn symbol(self) -> &'static str {
        match self {
            BinaryOperator::Add => "+",
            BinaryOperator::Sub => "-",
            BinaryOperator::Mul => "*",
            BinaryOperator::Div => "/",
        }
    }

    pub const fn precedence(self) -> u8 {
        match self {
            BinaryOperator::Add | BinaryOperator::Sub => 1,
            BinaryOperator::Mul | BinaryOperator::Div => 2,
        }
    }
}

impl fmt::Display for BinaryOperator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.symbol())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Expr {
    Number(Number),
    Variable(String),
    Binary {
        op: BinaryOperator,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Paren(Box<Expr>),
}

impl Expr {
    pub fn to_source(&self) -> String {
        self.render(0, false)
    }

    fn render(&self, parent_prec: u8, is_right_child: bool) -> String {
        match self {
            Expr::Number(number) => number.to_string(),
            Expr::Variable(name) => name.clone(),
            Expr::Paren(inner) => format!("({})", inner.render(0, false)),
            Expr::Binary { op, left, right } => {
                let precedence = op.precedence();
                let rendered = format!(
                    "{} {} {}",
                    left.render(precedence, false),
                    op.symbol(),
                    right.render(precedence, true)
                );

                if precedence < parent_prec || (precedence == parent_prec && is_right_child) {
                    format!("({rendered})")
                } else {
                    rendered
                }
            }
        }
    }

    pub fn visit_variables<F>(&self, visitor: &mut F)
    where
        F: FnMut(&str),
    {
        match self {
            Expr::Number(_) => {}
            Expr::Variable(name) => visitor(name),
            Expr::Binary { left, right, .. } => {
                left.visit_variables(visitor);
                right.visit_variables(visitor);
            }
            Expr::Paren(inner) => inner.visit_variables(visitor),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Formula {
    pub lhs: String,
    pub rhs: Expr,
    #[serde(skip_serializing)]
    pub line: usize,
    #[serde(skip_serializing)]
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Invariant {
    pub field: String,
    pub min: Number,
    pub max: Number,
    #[serde(skip_serializing)]
    pub line: usize,
    #[serde(skip_serializing)]
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PipelineOp {
    pub name: String,
    #[serde(skip_serializing)]
    pub line: usize,
    #[serde(skip_serializing)]
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContractInvariant {
    pub field: String,
    pub min: Number,
    pub max: Number,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Contract {
    pub meta: BTreeMap<String, String>,
    pub formulas: Vec<ContractFormula>,
    pub invariants: Vec<ContractInvariant>,
    pub pipeline: Vec<String>,
}
