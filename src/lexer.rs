use crate::{SyntaxKind, TextRange, TextSize};

/// A token produced by [`tokenize`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Token {
    kind: SyntaxKind,
    range: TextRange,
}

impl Token {
    /// Returns the token's lexical kind.
    #[must_use]
    pub const fn kind(self) -> SyntaxKind {
        self.kind
    }

    /// Returns the token's byte range.
    #[must_use]
    pub const fn range(self) -> TextRange {
        self.range
    }

    /// Returns the token's original source text.
    #[must_use]
    pub fn text(self, source: &str) -> &str {
        &source[self.range.start().as_usize()..self.range.end().as_usize()]
    }
}

/// Creates a streaming tokenizer over `source`.
#[must_use]
pub fn tokenize(source: &str) -> Tokenizer<'_> {
    Tokenizer::new(source)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Default,
    String,
    IndentedString,
    Path,
}

/// A streaming tokenizer for Nix source code.
#[derive(Clone, Debug)]
pub struct Tokenizer<'src> {
    source: &'src str,
    position: usize,
    modes: Vec<Mode>,
    finished: bool,
}

impl<'src> Tokenizer<'src> {
    fn new(source: &'src str) -> Self {
        Self {
            source,
            position: 0,
            modes: vec![Mode::Default],
            finished: false,
        }
    }

    fn bytes(&self) -> &[u8] {
        self.source.as_bytes()
    }

    fn starts_with(&self, needle: &[u8]) -> bool {
        self.bytes()[self.position..].starts_with(needle)
    }

    fn make_token(&self, kind: SyntaxKind, start: usize) -> Token {
        Token {
            kind,
            range: TextRange::new(
                TextSize::new(start as u32),
                TextSize::new(self.position as u32),
            ),
        }
    }

    fn bump_char(&mut self) {
        let len = self.source[self.position..]
            .chars()
            .next()
            .map_or(0, char::len_utf8);
        self.position += len;
    }

    fn lex_default(&mut self) -> Token {
        let start = self.position;
        let byte = self.bytes()[start];

        if byte.is_ascii_whitespace() {
            self.position += 1;
            while self
                .bytes()
                .get(self.position)
                .is_some_and(u8::is_ascii_whitespace)
            {
                self.position += 1;
            }
            return self.make_token(SyntaxKind::Whitespace, start);
        }

        if byte == b'#' {
            self.position += 1;
            while !matches!(self.bytes().get(self.position), None | Some(b'\n' | b'\r')) {
                self.position += 1;
            }
            return self.make_token(SyntaxKind::LineComment, start);
        }

        if self.starts_with(b"/*") {
            let is_doc = self
                .bytes()
                .get(start + 3)
                .is_some_and(|byte| *byte != b'/' && *byte != b'*')
                && self.starts_with(b"/**");
            self.position += 2;
            while self.position < self.source.len() && !self.starts_with(b"*/") {
                self.bump_char();
            }
            if self.starts_with(b"*/") {
                self.position += 2;
                return self.make_token(
                    if is_doc {
                        SyntaxKind::DocComment
                    } else {
                        SyntaxKind::BlockComment
                    },
                    start,
                );
            }
            return self.make_token(SyntaxKind::Error, start);
        }

        if let Some(end) = scan_uri(self.bytes(), start) {
            self.position = end;
            return self.make_token(SyntaxKind::Uri, start);
        }

        if let Some(end) = scan_search_path(self.bytes(), start) {
            self.position = end;
            return self.make_token(SyntaxKind::SearchPath, start);
        }

        if let Some((end, interpolated)) = scan_path(self.bytes(), start) {
            self.position = end;
            if interpolated {
                self.modes.push(Mode::Path);
                return self.make_token(SyntaxKind::PathFragment, start);
            }
            return self.make_token(SyntaxKind::Path, start);
        }

        if byte.is_ascii_digit()
            || (byte == b'.' && self.bytes().get(start + 1).is_some_and(u8::is_ascii_digit))
        {
            let kind = self.lex_number();
            return self.make_token(kind, start);
        }

        if is_identifier_start(byte) {
            self.position += 1;
            while self
                .bytes()
                .get(self.position)
                .is_some_and(|byte| is_identifier_continue(*byte))
            {
                self.position += 1;
            }
            let kind = keyword(&self.bytes()[start..self.position]);
            return self.make_token(kind, start);
        }

        for (text, kind) in MULTI_CHAR_TOKENS {
            if self.starts_with(text) {
                self.position += text.len();
                if *kind == SyntaxKind::InterpolationStart {
                    self.modes.push(Mode::Default);
                }
                return self.make_token(*kind, start);
            }
        }

        let kind = match byte {
            b'(' => SyntaxKind::LeftParen,
            b')' => SyntaxKind::RightParen,
            b'{' => {
                self.modes.push(Mode::Default);
                SyntaxKind::LeftBrace
            }
            b'}' => {
                if self.modes.len() > 1 {
                    self.modes.pop();
                }
                SyntaxKind::RightBrace
            }
            b'[' => SyntaxKind::LeftBracket,
            b']' => SyntaxKind::RightBracket,
            b'"' => {
                self.modes.push(Mode::String);
                SyntaxKind::Quote
            }
            b',' => SyntaxKind::Comma,
            b';' => SyntaxKind::Semicolon,
            b':' => SyntaxKind::Colon,
            b'@' => SyntaxKind::At,
            b'.' => SyntaxKind::Dot,
            b'=' => SyntaxKind::Assign,
            b'<' => SyntaxKind::Less,
            b'>' => SyntaxKind::Greater,
            b'+' => SyntaxKind::Plus,
            b'-' => SyntaxKind::Minus,
            b'*' => SyntaxKind::Star,
            b'/' => SyntaxKind::Slash,
            b'!' => SyntaxKind::Bang,
            b'?' => SyntaxKind::Question,
            b'\0' => SyntaxKind::Error,
            b'\'' if self.starts_with(b"''") => {
                self.position += 1;
                self.modes.push(Mode::IndentedString);
                SyntaxKind::IndentedQuote
            }
            _ => SyntaxKind::Error,
        };
        self.bump_char();
        self.make_token(kind, start)
    }

