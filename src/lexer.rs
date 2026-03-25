use crate::ast::{CompileError, ErrorCode, Result};
use serde_json::Number;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineKind {
    Section {
        name: String,
    },
    KeyValue {
        key: String,
        value: String,
    },
    Formula {
        lhs: String,
        rhs: String,
    },
    Invariant {
        field: String,
        min: String,
        max: String,
    },
    PipelineOp {
        name: String,
    },
}

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

pub fn classify_line(line: &str) -> Option<LineKind> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(name) = parse_section_header(trimmed) {
        return Some(LineKind::Section { name });
    }

    if let Some((lhs, rhs)) = parse_formula(trimmed) {
        return Some(LineKind::Formula { lhs, rhs });
    }

    if let Some((field, min, max)) = parse_invariant(trimmed) {
        return Some(LineKind::Invariant { field, min, max });
    }

    if let Some(name) = parse_pipeline(trimmed) {
        return Some(LineKind::PipelineOp { name });
    }

    if let Some((key, value)) = parse_key_value(trimmed) {
        return Some(LineKind::KeyValue { key, value });
    }

    None
}

pub fn validate_expression(expr: &str, line: usize, allowed_variables: &[&str]) -> Result<()> {
    let tokens = lex_expression(expr, line)?;
    let mut parser = ExprParser {
        tokens: &tokens,
        pos: 0,
        line,
        allowed_variables,
    };
    parser.parse_additive()?;
    if parser.pos != tokens.len() {
        return Err(CompileError::new(
            ErrorCode::InvalidFormula,
            "malformed expression",
            Some(line),
        ));
    }
    Ok(())
}

pub fn parse_number_literal(raw: &str, line: usize) -> Result<Number> {
    let trimmed = raw.trim();
    if !is_number_syntax(trimmed) {
        return Err(CompileError::new(
            ErrorCode::InvalidInvariant,
            "invalid range",
            Some(line),
        ));
    }

    let normalized = trimmed.strip_prefix('+').unwrap_or(trimmed);
    let has_fraction =
        normalized.contains('.') || normalized.contains('e') || normalized.contains('E');

    if !has_fraction {
        if let Ok(value) = normalized.parse::<i64>() {
            return Ok(Number::from(value));
        }
    }

    let value = normalized
        .parse::<f64>()
        .map_err(|_| CompileError::new(ErrorCode::InvalidInvariant, "invalid range", Some(line)))?;

    if !value.is_finite() {
        return Err(CompileError::new(
            ErrorCode::InvalidInvariant,
            "invalid range",
            Some(line),
        ));
    }

    if value.fract() == 0.0 && value >= i64::MIN as f64 && value <= i64::MAX as f64 {
        return Ok(Number::from(value as i64));
    }

    Number::from_f64(value)
        .ok_or_else(|| CompileError::new(ErrorCode::InvalidInvariant, "invalid range", Some(line)))
}

fn parse_section_header(line: &str) -> Option<String> {
    let (left, right) = line.split_once(':')?;
    if left.trim() != "section" {
        return None;
    }
    let name = right.trim();
    if !is_identifier(name) {
        return None;
    }
    Some(name.to_string())
}

fn parse_key_value(line: &str) -> Option<(String, String)> {
    let (left, right) = line.split_once(':')?;
    let key = left.trim();
    let value = right.trim();
    if !is_identifier(key) || !is_identifier(value) {
        return None;
    }
    Some((key.to_string(), value.to_string()))
}

fn parse_formula(line: &str) -> Option<(String, String)> {
    let (left, right) = line.split_once('=')?;
    let lhs = left.trim();
    if !is_identifier(lhs) {
        return None;
    }
    Some((lhs.to_string(), right.trim().to_string()))
}

fn parse_invariant(line: &str) -> Option<(String, String, String)> {
    let mut cursor = 0usize;
    let field = take_identifier(line, &mut cursor)?;
    skip_ws(line, &mut cursor);
    if !consume_word(line, &mut cursor, "in") {
        return None;
    }
    skip_ws(line, &mut cursor);
    if !consume_char(line, &mut cursor, '[') {
        return None;
    }

    let body = &line[cursor..];
    let comma_pos = body.find(',')?;
    let min = body[..comma_pos].trim().to_string();
    let after_comma = &body[comma_pos + 1..];
    let close_pos = after_comma.find(']')?;
    let max = after_comma[..close_pos].trim().to_string();

    if after_comma[close_pos + 1..].trim().is_empty() {
        Some((field, min, max))
    } else {
        None
    }
}

