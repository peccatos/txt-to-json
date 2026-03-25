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
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const DEFAULT_OUTPUT_PATH: &str = "вывод.json";
const VERSION: &str = env!("CARGO_PKG_VERSION");
const USAGE: &str = "\
txt-to-json - EVA DSL compiler

Usage:
  txt-to-json <command> <path>
  txt-to-json --help
  txt-to-json --version

Commands:
  compile <path>     Compile DSL and write ./вывод.json
  validate <path>    Parse and validate only
  print-ast <path>   Print AST as JSON
  ui                 Open the interactive terminal menu

Flags:
  -h, --help         Show this help
  -V, --version      Print version
";

#[derive(Debug, Clone, PartialEq, Eq)]
enum CliCommand {
    Compile { input: PathBuf },
    Validate { input: PathBuf },
    PrintAst { input: PathBuf },
    Interactive,
    Help,
    Version,
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
        CliCommand::Help => {
            println!("{USAGE}");
            Ok(())
        }
        CliCommand::Version => {
            println!("txt-to-json {VERSION}");
            Ok(())
        }
        CliCommand::Interactive => run_interactive(),
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
        return Ok(CliCommand::Help);
    };

    let Some(command) = command.to_str() else {
        return Err(cli_syntax_error("subcommand must be valid UTF-8"));
    };

    match command {
        "-h" | "--help" | "help" => {
            if args.next().is_some() {
                return Err(cli_syntax_error("unexpected extra arguments"));
            }
            return Ok(CliCommand::Help);
        }
        "-V" | "--version" | "version" => {
            if args.next().is_some() {
                return Err(cli_syntax_error("unexpected extra arguments"));
            }
            return Ok(CliCommand::Version);
        }
        "ui" | "interactive" | "menu" => {
            if args.next().is_some() {
                return Err(cli_syntax_error("unexpected extra arguments"));
            }
            return Ok(CliCommand::Interactive);
        }
        _ => {}
    }

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

fn run_interactive() -> Result<()> {
    println!("txt-to-json interactive terminal interface");
    println!("Type a number or command name, or `q` to quit.");

    loop {
        println!();
        println!("1) compile");
        println!("2) validate");
        println!("3) print-ast");
        println!("q) quit");

        let Some(choice) = read_prompt("Select: ")? else {
            return Ok(());
        };

        let choice = choice.trim().to_lowercase();
        if choice.is_empty() {
            continue;
        }

        if matches!(choice.as_str(), "q" | "quit" | "exit") {
            return Ok(());
        }

        let action = match choice.as_str() {
            "1" | "compile" => Some(InteractiveAction::Compile),
            "2" | "validate" => Some(InteractiveAction::Validate),
            "3" | "print-ast" | "ast" => Some(InteractiveAction::PrintAst),
            _ => {
                eprintln!("unknown choice: {choice}");
                None
            }
        };

        if let Some(action) = action {
            if let Err(err) = run_interactive_action(action) {
                print_error(&err);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InteractiveAction {
    Compile,
    Validate,
    PrintAst,
}

fn run_interactive_action(action: InteractiveAction) -> Result<()> {
    let Some(path_input) = read_prompt("Input path: ")? else {
        return Ok(());
    };

    let path_input = path_input.trim();
    if path_input.is_empty() {
        print_error(&cli_syntax_error("expected an input path"));
        return Ok(());
    }

    let path = PathBuf::from(path_input);
    match action {
        InteractiveAction::Compile => {
            let json = compile_file_to_json(&path)?;
            fs::write(DEFAULT_OUTPUT_PATH, json).map_err(CompileError::from)?;
            println!("compiled and wrote ./вывод.json");
        }
        InteractiveAction::Validate => {
            let _ = load_and_validate(&path)?;
            println!("validation ok");
        }
        InteractiveAction::PrintAst => {
            let ast = load_and_validate(&path)?;
            let json = serde_json::to_string_pretty(&ast).map_err(|err| {
                CompileError::new(
                    ErrorKind::InvalidSyntax,
                    format!("json serialization failed: {err}"),
                    0,
                    None,
                )
            })?;
            println!("{json}");
        }
    }

    Ok(())
}

fn read_prompt(prompt: &str) -> Result<Option<String>> {
    print!("{prompt}");
    io::stdout().flush().map_err(CompileError::from)?;

    let mut line = String::new();
    let bytes = io::stdin()
        .read_line(&mut line)
        .map_err(CompileError::from)?;
    if bytes == 0 {
        return Ok(None);
    }

    Ok(Some(line.trim().to_owned()))
}

fn print_error(err: &CompileError) {
    match serde_json::to_string_pretty(err) {
        Ok(json) => eprintln!("{json}"),
        Err(_) => eprintln!("{err}"),
    }
}