    fn lex_number(&mut self) -> SyntaxKind {
        let start = self.position;
        let starts_with_dot = self.bytes()[start] == b'.';

        if starts_with_dot {
            self.position += 1;
            self.consume_digits();
            self.consume_exponent();
            return SyntaxKind::Float;
        }

        self.consume_digits();
        let integer_end = self.position;
        let first = self.bytes()[start];
        let may_have_fraction = first != b'0' || integer_end == start + 1;
        let dot = self.bytes().get(self.position) == Some(&b'.');
        let has_fraction_digit = self
            .bytes()
            .get(self.position + 1)
            .is_some_and(u8::is_ascii_digit);

        if dot && may_have_fraction && (first != b'0' || has_fraction_digit) {
            self.position += 1;
            self.consume_digits();
            self.consume_exponent();
            SyntaxKind::Float
        } else {
            SyntaxKind::Integer
        }
    }

    fn consume_digits(&mut self) {
        while self
            .bytes()
            .get(self.position)
            .is_some_and(u8::is_ascii_digit)
        {
            self.position += 1;
        }
    }

    fn consume_exponent(&mut self) {
        if !matches!(self.bytes().get(self.position), Some(b'e' | b'E')) {
            return;
        }
        let exponent = self.position;
        self.position += 1;
        if matches!(self.bytes().get(self.position), Some(b'+' | b'-')) {
            self.position += 1;
        }
        let digits = self.position;
        self.consume_digits();
        if digits == self.position {
            self.position = exponent;
        }
    }

    fn lex_string(&mut self) -> Token {
        let start = self.position;
        if self.starts_with(b"${") {
            self.position += 2;
            self.modes.push(Mode::Default);
            return self.make_token(SyntaxKind::InterpolationStart, start);
        }
        if self.starts_with(b"\"") {
            self.position += 1;
            self.modes.pop();
            return self.make_token(SyntaxKind::Quote, start);
        }

        while self.position < self.source.len() {
            if self.starts_with(b"${") || self.starts_with(b"\"") {
                break;
            }
            if self.starts_with(b"\\") {
                self.position += 1;
                if self.position < self.source.len() {
                    self.bump_char();
                }
            } else {
                self.bump_char();
            }
        }
        self.make_token(SyntaxKind::StringFragment, start)
    }

    fn lex_indented_string(&mut self) -> Token {
        let start = self.position;
        if self.starts_with(b"${") {
            self.position += 2;
            self.modes.push(Mode::Default);
            return self.make_token(SyntaxKind::InterpolationStart, start);
        }
        if self.starts_with(b"''")
            && !self.starts_with(b"'''")
            && !self.starts_with(b"''$")
            && !self.starts_with(b"''\\")
        {
            self.position += 2;
            self.modes.pop();
            return self.make_token(SyntaxKind::IndentedQuote, start);
        }

        while self.position < self.source.len() {
            if self.starts_with(b"${") {
                break;
            }
            if self.starts_with(b"''") {
                if self.starts_with(b"'''") || self.starts_with(b"''$") {
                    self.position += 3;
                    continue;
                }
                if self.starts_with(b"''\\") {
                    self.position += 3;
                    if self.position < self.source.len() {
                        self.bump_char();
                    }
                    continue;
                }
                break;
            }
            self.bump_char();
        }
        self.make_token(SyntaxKind::StringFragment, start)
    }