fn parse_pipeline(line: &str) -> Option<String> {
    let mut parts = line.split_whitespace();
    match (parts.next(), parts.next(), parts.next()) {
        (Some("op"), Some(name), None) if is_identifier(name) => Some(name.to_string()),
        _ => None,
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

fn is_number_syntax(raw: &str) -> bool {
    let chars: Vec<char> = raw.chars().collect();
    if chars.is_empty() {
        return false;
    }

    let mut index = 0usize;
    if matches!(chars.get(index), Some('+') | Some('-')) {
        index += 1;
    }

    let digit_start = index;
    while matches!(chars.get(index), Some(ch) if ch.is_ascii_digit()) {
        index += 1;
    }
    if index == digit_start {
        return false;
    }

    if matches!(chars.get(index), Some('.')) {
        index += 1;
        let frac_start = index;
        while matches!(chars.get(index), Some(ch) if ch.is_ascii_digit()) {
            index += 1;
        }
        if index == frac_start {
            return false;
        }
    }

    if matches!(chars.get(index), Some('e') | Some('E')) {
        index += 1;
        if matches!(chars.get(index), Some('+') | Some('-')) {
            index += 1;
        }
        let exp_start = index;
        while matches!(chars.get(index), Some(ch) if ch.is_ascii_digit()) {
            index += 1;
        }
        if index == exp_start {
            return false;
        }
    }

    index == chars.len()
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExprTokenKind {
    Ident(String),
    Number(String),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    LParen,
    RParen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExprToken {
    kind: ExprTokenKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExprTokenClass {
    Ident,
    Number,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    LParen,
    RParen,
}

struct ExprParser<'a> {
    tokens: &'a [ExprToken],
    pos: usize,
    line: usize,
    allowed_variables: &'a [&'a str],
}

impl<'a> ExprParser<'a> {
    fn parse_additive(&mut self) -> Result<()> {
        self.parse_multiplicative()?;
        while self.consume_class(ExprTokenClass::Plus) || self.consume_class(ExprTokenClass::Minus)
        {
            self.parse_multiplicative()?;
        }
        Ok(())
    }

    fn parse_multiplicative(&mut self) -> Result<()> {
        self.parse_unary()?;
        while self.consume_class(ExprTokenClass::Star)
            || self.consume_class(ExprTokenClass::Slash)
            || self.consume_class(ExprTokenClass::Percent)
        {
            self.parse_unary()?;
        }
        Ok(())
    }

    fn parse_unary(&mut self) -> Result<()> {
        while self.consume_class(ExprTokenClass::Plus) || self.consume_class(ExprTokenClass::Minus)
        {
        }
        self.parse_power()
    }

    fn parse_power(&mut self) -> Result<()> {
        self.parse_primary()?;
        if self.consume_class(ExprTokenClass::Caret) {
            self.parse_unary()?;
        }
        Ok(())
    }

    fn parse_primary(&mut self) -> Result<()> {
        match self.peek_kind() {
            Some(ExprTokenKind::Ident(name)) => {
                let name = name.clone();
                self.pos += 1;
                if !self.allowed_variables.contains(&name.as_str()) {
                    return Err(CompileError::new(
                        ErrorCode::UnknownVariable,
                        format!("variable not allowed: {name}"),
                        Some(self.line),
                    ));
                }
                Ok(())
            }
            Some(ExprTokenKind::Number(_)) => {
                self.pos += 1;
                Ok(())
            }
            Some(ExprTokenKind::LParen) => {
                self.pos += 1;
                self.parse_additive()?;
                match self.peek_kind() {
                    Some(ExprTokenKind::RParen) => {
                        self.pos += 1;
                        Ok(())
                    }
                    _ => Err(CompileError::new(
                        ErrorCode::InvalidFormula,
                        "malformed expression",
                        Some(self.line),
                    )),
                }
            }
            _ => Err(CompileError::new(
                ErrorCode::InvalidFormula,
                "malformed expression",
                Some(self.line),
            )),
        }
    }

    fn consume_class(&mut self, class: ExprTokenClass) -> bool {
        if self.peek_class() == Some(class) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn peek_class(&self) -> Option<ExprTokenClass> {
        self.peek_kind().map(classify_token)
    }

    fn peek_kind(&self) -> Option<&ExprTokenKind> {
        self.tokens.get(self.pos).map(|token| &token.kind)
    }
}

fn classify_token(kind: &ExprTokenKind) -> ExprTokenClass {
    match kind {
        ExprTokenKind::Ident(_) => ExprTokenClass::Ident,
        ExprTokenKind::Number(_) => ExprTokenClass::Number,
        ExprTokenKind::Plus => ExprTokenClass::Plus,
        ExprTokenKind::Minus => ExprTokenClass::Minus,
        ExprTokenKind::Star => ExprTokenClass::Star,
        ExprTokenKind::Slash => ExprTokenClass::Slash,
        ExprTokenKind::Percent => ExprTokenClass::Percent,
        ExprTokenKind::Caret => ExprTokenClass::Caret,
        ExprTokenKind::LParen => ExprTokenClass::LParen,
        ExprTokenKind::RParen => ExprTokenClass::RParen,
    }
}

fn lex_expression(expr: &str, line: usize) -> Result<Vec<ExprToken>> {
    let chars: Vec<char> = expr.chars().collect();
    let mut index = 0usize;
    let mut tokens = Vec::new();

    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() {
            index += 1;
            continue;
        }

        if is_ident_start(ch) {
            let start = index;
            index += 1;
            while index < chars.len() && is_ident_continue(chars[index]) {
                index += 1;
            }
            tokens.push(ExprToken {
                kind: ExprTokenKind::Ident(chars[start..index].iter().collect()),
            });
            continue;
        }

        if ch.is_ascii_digit() {
            let start = index;
            index = consume_expr_number(&chars, index).ok_or_else(|| {
                CompileError::new(
                    ErrorCode::InvalidFormula,
                    "malformed expression",
                    Some(line),
                )
            })?;
            tokens.push(ExprToken {
                kind: ExprTokenKind::Number(chars[start..index].iter().collect()),
            });
            continue;
        }

        let kind = match ch {
            '+' => ExprTokenKind::Plus,
            '-' => ExprTokenKind::Minus,
            '*' => ExprTokenKind::Star,
            '/' => ExprTokenKind::Slash,
            '%' => ExprTokenKind::Percent,
            '^' => ExprTokenKind::Caret,
            '(' => ExprTokenKind::LParen,
            ')' => ExprTokenKind::RParen,
            _ => {
                return Err(CompileError::new(
                    ErrorCode::InvalidFormula,
                    "malformed expression",
                    Some(line),
                ));
            }
        };
        tokens.push(ExprToken { kind });
        index += 1;
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
