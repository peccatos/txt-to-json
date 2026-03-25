mod ast;
mod builder;
mod lexer;
mod parser;
mod validator;

use crate::ast::{CompileError, ErrorCode, Result};
use std::env;
use std::fs;
use std::io::{self, Read};

pub fn compile_to_json(input: &str) -> Result<String> {
    let document = parser::parse_document(input)?;
    let validated = validator::validate_document(document)?;
    let contract = builder::build_contract(validated);
    serde_json::to_string_pretty(&contract).map_err(|err| {
        CompileError::new(
            ErrorCode::InvalidSyntax,
            format!("json serialization failed: {err}"),
            None,
        )
    })
}

fn read_input() -> io::Result<String> {
    let mut args = env::args().skip(1);
    if let Some(path) = args.next() {
        if args.next().is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "expected zero or one positional argument",
            ));
        }
        fs::read_to_string(path)
    } else {
        let mut input = String::new();
        io::stdin().read_to_string(&mut input)?;
        Ok(input)
    }
}

fn main() {
    match read_input().and_then(|input| {
        compile_to_json(&input).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }) {
        Ok(json) => {
            println!("{json}");
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::compile_to_json;
    use crate::ast::ErrorCode;
    use serde_json::{json, Value};

    #[test]
    fn compiles_example_contract() {
        let input = "section: meta\ncontract: calibration\nversion: v1\n\nsection: formula\nconfidence = confidence * (1 - prediction_error)\n\nsection: invariant\nconfidence in [0,1]\n\nsection: pipeline\nop confidence_update\n";
        let output = compile_to_json(input).expect("should compile");
        let value: Value = serde_json::from_str(&output).expect("valid json");
        let expected = json!({
            "meta": {
                "contract": "calibration",
                "version": "v1"
            },
            "formulas": [
                {
                    "lhs": "confidence",
                    "rhs": "confidence * (1 - prediction_error)"
                }
            ],
            "invariants": [
                {
                    "field": "confidence",
                    "min": 0,
                    "max": 1
                }
            ],
            "pipeline": [
                "confidence_update"
            ]
        });
        assert_eq!(value, expected);
    }

    #[test]
    fn rejects_unknown_section() {
        let input = "section: meta\ncontract: calibration\n\nsection: mystery\nvalue: x\n\nsection: formula\nconfidence = score\n\nsection: pipeline\nop confidence_update\n";
        let err = compile_to_json(input).unwrap_err();
        assert_eq!(err.code, ErrorCode::UnknownSection);
    }
}
