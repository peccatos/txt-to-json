use crate::ast::{BinaryOperator, Expr, Formula, Invariant, PipelineOp};
use crate::error::{CompileError, ErrorKind, Result};
use serde_json::Number;

pub fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

pub fn is_ident_continue(ch: char) -> bool {
    is_ident_start(ch) || ch.is_ascii_digit()
}

pub fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) if is_ident_start(first) => chars.all(is_ident_continue),
        _ => false,
    }
}

pub fn parse_section_header(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let (left, right) = trimmed.split_once(':')?;
    if left.trim() != "section" {
        return None;
    }

    let name = right.trim();
    if !is_identifier(name) {
        return None;
    }

    Some(name.to_string())
}

pub fn parse_key_value(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let (key, value) = trimmed.split_once(':')?;
    let key = key.trim();
    let value = value.trim();

    if !is_identifier(key) || !is_identifier(value) {
        return None;
    }

    Some((key.to_string(), value.to_string()))
}

pub fn parse_pipeline_line(line: &str, line_no: usize) -> Result<Option<PipelineOp>> {
    let trimmed = line.trim();
    let mut parts = trimmed.split_whitespace();

    match (parts.next(), parts.next(), parts.next()) {
        (Some("op"), Some(name), None) if is_identifier(name) => Ok(Some(PipelineOp {
            name: name.to_string(),
            line: line_no,
            column: 1,
        })),
        _ => Ok(None),
    }
}

pub fn parse_formula_line(line: &str, line_no: usize) -> Result<Option<Formula>> {
    let trimmed = line.trim();
    let Some((lhs_raw, rhs_raw)) = trimmed.split_once('=') else {
        return Ok(None);
    };

    let lhs = lhs_raw.trim();
    let rhs = rhs_raw.trim();
    if !is_identifier(lhs) || rhs.is_empty() {
        return Ok(None);
    }

    let expr = parse_expression(rhs, line_no, 1)?;
    Ok(Some(Formula {
        lhs: lhs.to_string(),
        rhs: expr,
        line: line_no,
        column: 1,
    }))
}

pub fn parse_invariant_line(line: &str, line_no: usize) -> Result<Option<Invariant>> {
    let trimmed = line.trim();
    let mut cursor = 0usize;
    let Some(field) = take_identifier(trimmed, &mut cursor) else {
        return Ok(None);
    };

    skip_ws(trimmed, &mut cursor);
    if !consume_word(trimmed, &mut cursor, "in") {
        return Ok(None);
    }
    skip_ws(trimmed, &mut cursor);
    if !consume_char(trimmed, &mut cursor, '[') {
        return Ok(None);
    }
    skip_ws(trimmed, &mut cursor);

    let min_start = cursor + 1;
    let min_text = take_number_literal(trimmed, &mut cursor)
        .ok_or_else(|| invalid_invariant(line_no, Some(min_start)))?;
    let min = parse_number_literal(&min_text, line_no, min_start, ErrorKind::InvalidInvariant)?;

    skip_ws(trimmed, &mut cursor);
    if !consume_char(trimmed, &mut cursor, ',') {
        return Err(invalid_invariant(line_no, Some(cursor + 1)));
    }
    skip_ws(trimmed, &mut cursor);

    let max_start = cursor + 1;
    let max_text = take_number_literal(trimmed, &mut cursor)
        .ok_or_else(|| invalid_invariant(line_no, Some(max_start)))?;
    let max = parse_number_literal(&max_text, line_no, max_start, ErrorKind::InvalidInvariant)?;

    skip_ws(trimmed, &mut cursor);
    if !consume_char(trimmed, &mut cursor, ']') {
        return Err(invalid_invariant(line_no, Some(cursor + 1)));
    }
    skip_ws(trimmed, &mut cursor);

    if cursor != trimmed.len() {
        return Err(invalid_invariant(line_no, Some(cursor + 1)));
    }

    Ok(Some(Invariant {
        field,
        min,
        max,
        line: line_no,
        column: 1,
    }))
}

pub fn parse_expression(text: &str, line_no: usize, start_column: usize) -> Result<Expr> {
    let tokens = lex_expression(text, line_no, start_column)?;
    let mut parser = ExprParser {
        tokens: &tokens,
        pos: 0,
        line_no,
    };
    let expr = parser.parse_additive()?;

    if parser.pos != tokens.len() {
        let column = tokens
            .get(parser.pos)
            .map(|token| token.column)
            .unwrap_or(start_column);
        return Err(invalid_formula(line_no, Some(column)));
    }

    Ok(expr)
}

fn invalid_formula(line_no: usize, column: Option<usize>) -> CompileError {
    CompileError::new(
        ErrorKind::InvalidFormula,
        "malformed expression",
        line_no,
        column,
    )
}

