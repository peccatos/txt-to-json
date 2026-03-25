mod ast;
mod builder;
mod error;
mod lexer;
mod parser;
mod validator;

use crate::ast::{Contract, DocumentAst};
use crate::error::{CompileError, ErrorKind, Result};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_OUTPUT_PATH: &str = "вывод.json";

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Compile { input: PathBuf },
    Validate { input: PathBuf },
    PrintAst { input: PathBuf },
}

pub fn compile_to_contract(input: &str) -> Result<Contract> {
    let document = parser::parse_document(input)?;
    let validated = validator::validate_document(document)?;
    Ok(builder::build_contract(validated))
}

pub fn parse_and_validate(input: &str) -> Result<DocumentAst> {
    let document = parser::parse_document(input)?;
    validator::validate_document(document.clone())?;
    Ok(document)
}

pub fn compile_to_json(input: &str) -> Result<String> {
    let contract = compile_to_contract(input)?;
    serde_json::to_string_pretty(&contract).map_err(|err| {
        CompileError::new(
            ErrorKind::InvalidSyntax,
            format!("json serialization failed: {err}"),
            0,
            None,
        )
    })
}

fn main() {
    if let Err(err) = run_cli() {
        print_error(&err);
        std::process::exit(1);
    }
}

fn run_cli() -> Result<()> {
    match parse_args(env::args_os().skip(1))? {
        CliCommand::Compile { input } => {
            let json = compile_file_to_json(&input)?;
            fs::write(DEFAULT_OUTPUT_PATH, json).map_err(CompileError::from)?;
            Ok(())
        }
        CliCommand::Validate { input } => {
            let _ = load_and_validate(&input)?;
            Ok(())
        }
        CliCommand::PrintAst { input } => {
            let ast = load_and_validate(&input)?;
            let json = serde_json::to_string_pretty(&ast).map_err(|err| {
                CompileError::new(
                    ErrorKind::InvalidSyntax,
                    format!("json serialization failed: {err}"),
                    0,
                    None,
                )
            })?;
            println!("{json}");
            Ok(())
        }
    }
}

fn parse_args(mut args: impl Iterator<Item = std::ffi::OsString>) -> Result<CliCommand> {
    let Some(command) = args.next() else {
        return Err(cli_syntax_error("expected a subcommand"));
    };

    let Some(command) = command.to_str() else {
        return Err(cli_syntax_error("subcommand must be valid UTF-8"));
    };

    let Some(input) = args.next() else {
        return Err(cli_syntax_error("expected an input path"));
    };

    if args.next().is_some() {
        return Err(cli_syntax_error("unexpected extra arguments"));
    }

    let input = PathBuf::from(input);
    match command {
        "compile" => Ok(CliCommand::Compile { input }),
        "validate" => Ok(CliCommand::Validate { input }),
        "print-ast" => Ok(CliCommand::PrintAst { input }),
        _ => Err(cli_syntax_error("unknown subcommand")),
    }
}

fn load_and_validate(path: &Path) -> Result<DocumentAst> {
    let input = fs::read_to_string(path).map_err(CompileError::from)?;
    parse_and_validate(&input)
}

fn compile_file_to_json(path: &Path) -> Result<String> {
    let input = fs::read_to_string(path).map_err(CompileError::from)?;
    compile_to_json(&input)
}

fn cli_syntax_error(message: impl Into<String>) -> CompileError {
    CompileError::new(ErrorKind::InvalidSyntax, message, 0, None)
}

fn print_error(err: &CompileError) {
    match serde_json::to_string_pretty(err) {
        Ok(json) => eprintln!("{json}"),
        Err(_) => eprintln!("{err}"),
    }
}
