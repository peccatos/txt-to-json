# txt-to-json

Bilingual README. English first, then Russian.

## English

`txt-to-json` is a strict Rust CLI compiler for an EVA-style DSL.

It parses a plain-text input file, validates it against a fixed contract, and produces a deterministic JSON result.

### Features

- deterministic output for identical input;
- fail-fast behavior on invalid syntax or validation errors;
- structured JSON errors on stderr;
- no implicit assumptions or silent corrections;
- `compile` writes the result to `./вывод.json` in the current working directory.

### Installation

```bash
cargo build
```

### CLI

#### Compile

Compiles a DSL file into a JSON contract and writes it to `./вывод.json`.

```bash
cargo run -- compile example.eva
```

#### Validate

Parses and validates the input without writing any files.

```bash
cargo run -- validate example.eva
```

#### Print AST

Prints the parsed AST as deterministic JSON.

```bash
cargo run -- print-ast example.eva
```

### DSL Format

Each file is split into sections. A section starts with `section: IDENT` and continues until the next section header or end of file.

Supported sections:

- `meta`
- `formula`
- `invariant`
- `pipeline`

#### `meta`

```text
section: meta
contract: calibration
version: v1
```

Line format:

```text
IDENT: IDENT
```

#### `formula`

```text
section: formula
confidence = confidence * (1 - prediction_error)
```

Line format:

```text
IDENT = EXPR
```

Supported expressions:

- numbers;
- variables;
- binary operators `+`, `-`, `*`, `/`;
- parentheses.

#### `invariant`

```text
section: invariant
confidence in [0,1]
```

Line format:

```text
IDENT in [NUMBER, NUMBER]
```

#### `pipeline`

```text
section: pipeline
op confidence_update
```

Line format:

```text
op IDENT
```

### Validation Rules

- the `meta` section is required;
- the `formula` section is required;
- the `pipeline` section is required;
- at least one formula must exist;
- unknown sections are rejected;
- duplicate sections are rejected;
- duplicate keys in `meta` are rejected;
- variables in formulas and invariants must be in the allowed list;
- pipeline operations must be in the registered list;
- each invariant must satisfy `min <= max`.

#### Allowed Variables

- `confidence`
- `prediction_error`
- `score`
- `risk`
- `probability`
- `expected_value`
- `reward_weight`
- `risk_weight`

#### Allowed Operations

- `update_ema_error`
- `update_beliefs`
- `confidence_update`
- `expected_value`
- `selection_score`

### Error Format

On failure, the CLI prints structured JSON to stderr.

Example:

```json
{
  "kind": "UnknownVariable",
  "message": "variable not allowed: hidden_signal",
  "line": 6,
  "column": 1
}
```

Fields:

- `kind`
- `message`
- `line`
- `column` is optional

### Example

Input:

```text
section: meta
contract: calibration
version: v1

section: formula
confidence = confidence * (1 - prediction_error)

section: invariant
confidence in [0,1]

section: pipeline
op confidence_update
```

Command:

```bash
cargo run -- compile example.eva
```

Output written to `./вывод.json`:

```json
{
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
}
```

### Development

```bash
cargo test
```

### Project Layout

- `src/main.rs` - CLI entry point and file I/O;
- `src/parser.rs` - section and line parsing;
- `src/lexer.rs` - expression lexer and parser;
- `src/ast.rs` - AST and contract models;
- `src/validator.rs` - strict validation rules;
- `src/builder.rs` - final JSON contract builder;
- `src/error.rs` - typed error model.

## Русский

`txt-to-json` - строгий Rust CLI-компилятор для DSL в стиле EVA.

Он читает текстовый файл, проверяет его по фиксированному контракту и выдаёт детерминированный JSON.

### Возможности

- детерминированный результат для одинакового входа;
- fail-fast при ошибках синтаксиса или валидации;
- структурированные JSON-ошибки в stderr;
- никаких неявных допущений и "исправлений" входа;
- `compile` пишет результат в `./вывод.json` в текущем рабочем каталоге.

### Установка

```bash
cargo build
```

### CLI

#### Compile

Компилирует DSL-файл в JSON-контракт и пишет его в `./вывод.json`.

```bash
cargo run -- compile example.eva
```

#### Validate

Парсит и валидирует вход, не создавая файлов.

```bash
cargo run -- validate example.eva
```

#### Print AST

Печатает AST в детерминированном JSON-формате.

```bash
cargo run -- print-ast example.eva
```

### Формат DSL

Каждый файл разбивается на секции. Секция начинается строкой `section: IDENT` и продолжается до следующего заголовка секции или конца файла.

Поддерживаются только секции:

- `meta`
- `formula`
- `invariant`
- `pipeline`

#### `meta`

```text
section: meta
contract: calibration
version: v1
```

Формат строки:

```text
IDENT: IDENT
```

#### `formula`

```text
section: formula
confidence = confidence * (1 - prediction_error)
```

Формат строки:

```text
IDENT = EXPR
```

Поддерживаемые выражения:

- числа;
- переменные;
- бинарные операторы `+`, `-`, `*`, `/`;
- круглые скобки.

#### `invariant`

```text
section: invariant
confidence in [0,1]
```

Формат строки:

```text
IDENT in [NUMBER, NUMBER]
```

#### `pipeline`

```text
section: pipeline
op confidence_update
```

Формат строки:

```text
op IDENT
```

### Правила валидации

- секция `meta` обязательна;
- секция `formula` обязательна;
- секция `pipeline` обязательна;
- должна быть хотя бы одна формула;
- неизвестные секции запрещены;
- дубли секций запрещены;
- дубли ключей в `meta` запрещены;
- переменные в формулах и инвариантах должны быть из разрешённого списка;
- операции в `pipeline` должны быть из зарегистрированного списка;
- каждый инвариант должен удовлетворять `min <= max`.

#### Разрешённые переменные

- `confidence`
- `prediction_error`
- `score`
- `risk`
- `probability`
- `expected_value`
- `reward_weight`
- `risk_weight`

#### Разрешённые операции

- `update_ema_error`
- `update_beliefs`
- `confidence_update`
- `expected_value`
- `selection_score`

### Формат ошибок

При ошибке CLI печатает структурированный JSON в stderr.

Пример:

```json
{
  "kind": "UnknownVariable",
  "message": "variable not allowed: hidden_signal",
  "line": 6,
  "column": 1
}
```

Поля:

- `kind`
- `message`
- `line`
- `column` - опционально

### Пример

Вход:

```text
section: meta
contract: calibration
version: v1

section: formula
confidence = confidence * (1 - prediction_error)

section: invariant
confidence in [0,1]

section: pipeline
op confidence_update
```

Команда:

```bash
cargo run -- compile example.eva
```

Вывод записывается в `./вывод.json`:

```json
{
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
}
```

### Разработка

```bash
cargo test
```

### Структура проекта

- `src/main.rs` - точка входа CLI и файловый ввод/вывод;
- `src/parser.rs` - разбор секций и строк;
- `src/lexer.rs` - лексер и парсер выражений;
- `src/ast.rs` - AST и модели контракта;
- `src/validator.rs` - строгие правила валидации;
- `src/builder.rs` - сборщик итогового JSON-контракта;
- `src/error.rs` - типизированная модель ошибок.