fn invalid_invariant(line_no: usize, column: Option<usize>) -> CompileError {
    CompileError::new(ErrorKind::InvalidInvariant, "invalid range", line_no, column)
}

fn parse_number_literal(
    text: &str,
    line_no: usize,
    column: usize,
    kind: ErrorKind,
) -> Result<Number> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(number_error(kind, line_no, column));
    }

    let normalized = trimmed.strip_prefix('+').unwrap_or(trimmed);
    let has_fraction =
        normalized.contains('.') || normalized.contains('e') || normalized.contains('E');

    if !has_fraction {
        if let Ok(value) = normalized.parse::<i64>() {
            return Ok(Number::from(value));
        }

        if let Ok(value) = normalized.parse::<u64>() {
            return Ok(Number::from(value));
        }
    }

    let value = normalized
        .parse::<f64>()
        .map_err(|_| number_error(kind, line_no, column))?;
    if !value.is_finite() {
        return Err(number_error(kind, line_no, column));
    }

    if value.fract() == 0.0 && value >= i64::MIN as f64 && value <= i64::MAX as f64 {
        return Ok(Number::from(value as i64));
    }

    Number::from_f64(value).ok_or_else(|| number_error(kind, line_no, column))
}

fn number_error(kind: ErrorKind, line_no: usize, column: usize) -> CompileError {
    CompileError::new(kind, "invalid number", line_no, Some(column))
}

fn lex_expression(text: &str, line_no: usize, start_column: usize) -> Result<Vec<ExprToken>> {
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0usize;
    let mut column = start_column;
    let mut tokens = Vec::new();

    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() {
            index += 1;
            column += 1;
            continue;
        }

        if is_ident_start(ch) {
            let start = index;
            let start_column = column;
            index += 1;
            column += 1;
            while index < chars.len() && is_ident_continue(chars[index]) {
                index += 1;
                column += 1;
            }
            tokens.push(ExprToken {
                kind: ExprTokenKind::Ident(chars[start..index].iter().collect()),
                column: start_column,
            });
            continue;
        }

        if ch.is_ascii_digit() {
            let start = index;
            let start_column = column;
            let end = consume_expr_number(&chars, index)
                .ok_or_else(|| invalid_formula(line_no, Some(start_column)))?;
            index = end;
            column += end - start;
            tokens.push(ExprToken {
                kind: ExprTokenKind::Number(chars[start..end].iter().collect()),
                column: start_column,
            });
            continue;
        }

        let kind = match ch {
            '+' => ExprTokenKind::Plus,
            '-' => ExprTokenKind::Minus,
            '*' => ExprTokenKind::Mul,
            '/' => ExprTokenKind::Div,
            '(' => ExprTokenKind::LParen,
            ')' => ExprTokenKind::RParen,
            _ => return Err(invalid_formula(line_no, Some(column))),
        };
        tokens.push(ExprToken { kind, column });
        index += 1;
        column += 1;
    }

    Ok(tokens)
}