    fn lex_path(&mut self) -> Option<Token> {
        let start = self.position;
        if self.starts_with(b"${") {
            self.position += 2;
            self.modes.push(Mode::Default);
            return Some(self.make_token(SyntaxKind::InterpolationStart, start));
        }

        while self.position < self.source.len() {
            if self.starts_with(b"${") {
                break;
            }
            let byte = self.bytes()[self.position];
            if is_path_char(byte) || byte == b'/' {
                self.position += 1;
            } else {
                break;
            }
        }
        if start == self.position {
            self.modes.pop();
            None
        } else {
            Some(self.make_token(SyntaxKind::PathFragment, start))
        }
    }
}

impl Iterator for Tokenizer<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        loop {
            if self.position == self.source.len() {
                self.finished = true;
                return Some(self.make_token(SyntaxKind::Eof, self.position));
            }
            let mode = self.modes.last().copied().unwrap_or(Mode::Default);
            let token = match mode {
                Mode::Default => Some(self.lex_default()),
                Mode::String => Some(self.lex_string()),
                Mode::IndentedString => Some(self.lex_indented_string()),
                Mode::Path => self.lex_path(),
            };
            if token.is_some() {
                return token;
            }
        }
    }
}

const MULTI_CHAR_TOKENS: &[(&[u8], SyntaxKind)] = &[
    (b"...", SyntaxKind::Ellipsis),
    (b"${", SyntaxKind::InterpolationStart),
    (b"==", SyntaxKind::Equal),
    (b"!=", SyntaxKind::NotEqual),
    (b"<=", SyntaxKind::LessEqual),
    (b">=", SyntaxKind::GreaterEqual),
    (b"&&", SyntaxKind::And),
    (b"||", SyntaxKind::Or),
    (b"->", SyntaxKind::Implication),
    (b"//", SyntaxKind::Update),
    (b"++", SyntaxKind::Concatenate),
    (b"<|", SyntaxKind::PipeFrom),
    (b"|>", SyntaxKind::PipeInto),
];

fn keyword(text: &[u8]) -> SyntaxKind {
    match text {
        b"if" => SyntaxKind::IfKeyword,
        b"then" => SyntaxKind::ThenKeyword,
        b"else" => SyntaxKind::ElseKeyword,
        b"assert" => SyntaxKind::AssertKeyword,
        b"with" => SyntaxKind::WithKeyword,
        b"let" => SyntaxKind::LetKeyword,
        b"in" => SyntaxKind::InKeyword,
        b"rec" => SyntaxKind::RecKeyword,
        b"inherit" => SyntaxKind::InheritKeyword,
        b"or" => SyntaxKind::OrKeyword,
        _ => SyntaxKind::Identifier,
    }
}

fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_identifier_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'\'' | b'-')
}

fn is_path_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'+')
}

fn scan_path(bytes: &[u8], start: usize) -> Option<(usize, bool)> {
    if bytes.get(start) == Some(&b'~') && bytes.get(start + 1) != Some(&b'/') {
        return None;
    }
    if !bytes
        .get(start)
        .is_some_and(|byte| is_path_char(*byte) || *byte == b'/' || *byte == b'~')
    {
        return None;
    }

    let mut position = start;
    let mut saw_slash = false;
    let mut has_path_segment = false;
    while let Some(&byte) = bytes.get(position) {
        if is_path_char(byte) || (byte == b'~' && position == start) {
            has_path_segment |= saw_slash;
            position += 1;
        } else if byte == b'/' {
            saw_slash = true;
            position += 1;
        } else {
            break;
        }
    }

    let interpolated = bytes[position..].starts_with(b"${");
    if saw_slash && (has_path_segment || interpolated) {
        Some((position, interpolated))
    } else {
        None
    }
}

fn scan_search_path(bytes: &[u8], start: usize) -> Option<usize> {
    if bytes.get(start) != Some(&b'<') {
        return None;
    }
    let mut position = start + 1;
    let mut has_segment = false;
    while let Some(&byte) = bytes.get(position) {
        if is_path_char(byte) || byte == b'/' {
            has_segment = true;
            position += 1;
        } else {
            break;
        }
    }
    (has_segment && bytes.get(position) == Some(&b'>')).then_some(position + 1)
}

fn scan_uri(bytes: &[u8], start: usize) -> Option<usize> {
    if !bytes.get(start).is_some_and(u8::is_ascii_alphabetic) {
        return None;
    }
    let mut position = start + 1;
    while bytes
        .get(position)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
    {
        position += 1;
    }
    if bytes.get(position) != Some(&b':') {
        return None;
    }
    position += 1;
    let body = position;
    while bytes.get(position).is_some_and(|byte| {
        byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'%' | b'/'
                    | b'?'
                    | b':'
                    | b'@'
                    | b'&'
                    | b'='
                    | b'+'
                    | b'$'
                    | b','
                    | b'-'
                    | b'_'
                    | b'.'
                    | b'!'
                    | b'~'
                    | b'*'
                    | b'\''
            )
    }) {
        position += 1;
    }
    (position > body).then_some(position)
}

