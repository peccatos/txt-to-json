use crate::ast::{DocumentAst, KeyValue, Section};
use crate::error::{CompileError, ErrorKind, Result};
use crate::lexer::{
    parse_formula_line, parse_invariant_line, parse_key_value, parse_pipeline_line,
    parse_section_header,
};
use std::collections::HashSet;

const ALLOWED_SECTIONS: &[&str] = &["meta", "formula", "invariant", "pipeline"];

pub fn parse_document(input: &str) -> Result<DocumentAst> {
    let mut document = DocumentAst::default();
    let mut current_section: Option<String> = None;
    let mut seen_sections = HashSet::new();

    for (index, raw_line) in input.lines().enumerate() {
        let line_no = index + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(section) = parse_section_header(trimmed) {
            if !ALLOWED_SECTIONS.contains(&section.as_str()) {
                return Err(CompileError::new(
                    ErrorKind::UnknownSection,
                    "section not allowed",
                    line_no,
                    Some(1),
                ));
            }

            if !seen_sections.insert(section.clone()) {
                return Err(CompileError::new(
                    ErrorKind::InvalidSyntax,
                    "duplicate section",
                    line_no,
                    Some(1),
                ));
            }

            current_section = Some(section.clone());
            document.sections.push(Section {
                name: section,
                line: line_no,
                column: 1,
            });
            continue;
        }

        let Some(section_name) = current_section.as_deref() else {
            return Err(CompileError::new(
                ErrorKind::InvalidSyntax,
                "syntax error",
                line_no,
                Some(1),
            ));
        };

        match section_name {
            "meta" => match parse_key_value(trimmed) {
                Some((key, value)) => document.meta.push(KeyValue {
                    key,
                    value,
                    line: line_no,
                    column: 1,
                }),
                None => {
                    return Err(CompileError::new(
                        ErrorKind::InvalidSyntax,
                        "syntax error",
                        line_no,
                        Some(1),
                    ));
                }
            },
            "formula" => match parse_formula_line(trimmed, line_no)? {
                Some(formula) => document.formulas.push(formula),
                None => {
                    return Err(CompileError::new(
                        ErrorKind::InvalidFormula,
                        "malformed expression",
                        line_no,
                        Some(1),
                    ));
                }
            },
            "invariant" => match parse_invariant_line(trimmed, line_no)? {
                Some(invariant) => document.invariants.push(invariant),
                None => {
                    return Err(CompileError::new(
                        ErrorKind::InvalidInvariant,
                        "invalid range",
                        line_no,
                        Some(1),
                    ));
                }
            },
            "pipeline" => match parse_pipeline_line(trimmed, line_no)? {
                Some(op) => document.pipeline.push(op),
                None => {
                    return Err(CompileError::new(
                        ErrorKind::InvalidSyntax,
                        "syntax error",
                        line_no,
                        Some(1),
                    ));
                }
            },
            _ => unreachable!("parser only accepts known sections"),
        }
    }

    Ok(document)
}
