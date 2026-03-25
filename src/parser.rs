use crate::ast::{
    CompileError, DocumentAst, ErrorCode, Formula, Invariant, KeyValue, PipelineOp, Result, Section,
};
use crate::lexer::{classify_line, LineKind};

const ALLOWED_SECTIONS: &[&str] = &["meta", "formula", "invariant", "pipeline"];

pub fn parse_document(input: &str) -> Result<DocumentAst> {
    let mut document = DocumentAst::default();
    let mut current_section: Option<String> = None;
    let mut seen_meta = false;
    let mut seen_formula = false;
    let mut seen_invariant = false;
    let mut seen_pipeline = false;

    for (index, raw_line) in input.lines().enumerate() {
        let line_no = index + 1;
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        let kind = classify_line(line).ok_or_else(|| invalid_syntax(line_no))?;
        match kind {
            LineKind::Section { name } => {
                if !ALLOWED_SECTIONS.contains(&name.as_str()) {
                    return Err(CompileError::new(
                        ErrorCode::UnknownSection,
                        "section not allowed",
                        Some(line_no),
                    ));
                }
                if section_seen(
                    &name,
                    seen_meta,
                    seen_formula,
                    seen_invariant,
                    seen_pipeline,
                ) {
                    return Err(CompileError::new(
                        ErrorCode::DuplicateSection,
                        format!("section already defined: {name}"),
                        Some(line_no),
                    ));
                }
                mark_seen(
                    &name,
                    &mut seen_meta,
                    &mut seen_formula,
                    &mut seen_invariant,
                    &mut seen_pipeline,
                );
                current_section = Some(name.clone());
                document.sections.push(Section {
                    name,
                    line: line_no,
                });
            }
            LineKind::KeyValue { key, value } => {
                let section = current_section
                    .as_deref()
                    .ok_or_else(|| invalid_syntax(line_no))?;
                if section != "meta" {
                    return Err(unknown_field(line_no, section));
                }
                document.meta.push(KeyValue {
                    key,
                    value,
                    line: line_no,
                });
            }
            LineKind::Formula { lhs, rhs } => {
                let section = current_section
                    .as_deref()
                    .ok_or_else(|| invalid_syntax(line_no))?;
                if section != "formula" {
                    return Err(unknown_field(line_no, section));
                }
                document.formulas.push(Formula {
                    lhs,
                    rhs,
                    line: line_no,
                });
            }
            LineKind::Invariant { field, min, max } => {
                let section = current_section
                    .as_deref()
                    .ok_or_else(|| invalid_syntax(line_no))?;
                if section != "invariant" {
                    return Err(unknown_field(line_no, section));
                }
                document.invariants.push(Invariant {
                    field,
                    min,
                    max,
                    line: line_no,
                });
            }
            LineKind::PipelineOp { name } => {
                let section = current_section
                    .as_deref()
                    .ok_or_else(|| invalid_syntax(line_no))?;
                if section != "pipeline" {
                    return Err(unknown_field(line_no, section));
                }
                document.pipeline.push(PipelineOp {
                    name,
                    line: line_no,
                });
            }
        }
    }

    Ok(document)
}

fn section_seen(
    name: &str,
    seen_meta: bool,
    seen_formula: bool,
    seen_invariant: bool,
    seen_pipeline: bool,
) -> bool {
    match name {
        "meta" => seen_meta,
        "formula" => seen_formula,
        "invariant" => seen_invariant,
        "pipeline" => seen_pipeline,
        _ => false,
    }
}

fn mark_seen(
    name: &str,
    seen_meta: &mut bool,
    seen_formula: &mut bool,
    seen_invariant: &mut bool,
    seen_pipeline: &mut bool,
) {
    match name {
        "meta" => *seen_meta = true,
        "formula" => *seen_formula = true,
        "invariant" => *seen_invariant = true,
        "pipeline" => *seen_pipeline = true,
        _ => {}
    }
}

fn invalid_syntax(line: usize) -> CompileError {
    CompileError::new(ErrorCode::InvalidSyntax, "syntax error", Some(line))
}

fn unknown_field(line: usize, section: &str) -> CompileError {
    CompileError::new(
        ErrorCode::UnknownField,
        format!("field not allowed in section `{section}`"),
        Some(line),
    )
}

#[cfg(test)]
mod tests {
    use super::parse_document;

    #[test]
    fn parses_example_structure() {
        let input = "section: meta\ncontract: calibration\nversion: v1\n\nsection: formula\nconfidence = confidence * (1 - prediction_error)\n\nsection: invariant\nconfidence in [0,1]\n\nsection: pipeline\nop confidence_update\n";
        let document = parse_document(input).expect("document should parse");
        assert_eq!(document.sections.len(), 4);
        assert_eq!(document.meta.len(), 2);
        assert_eq!(document.formulas.len(), 1);
        assert_eq!(document.invariants.len(), 1);
        assert_eq!(document.pipeline.len(), 1);
    }
}
