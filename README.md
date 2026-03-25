# txt-to-json

Rust CLI for compiling a strict EVA-style DSL into a JSON contract.

## Commands

- `cargo run -- compile <path>` writes the compiled contract to `./вывод.json` in the current working directory.
- `cargo run -- validate <path>` parses and validates the input without writing any files.
- `cargo run -- print-ast <path>` prints the parsed AST as deterministic JSON.

## Notes

- Output paths are always relative to the current working directory.
- The compiler is strict: unknown sections, malformed formulas, unknown variables, duplicate meta keys, and invalid invariants fail fast with structured JSON errors.
