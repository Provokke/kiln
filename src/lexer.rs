//! Tokenizer for the kiln JavaScript subset.
//!
//! Produces tokens with byte-offset spans. Line/column information is derived
//! lazily from spans when an error message is rendered.

use std::fmt;

/// A half-open byte range into the source text.
///
/// `PartialEq` deliberately always returns `true` so that AST nodes can be
/// compared structurally while ignoring source positions (used by the
/// parse -> print -> parse round-trip tests).
#[derive(Clone, Copy, Debug, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl PartialEq for Span {
    fn eq(&self, _other: &Span) -> bool {
        true
    }
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        Span {
            start: start as u32,
            end: end as u32,
        }
    }
    pub fn to(self, other: Span) -> Span {
        Span {
            start: self.start,
            end: other.end,
        }
    }
}

/// Compute a 1-based (line, column) for a byte offset.
pub fn line_col(src: &str, offset: u32) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for (i, ch) in src.char_indices() {
        if i >= offset as usize {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// A syntax error with location.
#[derive(Debug, Clone)]
pub struct SyntaxError {
    pub message: String,
    pub span: Span,
}

impl SyntaxError {
    pub fn new(message: impl Into<String>, span: Span) -> SyntaxError {
        SyntaxError {
            message: message.into(),
            span,
        }
    }

    /// Render `SyntaxError: msg (line:col)` followed by the source line and a caret.
    pub fn render(&self, src: &str, file: &str) -> String {
        let (line, col) = line_col(src, self.span.start);
        let text = src.lines().nth(line - 1).unwrap_or("");
        format!(
            "{file}:{line}:{col}: SyntaxError: {}\n  {text}\n  {}^",
            self.message,
            " ".repeat(col.saturating_sub(1))
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    Num(f64),
    Str(String),
    /// Template literal: cooked string parts and the raw source (with offset) of each `${...}` expression.
    Template(Vec<String>, Vec<(String, u32)>),
    Ident(String),
    Keyword(&'static str),
    Punct(&'static str),
    Eof,
}

impl fmt::Display for Tok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tok::Num(n) => write!(f, "number {n}"),
            Tok::Str(s) => write!(f, "string {s:?}"),
            Tok::Template(..) => write!(f, "template literal"),
            Tok::Ident(s) => write!(f, "identifier '{s}'"),
            Tok::Keyword(k) => write!(f, "keyword '{k}'"),
            Tok::Punct(p) => write!(f, "'{p}'"),
            Tok::Eof => write!(f, "end of input"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
    /// True if a line terminator precedes this token (used for ASI and `return`).
    pub nl_before: bool,
}

pub const KEYWORDS: &[&str] = &[
    "let",
    "const",
    "var",
    "function",
    "return",
    "if",
    "else",
    "while",
    "for",
    "of",
    "in",
    "break",
    "continue",
    "true",
    "false",
    "null",
    "undefined",
    "typeof",
    "new",
    "this",
    "class",
    "extends",
    "super",
    "try",
    "catch",
    "finally",
    "throw",
    "do",
    "instanceof",
    "void",
];

// Longest first so that maximal munch works with a simple prefix scan.
const PUNCTS: &[&str] = &[
    ">>>=", "===", "!==", "**=", "...", "<<=", ">>=", ">>>", "&&=", "||=", "??=", "=>", "==", "!=", "<=", ">=", "&&", "||", "??", "?.",
    "++", "--", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "**", "<<", ">>", "{", "}", "(", ")", "[", "]", ";", ",", "<", ">", "+",
    "-", "*", "/", "%", "&", "|", "^", "!", "~", "?", ":", "=", ".",
];

pub struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    base: u32,
}

fn is_id_start(c: char) -> bool {
    c == '_' || c == '$' || c.is_alphabetic()
}
fn is_id_continue(c: char) -> bool {
    c == '_' || c == '$' || c.is_alphanumeric() || c == '\u{200c}' || c == '\u{200d}'
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Lexer<'a> {
        Lexer::with_base(src, 0)
    }

    /// A lexer whose spans are offset by `base` (used for template substitutions).
    pub fn with_base(src: &'a str, base: u32) -> Lexer<'a> {
        Lexer {
            src,
            bytes: src.as_bytes(),
            pos: 0,
            base,
        }
    }

    fn span(&self, start: usize) -> Span {
        Span::new(start + self.base as usize, self.pos + self.base as usize)
    }

    fn err<T>(&self, msg: impl Into<String>, start: usize) -> Result<T, SyntaxError> {
        Err(SyntaxError::new(msg, self.span(start)))
    }

    fn peek_char(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    /// Skip whitespace and comments; returns whether a newline was crossed.
    fn skip_trivia(&mut self) -> Result<bool, SyntaxError> {
        let mut nl = false;
        while let Some(c) = self.peek_char() {
            if c == '\n' {
                nl = true;
                self.pos += 1;
            } else if c.is_whitespace() {
                self.pos += c.len_utf8();
            } else if self.src[self.pos..].starts_with("//") {
                while let Some(c) = self.peek_char() {
                    if c == '\n' {
                        break;
                    }
                    self.pos += c.len_utf8();
                }
            } else if self.src[self.pos..].starts_with("/*") {
                let start = self.pos;
                match self.src[self.pos + 2..].find("*/") {
                    Some(i) => {
                        if self.src[self.pos..self.pos + 2 + i].contains('\n') {
                            nl = true;
                        }
                        self.pos += i + 4;
                    }
                    None => return self.err("unterminated block comment", start),
                }
            } else {
                break;
            }
        }
        Ok(nl)
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, SyntaxError> {
        let mut out = Vec::new();
        loop {
            let nl = self.skip_trivia()?;
            let start = self.pos;
            let Some(c) = self.peek_char() else {
                out.push(Token {
                    tok: Tok::Eof,
                    span: self.span(start),
                    nl_before: true,
                });
                return Ok(out);
            };
            let tok = if c.is_ascii_digit() || (c == '.' && self.bytes.get(self.pos + 1).is_some_and(|b| b.is_ascii_digit())) {
                self.number()?
            } else if c == '"' || c == '\'' {
                self.string(c)?
            } else if c == '`' {
                self.template()?
            } else if is_id_start(c) || c == '\\' {
                if c == '\\' {
                    return self.err("unicode escapes in identifiers are not supported", start);
                }
                while let Some(c) = self.peek_char() {
                    if is_id_continue(c) {
                        self.pos += c.len_utf8();
                    } else {
                        break;
                    }
                }
                let word = &self.src[start..self.pos];
                match KEYWORDS.iter().find(|k| **k == word) {
                    Some(k) => Tok::Keyword(k),
                    None => Tok::Ident(word.to_string()),
                }
            } else {
                match PUNCTS.iter().find(|p| self.src[self.pos..].starts_with(**p)) {
                    Some(p) => {
                        // `?.` followed by a digit is a conditional + number (a?.5:1)
                        if *p == "?." && self.bytes.get(self.pos + 2).is_some_and(|b| b.is_ascii_digit()) {
                            self.pos += 1;
                            Tok::Punct("?")
                        } else {
                            self.pos += p.len();
                            Tok::Punct(p)
                        }
                    }
                    None => return self.err(format!("unexpected character '{c}'"), start),
                }
            };
            out.push(Token {
                tok,
                span: self.span(start),
                nl_before: nl,
            });
        }
    }

    fn number(&mut self) -> Result<Tok, SyntaxError> {
        let start = self.pos;
        let rest = &self.src[self.pos..];
        let radix = if rest.starts_with("0x") || rest.starts_with("0X") {
            Some(16)
        } else if rest.starts_with("0b") || rest.starts_with("0B") {
            Some(2)
        } else if rest.starts_with("0o") || rest.starts_with("0O") {
            Some(8)
        } else {
            None
        };
        if let Some(radix) = radix {
            self.pos += 2;
            let ds = self.pos;
            while self.pos < self.bytes.len() && (self.bytes[self.pos] as char).is_digit(radix) {
                self.pos += 1;
            }
            if ds == self.pos {
                return self.err("missing digits after radix prefix", start);
            }
            let mut v = 0f64;
            for ch in self.src[ds..self.pos].chars() {
                v = v * radix as f64 + ch.to_digit(radix).unwrap() as f64;
            }
            return self.finish_number(v, start);
        }
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'.' {
            self.pos += 1;
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }
        if self.pos < self.bytes.len() && (self.bytes[self.pos] | 0x20) == b'e' {
            let save = self.pos;
            self.pos += 1;
            if self.pos < self.bytes.len() && (self.bytes[self.pos] == b'+' || self.bytes[self.pos] == b'-') {
                self.pos += 1;
            }
            let ds = self.pos;
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
            if ds == self.pos {
                self.pos = save;
                return self.err("missing exponent digits", start);
            }
        }
        let v: f64 = self.src[start..self.pos]
            .parse()
            .map_err(|_| SyntaxError::new("invalid number", self.span(start)))?;
        self.finish_number(v, start)
    }

    fn finish_number(&mut self, v: f64, start: usize) -> Result<Tok, SyntaxError> {
        if let Some(c) = self.peek_char() {
            if is_id_start(c) {
                return self.err("identifier starts immediately after numeric literal", start);
            }
        }
        Ok(Tok::Num(v))
    }

    fn escape(&mut self, out: &mut String, start: usize) -> Result<(), SyntaxError> {
        // positioned after the backslash
        let Some(c) = self.peek_char() else {
            return self.err("unterminated escape", start);
        };
        self.pos += c.len_utf8();
        match c {
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'r' => out.push('\r'),
            'b' => out.push('\u{8}'),
            'f' => out.push('\u{c}'),
            'v' => out.push('\u{b}'),
            '0' => out.push('\0'),
            '\n' => {}
            'x' => {
                let h = self.src.get(self.pos..self.pos + 2).unwrap_or("");
                let v = u32::from_str_radix(h, 16).map_err(|_| SyntaxError::new("bad \\x escape", self.span(start)))?;
                self.pos += 2;
                out.push(char::from_u32(v).unwrap());
            }
            'u' => {
                let v = if self.peek_char() == Some('{') {
                    let end = self.src[self.pos..]
                        .find('}')
                        .ok_or_else(|| SyntaxError::new("bad \\u escape", self.span(start)))?;
                    let h = &self.src[self.pos + 1..self.pos + end];
                    self.pos += end + 1;
                    u32::from_str_radix(h, 16).ok()
                } else {
                    let h = self.src.get(self.pos..self.pos + 4).unwrap_or("");
                    self.pos += 4;
                    u32::from_str_radix(h, 16).ok()
                };
                match v.and_then(char::from_u32) {
                    Some(ch) => out.push(ch),
                    None => return self.err("bad \\u escape", start),
                }
            }
            other => out.push(other),
        }
        Ok(())
    }

    fn string(&mut self, quote: char) -> Result<Tok, SyntaxError> {
        let start = self.pos;
        self.pos += 1;
        let mut s = String::new();
        loop {
            let Some(c) = self.peek_char() else {
                return self.err("unterminated string literal", start);
            };
            self.pos += c.len_utf8();
            if c == quote {
                return Ok(Tok::Str(s));
            } else if c == '\\' {
                self.escape(&mut s, start)?;
            } else if c == '\n' {
                return self.err("unterminated string literal", start);
            } else {
                s.push(c);
            }
        }
    }

    fn template(&mut self) -> Result<Tok, SyntaxError> {
        let start = self.pos;
        self.pos += 1;
        let mut quasis = Vec::new();
        let mut exprs = Vec::new();
        let mut cur = String::new();
        loop {
            let Some(c) = self.peek_char() else {
                return self.err("unterminated template literal", start);
            };
            if c == '`' {
                self.pos += 1;
                quasis.push(cur);
                return Ok(Tok::Template(quasis, exprs));
            } else if c == '\\' {
                self.pos += 1;
                self.escape(&mut cur, start)?;
            } else if self.src[self.pos..].starts_with("${") {
                self.pos += 2;
                let es = self.pos;
                // Find the matching '}' by tokenizing the inner region.
                let mut depth = 0i32;
                let mut sub = Lexer::with_base(&self.src[es..], self.base + es as u32);
                loop {
                    sub.skip_trivia()?;
                    let Some(c) = sub.peek_char() else {
                        return self.err("unterminated template substitution", start);
                    };
                    match c {
                        '{' => {
                            depth += 1;
                            sub.pos += 1
                        }
                        '}' if depth == 0 => break,
                        '}' => {
                            depth -= 1;
                            sub.pos += 1
                        }
                        '"' | '\'' => {
                            sub.string(c)?;
                        }
                        '`' => {
                            sub.template()?;
                        }
                        _ => sub.pos += c.len_utf8(),
                    }
                }
                exprs.push((self.src[es..es + sub.pos].to_string(), self.base + es as u32));
                self.pos = es + sub.pos + 1;
                quasis.push(std::mem::take(&mut cur));
            } else {
                self.pos += c.len_utf8();
                cur.push(c);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(s: &str) -> Vec<Tok> {
        Lexer::new(s).tokenize().unwrap().into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn numbers() {
        assert_eq!(
            toks("1 2.5 .5 1e3 0x1F 0b101 0o17 1.5e-2"),
            vec![
                Tok::Num(1.0),
                Tok::Num(2.5),
                Tok::Num(0.5),
                Tok::Num(1000.0),
                Tok::Num(31.0),
                Tok::Num(5.0),
                Tok::Num(15.0),
                Tok::Num(0.015),
                Tok::Eof
            ]
        );
    }

    #[test]
    fn strings_and_escapes() {
        assert_eq!(
            toks(r#"'a\n' "b\"c" 'A\x42\u{1F600}'"#),
            vec![
                Tok::Str("a\n".into()),
                Tok::Str("b\"c".into()),
                Tok::Str("AB\u{1F600}".into()),
                Tok::Eof
            ]
        );
    }

    #[test]
    fn punct_maximal_munch() {
        assert_eq!(
            toks("a===b!==c=>d>>>=e"),
            vec![
                Tok::Ident("a".into()),
                Tok::Punct("==="),
                Tok::Ident("b".into()),
                Tok::Punct("!=="),
                Tok::Ident("c".into()),
                Tok::Punct("=>"),
                Tok::Ident("d".into()),
                Tok::Punct(">>>="),
                Tok::Ident("e".into()),
                Tok::Eof
            ]
        );
    }

    #[test]
    fn keywords_and_unicode_idents() {
        assert_eq!(
            toks("let été = π"),
            vec![
                Tok::Keyword("let"),
                Tok::Ident("été".into()),
                Tok::Punct("="),
                Tok::Ident("π".into()),
                Tok::Eof
            ]
        );
    }

    #[test]
    fn comments_and_newlines() {
        let t = Lexer::new("a // c\n/* x\n */ b").tokenize().unwrap();
        assert_eq!(t.len(), 3);
        assert!(t[1].nl_before);
        assert_eq!(t[1].span.start, 16);
    }

    #[test]
    fn template() {
        let t = toks("`a${x + `in${y}`}b${ {a:1}.a }`");
        match &t[0] {
            Tok::Template(q, e) => {
                assert_eq!(q, &vec!["a".to_string(), "b".into(), "".into()]);
                assert_eq!(e[0].0, "x + `in${y}`");
                assert_eq!(e[1].0, " {a:1}.a ");
            }
            _ => panic!(),
        }
    }

    #[test]
    fn errors_have_positions() {
        let e = Lexer::new("let x = 'abc\n").tokenize().unwrap_err();
        assert_eq!(e.message, "unterminated string literal");
        assert_eq!(line_col("let x = 'abc\n", e.span.start), (1, 9));
        let e = Lexer::new("\n  #").tokenize().unwrap_err();
        assert!(e
            .render("\n  #", "t.js")
            .starts_with("t.js:2:3: SyntaxError: unexpected character '#'"));
        assert!(Lexer::new("3in").tokenize().is_err());
    }
}
