use crate::ast::{DocumentAst, Expr, Formula, Invariant, PipelineOp};
use crate::error::{CompileError, ErrorKind, Result};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedDocument {
    pub meta: BTreeMap<String, String>,
    pub formulas: Vec<Formula>,
    pub invariants: Vec<Invariant>,
    pub pipeline: Vec<PipelineOp>,
}

const ALLOWED_VARIABLES: &[&str] = &[
    "confidence",
    "prediction_error",
    "score",
    "risk",
    "probability",
    "expected_value",
    "reward_weight",
    "risk_weight",
];

const ALLOWED_OPS: &[&str] = &[
    "update_ema_error",
    "update_beliefs",
    "confidence_update",
    "expected_value",
    "selection_score",
];

pub fn validate_document(document: DocumentAst) -> Result<ValidatedDocument> {
    ensure_required_sections(&document)?;
    let meta = validate_meta(&document)?;
    let formulas = validate_formulas(&document)?;
    let invariants = validate_invariants(&document)?;
    let pipeline = validate_pipeline(&document)?;

    Ok(ValidatedDocument {
        meta,
        formulas,
        invariants,
        pipeline,
    })
}

fn ensure_required_sections(document: &DocumentAst) -> Result<()> {
    if !document
        .sections
        .iter()
        .any(|section| section.name == "meta")
    {
        return Err(CompileError::new(
            ErrorKind::MissingMeta,
            "meta section required",
            0,
            None,
        ));
    }

    if document.formulas.is_empty() {
        return Err(CompileError::new(
            ErrorKind::MissingFormula,
            "at least one formula required",
            0,
            None,
        ));
    }

    Ok(())
}

fn validate_meta(document: &DocumentAst) -> Result<BTreeMap<String, String>> {
    let mut seen = HashSet::new();
    let mut meta = BTreeMap::new();

    for entry in &document.meta {
        if !seen.insert(entry.key.as_str()) {
            return Err(CompileError::new(
                ErrorKind::DuplicateMetaKey,
                "duplicate meta key",
                entry.line,
                Some(entry.column),
            ));
        }

        meta.insert(entry.key.clone(), entry.value.clone());
    }

    Ok(meta)
}

fn validate_formulas(document: &DocumentAst) -> Result<Vec<Formula>> {
    let mut validated = Vec::with_capacity(document.formulas.len());

    for formula in &document.formulas {
        ensure_allowed_variable(&formula.lhs, formula.line, Some(formula.column))?;
        ensure_expression_variables_allowed(&formula.rhs, formula.line)?;
        validated.push(formula.clone());
    }

    Ok(validated)
}

fn validate_invariants(document: &DocumentAst) -> Result<Vec<Invariant>> {
    let mut validated = Vec::with_capacity(document.invariants.len());

    for invariant in &document.invariants {
        ensure_allowed_variable(&invariant.field, invariant.line, Some(invariant.column))?;

        let min = invariant.min.as_f64().ok_or_else(|| {
            CompileError::new(
                ErrorKind::InvalidInvariant,
                "invalid range",
                invariant.line,
                Some(invariant.column),
            )
        })?;
        let max = invariant.max.as_f64().ok_or_else(|| {
            CompileError::new(
                ErrorKind::InvalidInvariant,
                "invalid range",
                invariant.line,
                Some(invariant.column),
            )
        })?;

        if min > max {
            return Err(CompileError::new(
                ErrorKind::InvalidInvariant,
                "invalid range",
                invariant.line,
                Some(invariant.column),
            ));
        }

        validated.push(invariant.clone());
    }

    Ok(validated)
}

fn validate_pipeline(document: &DocumentAst) -> Result<Vec<PipelineOp>> {
    let mut validated = Vec::with_capacity(document.pipeline.len());

    for op in &document.pipeline {
        if !ALLOWED_OPS.contains(&op.name.as_str()) {
            return Err(CompileError::new(
                ErrorKind::UnknownOp,
                "operation not registered",
                op.line,
                Some(op.column),
            ));
        }

        validated.push(op.clone());
    }

    Ok(validated)
}

fn ensure_allowed_variable(name: &str, line: usize, column: Option<usize>) -> Result<()> {
    if ALLOWED_VARIABLES.contains(&name) {
        Ok(())
    } else {
        Err(CompileError::new(
            ErrorKind::UnknownVariable,
            format!("variable not allowed: {name}"),
            line,
            column,
        ))
    }
}

fn ensure_expression_variables_allowed(expr: &Expr, line: usize) -> Result<()> {
    let mut error: Option<CompileError> = None;
    expr.visit_variables(&mut |name| {
        if error.is_none() && !ALLOWED_VARIABLES.contains(&name) {
            error = Some(CompileError::new(
                ErrorKind::UnknownVariable,
                format!("variable not allowed: {name}"),
                line,
                None,
            ));
        }
    });

    match error {
        Some(err) => Err(err),
        None => Ok(()),
    }
}
