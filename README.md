# txt-to-json

`txt-to-json` - строгий Rust CLI-компилятор для EVA-подобного DSL.

Он читает текстовый файл, проверяет его по жёсткой схеме и либо:

- собирает JSON-контракт;
- только валидирует вход;
- печатает AST в JSON.

## Возможности

- детерминированный результат для одинакового входа;
- fail-fast на любой ошибке синтаксиса или валидации;
- без неявных допущений и без "исправления" входа;
- структурированные ошибки в JSON;
- вывод `compile` всегда пишется в `./вывод.json` в текущем рабочем каталоге.

## Установка

```bash
cargo build
```

## CLI

### Compile

Компилирует DSL-файл в JSON-контракт и пишет результат в `./вывод.json`.

```bash
cargo run -- compile example.eva
```

### Validate

Проверяет файл, но ничего не пишет на диск.

```bash
cargo run -- validate example.eva
```

### Print AST

Печатает разобранный AST в детерминированном JSON-формате.

```bash
cargo run -- print-ast example.eva
```

## Формат DSL

Файл состоит из секций. Секция начинается строкой `section: IDENT`, после чего идут строки содержимого до следующего заголовка секции.

Поддерживаются только:

- `meta`
- `formula`
- `invariant`
- `pipeline`

### Секция `meta`

```text
section: meta
contract: calibration
version: v1
```

Формат строки:

```text
IDENT: IDENT
```

### Секция `formula`

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
- операции `+`, `-`, `*`, `/`;
- круглые скобки.

### Секция `invariant`

```text
section: invariant
confidence in [0,1]
```

Формат строки:

```text
IDENT in [NUMBER, NUMBER]
```

### Секция `pipeline`

```text
section: pipeline
op confidence_update
```

Формат строки:

```text
op IDENT
```

## Правила валидации

- секция `meta` обязательна;
- секция `formula` обязательна;
- секция `pipeline` обязательна;
- должна быть хотя бы одна формула;
- неизвестные секции запрещены;
- дубли секций запрещены;
- дубли ключей в `meta` запрещены;
- переменные в формулах и инвариантах должны быть из разрешённого списка;
- операции в `pipeline` должны быть из зарегистрированного списка;
- `min` в инварианте должен быть меньше или равен `max`.

### Разрешённые переменные

- `confidence`
- `prediction_error`
- `score`
- `risk`
- `probability`
- `expected_value`
- `reward_weight`
- `risk_weight`

### Разрешённые операции

- `update_ema_error`
- `update_beliefs`
- `confidence_update`
- `expected_value`
- `selection_score`

## Формат ошибок

При ошибке CLI печатает JSON в stderr.

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

## Пример

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

Результат будет записан в `./вывод.json`:

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

## Разработка

```bash
cargo test
```

## Структура проекта

- `src/main.rs` - CLI и файловый ввод/вывод;
- `src/parser.rs` - разбор секций и строк;
- `src/lexer.rs` - лексер и парсер выражений;
- `src/ast.rs` - AST и модель контракта;
- `src/validator.rs` - строгая проверка правил;
- `src/builder.rs` - сборка финального JSON-контракта;
- `src/error.rs` - типизированные ошибки.
