use crate::ast::{
    CompileError, ContractFormula, ContractInvariant, DocumentAst, ErrorCode, Result,
};
use crate::lexer::{parse_number_literal, validate_expression};
use serde_json::Number;
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedDocument {
    pub meta: BTreeMap<String, String>,
    pub formulas: Vec<ContractFormula>,
    pub invariants: Vec<ContractInvariant>,
    pub pipeline: Vec<String>,
}

const ALLOWED_VARIABLES: &[&str] = &[
    "confidence",
    "prediction_error",
    "score",
    "risk",
    "probability",
    "expected_value",
];

const ALLOWED_OPS: &[&str] = &[
    "update_ema_error",
    "update_beliefs",
    "confidence_update",
    "expected_value",
    "selection_score",
];

pub fn validate_document(document: DocumentAst) -> Result<ValidatedDocument> {
    ensure_meta_and_formula_present(&document)?;
    let meta = validate_meta(&document)?;
    let formulas = validate_formulas(&document)?;
    let invariants = validate_invariants(&document)?;
    ensure_pipeline_present(&document)?;
    let pipeline = validate_pipeline(&document)?;

    Ok(ValidatedDocument {
        meta,
        formulas,
        invariants,
        pipeline,
    })
}

fn ensure_meta_and_formula_present(document: &DocumentAst) -> Result<()> {
    if !document
        .sections
        .iter()
        .any(|section| section.name == "meta")
    {
        return Err(CompileError::new(
            ErrorCode::MissingMeta,
            "meta section required",
            None,
        ));
    }

    if document.formulas.is_empty() {
        return Err(CompileError::new(
            ErrorCode::EmptyFormula,
            "at least one formula required",
            None,
        ));
    }

    Ok(())
}

fn ensure_pipeline_present(document: &DocumentAst) -> Result<()> {
    if !document
        .sections
        .iter()
        .any(|section| section.name == "pipeline")
    {
        return Err(CompileError::new(
            ErrorCode::MissingPipeline,
            "pipeline section required",
            None,
        ));
    }

    Ok(())
}

fn validate_meta(document: &DocumentAst) -> Result<BTreeMap<String, String>> {
    let mut seen = HashSet::new();
    let mut meta = BTreeMap::new();

    for kv in &document.meta {
        if !seen.insert(kv.key.as_str()) {
            return Err(CompileError::new(
                ErrorCode::DuplicateMetaKey,
                "duplicate meta key",
                Some(kv.line),
            ));
        }
        meta.insert(kv.key.clone(), kv.value.clone());
    }

    Ok(meta)
}

fn validate_formulas(document: &DocumentAst) -> Result<Vec<ContractFormula>> {
    let mut validated = Vec::with_capacity(document.formulas.len());
    for formula in &document.formulas {
        if !is_allowed_variable(&formula.lhs) {
            return Err(CompileError::new(
                ErrorCode::UnknownVariable,
                format!("variable not allowed: {}", formula.lhs),
                Some(formula.line),
            ));
        }
        validate_expression(&formula.rhs, formula.line, ALLOWED_VARIABLES)?;
        validated.push(ContractFormula {
            lhs: formula.lhs.clone(),
            rhs: formula.rhs.clone(),
        });
    }
    Ok(validated)
}

fn validate_invariants(document: &DocumentAst) -> Result<Vec<ContractInvariant>> {
    let mut validated = Vec::with_capacity(document.invariants.len());
    for invariant in &document.invariants {
        if !is_allowed_variable(&invariant.field) {
            return Err(CompileError::new(
                ErrorCode::UnknownVariable,
                format!("variable not allowed: {}", invariant.field),
                Some(invariant.line),
            ));
        }

        let min: Number = parse_number_literal(&invariant.min, invariant.line)?;
        let max: Number = parse_number_literal(&invariant.max, invariant.line)?;
        let min_value = min.as_f64().ok_or_else(|| {
            CompileError::new(
                ErrorCode::InvalidInvariant,
                "invalid range",
                Some(invariant.line),
            )
        })?;
        let max_value = max.as_f64().ok_or_else(|| {
            CompileError::new(
                ErrorCode::InvalidInvariant,
                "invalid range",
                Some(invariant.line),
            )
        })?;
        if min_value > max_value {
            return Err(CompileError::new(
                ErrorCode::InvalidInvariant,
                "invalid range",
                Some(invariant.line),
            ));
        }

        validated.push(ContractInvariant {
            field: invariant.field.clone(),
            min,
            max,
        });
    }
    Ok(validated)
}

fn validate_pipeline(document: &DocumentAst) -> Result<Vec<String>> {
    let mut validated = Vec::with_capacity(document.pipeline.len());
    for op in &document.pipeline {
        if !ALLOWED_OPS.contains(&op.name.as_str()) {
            return Err(CompileError::new(
                ErrorCode::UnknownOp,
                format!("operation not registered: {}", op.name),
                Some(op.line),
            ));
        }
        validated.push(op.name.clone());
    }
    Ok(validated)
}

fn is_allowed_variable(name: &str) -> bool {
    ALLOWED_VARIABLES.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::validate_document;
    use crate::ast::ErrorCode;
    use crate::parser::parse_document;

    #[test]
    fn validates_example() {
        let input = "section: meta\ncontract: calibration\nversion: v1\n\nsection: formula\nconfidence = confidence * (1 - prediction_error)\n\nsection: invariant\nconfidence in [0,1]\n\nsection: pipeline\nop confidence_update\n";
        let document = parse_document(input).unwrap();
        let validated = validate_document(document).unwrap();
        assert_eq!(
            validated.meta.get("contract"),
            Some(&"calibration".to_string())
        );
        assert_eq!(validated.formulas.len(), 1);
        assert_eq!(validated.invariants.len(), 1);
        assert_eq!(validated.pipeline, vec!["confidence_update".to_string()]);
    }

    #[test]
    fn rejects_unknown_variable() {
        let input = "section: meta\ncontract: calibration\n\nsection: formula\nmystery = confidence + 1\n\nsection: pipeline\nop confidence_update\n";
        let document = parse_document(input).unwrap();
        let err = validate_document(document).unwrap_err();
        assert_eq!(err.code, ErrorCode::UnknownVariable);
    }
}