fn consume_expr_number(chars: &[char], start: usize) -> Option<usize> {
    let mut index = start;
    while index < chars.len() && chars[index].is_ascii_digit() {
        index += 1;
    }

    if index < chars.len() && chars[index] == '.' {
        index += 1;
        let frac_start = index;
        while index < chars.len() && chars[index].is_ascii_digit() {
            index += 1;
        }
        if index == frac_start {
            return None;
        }
    }

    if index < chars.len() && matches!(chars[index], 'e' | 'E') {
        index += 1;
        if index < chars.len() && matches!(chars[index], '+' | '-') {
            index += 1;
        }
        let exp_start = index;
        while index < chars.len() && chars[index].is_ascii_digit() {
            index += 1;
        }
        if index == exp_start {
            return None;
        }
    }

    Some(index)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExprToken {
    kind: ExprTokenKind,
    column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExprTokenKind {
    Ident(String),
    Number(String),
    Plus,
    Minus,
    Mul,
    Div,
    LParen,
    RParen,
}

struct ExprParser<'a> {
    tokens: &'a [ExprToken],
    pos: usize,
    line_no: usize,
}

impl<'a> ExprParser<'a> {
    fn parse_additive(&mut self) -> Result<Expr> {
        let mut expr = self.parse_multiplicative()?;
        while let Some(op) = self.peek_binary_operator() {
            if !matches!(op, BinaryOperator::Add | BinaryOperator::Sub) {
                break;
            }
            self.pos += 1;
            let right = self.parse_multiplicative()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr> {
        let mut expr = self.parse_primary()?;
        while let Some(op) = self.peek_binary_operator() {
            if !matches!(op, BinaryOperator::Mul | BinaryOperator::Div) {
                break;
            }
            self.pos += 1;
            let right = self.parse_primary()?;
            expr = Expr::Binary {
                op,
                left: Box::new(expr),
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        let token = self
            .tokens
            .get(self.pos)
            .ok_or_else(|| invalid_formula(self.line_no, None))?
            .clone();

        match token.kind {
            ExprTokenKind::Ident(name) => {
                self.pos += 1;
                Ok(Expr::Variable(name))
            }
            ExprTokenKind::Number(text) => {
                self.pos += 1;
                Ok(Expr::Number(parse_number_literal(
                    &text,
                    self.line_no,
                    token.column,
                    ErrorKind::InvalidFormula,
                )?))
            }
            ExprTokenKind::LParen => {
                self.pos += 1;
                let inner = self.parse_additive()?;
                match self.tokens.get(self.pos) {
                    Some(ExprToken {
                        kind: ExprTokenKind::RParen,
                        ..
                    }) => {
                        self.pos += 1;
                        Ok(Expr::Paren(Box::new(inner)))
                    }
                    Some(token) => Err(invalid_formula(self.line_no, Some(token.column))),
                    None => Err(invalid_formula(self.line_no, Some(token.column + 1))),
                }
            }
            _ => Err(invalid_formula(self.line_no, Some(token.column))),
        }
    }

    fn peek_binary_operator(&self) -> Option<BinaryOperator> {
        match self.tokens.get(self.pos).map(|token| &token.kind) {
            Some(ExprTokenKind::Plus) => Some(BinaryOperator::Add),
            Some(ExprTokenKind::Minus) => Some(BinaryOperator::Sub),
            Some(ExprTokenKind::Mul) => Some(BinaryOperator::Mul),
            Some(ExprTokenKind::Div) => Some(BinaryOperator::Div),
            _ => None,
        }
    }
}

fn take_identifier(line: &str, cursor: &mut usize) -> Option<String> {
    let first = line[*cursor..].chars().next()?;
    if !is_ident_start(first) {
        return None;
    }

    let start = *cursor;
    *cursor += first.len_utf8();

    while let Some(ch) = line[*cursor..].chars().next() {
        if is_ident_continue(ch) {
            *cursor += ch.len_utf8();
        } else {
            break;
        }
    }

    Some(line[start..*cursor].to_string())
}

fn skip_ws(line: &str, cursor: &mut usize) {
    while let Some(ch) = line[*cursor..].chars().next() {
        if ch.is_whitespace() {
            *cursor += ch.len_utf8();
        } else {
            break;
        }
    }
}

fn consume_word(line: &str, cursor: &mut usize, word: &str) -> bool {
    if !line[*cursor..].starts_with(word) {
        return false;
    }

    let end = *cursor + word.len();
    let prev = if *cursor == 0 {
        None
    } else {
        line[..*cursor].chars().last()
    };
    let next = line[end..].chars().next();

    if prev.is_some_and(is_ident_continue) {
        return false;
    }
    if next.is_some_and(is_ident_continue) {
        return false;
    }

    *cursor = end;
    true
}

fn consume_char(line: &str, cursor: &mut usize, expected: char) -> bool {
    match line[*cursor..].chars().next() {
        Some(ch) if ch == expected => {
            *cursor += ch.len_utf8();
            true
        }
        _ => false,
    }
}

fn take_number_literal(line: &str, cursor: &mut usize) -> Option<String> {
    let start = *cursor;
    let mut index = *cursor;

    if matches!(line[index..].chars().next(), Some('+') | Some('-')) {
        index += line[index..].chars().next()?.len_utf8();
    }

    let mut digit_count = 0usize;
    while let Some(ch) = line[index..].chars().next() {
        if ch.is_ascii_digit() {
            index += ch.len_utf8();
            digit_count += 1;
        } else {
            break;
        }
    }

    if digit_count == 0 {
        return None;
    }

    if matches!(line[index..].chars().next(), Some('.')) {
        index += 1;
        let mut frac_digits = 0usize;
        while let Some(ch) = line[index..].chars().next() {
            if ch.is_ascii_digit() {
                index += ch.len_utf8();
                frac_digits += 1;
            } else {
                break;
            }
        }
        if frac_digits == 0 {
            return None;
        }
    }

    if matches!(line[index..].chars().next(), Some('e') | Some('E')) {
        index += 1;
        if matches!(line[index..].chars().next(), Some('+') | Some('-')) {
            index += 1;
        }
        let exp_start = index;
        while let Some(ch) = line[index..].chars().next() {
            if ch.is_ascii_digit() {
                index += ch.len_utf8();
            } else {
                break;
            }
        }
        if index == exp_start {
            return None;
        }
    }

    *cursor = index;
    Some(line[start..index].to_string())
}
