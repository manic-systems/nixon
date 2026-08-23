use crate::{
    Diagnostic, DiagnosticKind, Document, InputError, SyntaxKind, TextSize, Token, Tokenizer,
    tree::{Event, build_tree},
};

/// Parses one UTF-8 Nix expression.
///
/// # Errors
///
/// Returns [`InputError::TooLarge`] when the source cannot be represented with
/// compact offsets. Syntax errors are retained in the returned document.
pub fn parse(source: &str) -> Result<Document<'_>, InputError> {
    if source.len() > u32::MAX as usize {
        return Err(InputError::TooLarge);
    }
    Ok(Parser::new(source).parse())
}

/// Validates UTF-8 and parses one Nix expression from bytes.
///
/// # Errors
///
/// Returns an error for invalid UTF-8 or sources too large for compact offsets.
pub fn parse_bytes(source: &[u8]) -> Result<Document<'_>, InputError> {
    let source = str::from_utf8(source).map_err(|error| InputError::InvalidUtf8 {
        valid_up_to: error.valid_up_to(),
    })?;
    parse(source)
}

struct Parser<'src> {
    source: &'src str,
    tokenizer: Tokenizer<'src>,
    current: Token,
    trivia: Vec<Token>,
    events: Vec<Event>,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Copy)]
struct Marker(usize);

#[derive(Clone, Copy)]
struct CompletedMarker(usize);

impl<'src> Parser<'src> {
    fn new(source: &'src str) -> Self {
        let mut tokenizer = crate::tokenize(source);
        let mut trivia = Vec::new();
        let current = next_significant(&mut tokenizer, &mut trivia);
        Self {
            source,
            tokenizer,
            current,
            trivia,
            events: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn parse(mut self) -> Document<'src> {
        let root = self.start();
        if !self.at(SyntaxKind::Eof) {
            self.parse_expression();
        } else {
            self.error_expected(None);
        }
        while !self.at(SyntaxKind::Eof) {
            self.error_and_bump(DiagnosticKind::TrailingInput);
        }
        self.flush_trivia();
        self.complete(root, SyntaxKind::Root);
        let elements = build_tree(&mut self.events);
        Document::new(self.source, elements, self.diagnostics)
    }

    fn parse_expression(&mut self) -> CompletedMarker {
        self.parse_function()
    }

    fn parse_function(&mut self) -> CompletedMarker {
        if self.at(SyntaxKind::Identifier) && self.nth_kind(1) == SyntaxKind::Colon {
            let marker = self.start();
            self.bump();
            self.bump();
            self.parse_function();
            return self.complete(marker, SyntaxKind::Lambda);
        }
        if self.at(SyntaxKind::Identifier)
            && self.nth_kind(1) == SyntaxKind::At
            && self.nth_kind(2) == SyntaxKind::LeftBrace
        {
            let marker = self.start();
            self.bump();
            self.bump();
            self.parse_formal_set();
            self.expect(SyntaxKind::Colon);
            self.parse_function();
            return self.complete(marker, SyntaxKind::Lambda);
        }
        if self.at(SyntaxKind::LeftBrace) && self.looks_like_formal_set() {
            let marker = self.start();
            self.parse_formal_set();
            if self.eat(SyntaxKind::At) {
                self.expect(SyntaxKind::Identifier);
            }
            self.expect(SyntaxKind::Colon);
            self.parse_function();
            return self.complete(marker, SyntaxKind::Lambda);
        }
        if self.at(SyntaxKind::AssertKeyword) {
            let marker = self.start();
            self.bump();
            self.parse_expression();
            self.expect(SyntaxKind::Semicolon);
            self.parse_function();
            return self.complete(marker, SyntaxKind::Assert);
        }
        if self.at(SyntaxKind::WithKeyword) {
            let marker = self.start();
            self.bump();
            self.parse_expression();
            self.expect(SyntaxKind::Semicolon);
            self.parse_function();
            return self.complete(marker, SyntaxKind::With);
        }
        if self.at(SyntaxKind::LetKeyword) && self.nth_kind(1) != SyntaxKind::LeftBrace {
            let marker = self.start();
            self.bump();
            self.parse_bindings(SyntaxKind::InKeyword);
            self.expect(SyntaxKind::InKeyword);
            self.parse_function();
            return self.complete(marker, SyntaxKind::LetIn);
        }
        self.parse_if()
    }

    fn parse_if(&mut self) -> CompletedMarker {
        if self.at(SyntaxKind::IfKeyword) {
            let marker = self.start();
            self.bump();
            self.parse_expression();
            self.expect(SyntaxKind::ThenKeyword);
            self.parse_expression();
            self.expect(SyntaxKind::ElseKeyword);
            self.parse_expression();
            return self.complete(marker, SyntaxKind::IfThenElse);
        }
        self.parse_pipe()
    }

    fn parse_pipe(&mut self) -> CompletedMarker {
        let mut left = self.parse_operator(0);
        if self.at(SyntaxKind::PipeInto) {
            while self.at(SyntaxKind::PipeInto) {
                let marker = self.precede(left);
                self.bump();
                self.parse_operator(0);
                left = self.complete(marker, SyntaxKind::BinaryOperation);
            }
        } else if self.at(SyntaxKind::PipeFrom) {
            left = self.parse_pipe_from(left);
        }
        left
    }

    fn parse_pipe_from(&mut self, left: CompletedMarker) -> CompletedMarker {
        let marker = self.precede(left);
        self.bump();
        let right = self.parse_operator(0);
        if self.at(SyntaxKind::PipeFrom) {
            self.parse_pipe_from(right);
        }
        self.complete(marker, SyntaxKind::BinaryOperation)
    }

    fn parse_operator(&mut self, minimum: u8) -> CompletedMarker {
        let mut left = if matches!(self.current.kind(), SyntaxKind::Bang | SyntaxKind::Minus) {
            let marker = self.start();
            let binding = if self.at(SyntaxKind::Bang) { 7 } else { 12 };
            self.bump();
            self.parse_operator(binding);
            self.complete(marker, SyntaxKind::UnaryOperation)
        } else {
            self.parse_application()
        };

        loop {
            if self.at(SyntaxKind::Question) {
                if 11 < minimum {
                    break;
                }
                let marker = self.precede(left);
                self.bump();
                self.parse_attribute_path();
                left = self.complete(marker, SyntaxKind::HasAttribute);
                continue;
            }
            let Some((left_binding, right_binding)) = infix_binding(self.current.kind()) else {
                break;
            };
            if left_binding < minimum {
                break;
            }
            let marker = self.precede(left);
            self.bump();
            self.parse_operator(right_binding);
            left = self.complete(marker, SyntaxKind::BinaryOperation);
        }
        left
    }

    fn parse_application(&mut self) -> CompletedMarker {
        let mut left = self.parse_select();
        while starts_primary(self.current.kind()) {
            let marker = self.precede(left);
            self.parse_select();
            left = self.complete(marker, SyntaxKind::Apply);
        }
        left
    }

    fn parse_select(&mut self) -> CompletedMarker {
        let mut expression = self.parse_primary();
        if self.eat(SyntaxKind::Dot) {
            let marker = self.precede(expression);
            self.parse_attribute_path();
            if self.eat(SyntaxKind::OrKeyword) {
                self.parse_select();
            }
            expression = self.complete(marker, SyntaxKind::Select);
        } else if self.eat(SyntaxKind::OrKeyword) {
            let marker = self.precede(expression);
            expression = self.complete(marker, SyntaxKind::Apply);
        }
        expression
    }

    fn parse_primary(&mut self) -> CompletedMarker {
        match self.current.kind() {
            SyntaxKind::Identifier
            | SyntaxKind::Integer
            | SyntaxKind::Float
            | SyntaxKind::Path
            | SyntaxKind::SearchPath
            | SyntaxKind::Uri => {
                let marker = self.start();
                self.bump();
                self.complete(marker, SyntaxKind::Literal)
            }
            SyntaxKind::Quote | SyntaxKind::IndentedQuote => self.parse_string(),
            SyntaxKind::PathFragment => self.parse_interpolated_path(),
            SyntaxKind::LeftParen => {
                let marker = self.start();
                self.bump();
                self.parse_expression();
                self.expect(SyntaxKind::RightParen);
                self.complete(marker, SyntaxKind::Parenthesized)
            }
            SyntaxKind::LeftBracket => self.parse_list(),
            SyntaxKind::LeftBrace => self.parse_attribute_set(false, false),
            SyntaxKind::RecKeyword => self.parse_attribute_set(true, false),
            SyntaxKind::LetKeyword => self.parse_attribute_set(false, true),
            SyntaxKind::Eof => {
                let marker = self.start();
                self.error_expected(None);
                self.complete(marker, SyntaxKind::ErrorNode)
            }
            _ => {
                let marker = self.start();
                self.error_and_bump(if self.at(SyntaxKind::Error) {
                    DiagnosticKind::InvalidToken
                } else {
                    DiagnosticKind::UnexpectedToken
                });
                self.complete(marker, SyntaxKind::ErrorNode)
            }
        }
    }

    fn parse_string(&mut self) -> CompletedMarker {
        let marker = self.start();
        let delimiter = self.current.kind();
        self.bump();
        while !self.at(delimiter) && !self.at(SyntaxKind::Eof) {
            if self.at(SyntaxKind::StringFragment) {
                self.bump();
            } else if self.at(SyntaxKind::InterpolationStart) {
                self.parse_interpolation();
            } else {
                self.error_and_bump(DiagnosticKind::UnexpectedToken);
            }
        }
        self.expect(delimiter);
        self.complete(marker, SyntaxKind::String)
    }

    fn parse_interpolated_path(&mut self) -> CompletedMarker {
        let marker = self.start();
        while matches!(
            self.current.kind(),
            SyntaxKind::PathFragment | SyntaxKind::InterpolationStart
        ) {
            if self.at(SyntaxKind::PathFragment) {
                self.bump();
            } else {
                self.parse_interpolation();
            }
        }
        self.complete(marker, SyntaxKind::PathExpression)
    }

    fn parse_interpolation(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.bump();
        self.parse_expression();
        self.expect(SyntaxKind::RightBrace);
        self.complete(marker, SyntaxKind::Interpolation)
    }

    fn parse_list(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.bump();
        while !self.at(SyntaxKind::RightBracket) && !self.at(SyntaxKind::Eof) {
            let before = self.current.range().start();
            self.parse_select();
            if self.current.range().start() == before {
                self.error_and_bump(DiagnosticKind::UnexpectedToken);
            }
        }
        self.expect(SyntaxKind::RightBracket);
        self.complete(marker, SyntaxKind::List)
    }

    fn parse_attribute_set(&mut self, recursive: bool, legacy_let: bool) -> CompletedMarker {
        let marker = self.start();
        if recursive || legacy_let {
            self.bump();
        }
        self.expect(SyntaxKind::LeftBrace);
        self.parse_bindings(SyntaxKind::RightBrace);
        self.expect(SyntaxKind::RightBrace);
        self.complete(
            marker,
            if legacy_let {
                SyntaxKind::LegacyLet
            } else {
                SyntaxKind::AttributeSet
            },
        )
    }

    fn parse_bindings(&mut self, end: SyntaxKind) {
        while !self.at(end) && !self.at(SyntaxKind::Eof) {
            let before = self.current.range().start();
            if self.at(SyntaxKind::InheritKeyword) {
                self.parse_inherit();
            } else if starts_attribute(self.current.kind()) {
                let marker = self.start();
                self.parse_attribute_path();
                self.expect(SyntaxKind::Assign);
                self.parse_expression();
                self.expect(SyntaxKind::Semicolon);
                self.complete(marker, SyntaxKind::Binding);
            } else {
                self.error_and_bump(DiagnosticKind::UnexpectedToken);
            }
            if self.current.range().start() == before {
                self.error_and_bump(DiagnosticKind::UnexpectedToken);
            }
        }
    }

    fn parse_inherit(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.bump();
        if self.eat(SyntaxKind::LeftParen) {
            self.parse_expression();
            self.expect(SyntaxKind::RightParen);
        }
        while starts_attribute(self.current.kind()) {
            self.parse_attribute();
        }
        self.expect(SyntaxKind::Semicolon);
        self.complete(marker, SyntaxKind::Inherit)
    }

    fn parse_attribute_path(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.parse_attribute();
        while self.eat(SyntaxKind::Dot) {
            self.parse_attribute();
        }
        self.complete(marker, SyntaxKind::AttributePath)
    }

    fn parse_attribute(&mut self) {
        match self.current.kind() {
            SyntaxKind::Identifier | SyntaxKind::OrKeyword => self.bump(),
            SyntaxKind::Quote | SyntaxKind::IndentedQuote => {
                self.parse_string();
            }
            SyntaxKind::InterpolationStart => {
                self.parse_interpolation();
            }
            _ => self.error_expected(Some(SyntaxKind::Identifier)),
        }
    }

    fn parse_formal_set(&mut self) -> CompletedMarker {
        let marker = self.start();
        self.expect(SyntaxKind::LeftBrace);
        while !self.at(SyntaxKind::RightBrace) && !self.at(SyntaxKind::Eof) {
            if self.eat(SyntaxKind::Ellipsis) {
                self.eat(SyntaxKind::Comma);
                break;
            }
            let formal = self.start();
            self.expect(SyntaxKind::Identifier);
            if self.eat(SyntaxKind::Question) {
                self.parse_expression();
            }
            self.complete(formal, SyntaxKind::Formal);
            if !self.eat(SyntaxKind::Comma) {
                break;
            }
        }
        self.expect(SyntaxKind::RightBrace);
        self.complete(marker, SyntaxKind::FormalSet)
    }

    fn looks_like_formal_set(&self) -> bool {
        let mut depth = 0_u32;
        let mut tokens = core::iter::once(self.current).chain(self.tokenizer.clone());
        for token in tokens.by_ref().filter(|token| !token.kind().is_trivia()) {
            match token.kind() {
                SyntaxKind::LeftBrace => depth += 1,
                SyntaxKind::RightBrace => {
                    depth -= 1;
                    if depth == 0 {
                        return tokens.find(|token| !token.kind().is_trivia()).is_some_and(
                            |token| matches!(token.kind(), SyntaxKind::Colon | SyntaxKind::At),
                        );
                    }
                }
                SyntaxKind::Eof => break,
                _ => {}
            }
        }
        false
    }

    fn nth_kind(&self, index: usize) -> SyntaxKind {
        if index == 0 {
            return self.current.kind();
        }
        self.tokenizer
            .clone()
            .filter(|token| !token.kind().is_trivia())
            .nth(index - 1)
            .map_or(SyntaxKind::Eof, |token| token.kind())
    }

    fn start(&mut self) -> Marker {
        let position = self.events.len();
        self.events.push(Event::Start {
            kind: None,
            forward_parent: None,
        });
        Marker(position)
    }

    fn complete(&mut self, marker: Marker, kind: SyntaxKind) -> CompletedMarker {
        let Event::Start {
            kind: event_kind, ..
        } = &mut self.events[marker.0]
        else {
            unreachable!("marker did not point to a start event")
        };
        *event_kind = Some(kind);
        self.events.push(Event::Finish);
        CompletedMarker(marker.0)
    }

    fn precede(&mut self, completed: CompletedMarker) -> Marker {
        let marker = self.start();
        let Event::Start { forward_parent, .. } = &mut self.events[completed.0] else {
            unreachable!("completed marker did not point to a start event")
        };
        *forward_parent = Some(marker.0 - completed.0);
        marker
    }

    fn at(&self, kind: SyntaxKind) -> bool {
        self.current.kind() == kind
    }

    fn eat(&mut self, kind: SyntaxKind) -> bool {
        if self.at(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: SyntaxKind) {
        if !self.eat(kind) {
            self.error_expected(Some(kind));
        }
    }

    fn bump(&mut self) {
        self.flush_trivia();
        if !self.at(SyntaxKind::Eof) {
            self.events.push(Event::Token(self.current));
            self.trivia.clear();
            self.current = next_significant(&mut self.tokenizer, &mut self.trivia);
        }
    }

    fn flush_trivia(&mut self) {
        self.events.extend(self.trivia.drain(..).map(Event::Token));
    }

    fn error_expected(&mut self, expected: Option<SyntaxKind>) {
        let kind = if self.at(SyntaxKind::Eof) {
            DiagnosticKind::UnexpectedEof
        } else {
            DiagnosticKind::UnexpectedToken
        };
        self.diagnostics.push(Diagnostic::syntax(
            kind,
            self.current.range(),
            expected,
            self.current.kind(),
        ));
    }

    fn error_and_bump(&mut self, kind: DiagnosticKind) {
        self.diagnostics.push(Diagnostic::syntax(
            kind,
            self.current.range(),
            None,
            self.current.kind(),
        ));
        self.bump();
    }
}

fn next_significant(tokenizer: &mut Tokenizer<'_>, trivia: &mut Vec<Token>) -> Token {
    loop {
        let token = tokenizer.next().unwrap_or(Token::eof(TextSize::new(0)));
        if token.kind().is_trivia() {
            trivia.push(token);
        } else {
            return token;
        }
    }
}

fn infix_binding(kind: SyntaxKind) -> Option<(u8, u8)> {
    Some(match kind {
        SyntaxKind::Implication => (1, 1),
        SyntaxKind::Or => (2, 3),
        SyntaxKind::And => (3, 4),
        SyntaxKind::Equal | SyntaxKind::NotEqual => (4, 5),
        SyntaxKind::Less
        | SyntaxKind::Greater
        | SyntaxKind::LessEqual
        | SyntaxKind::GreaterEqual => (5, 6),
        SyntaxKind::Update => (6, 6),
        SyntaxKind::Plus | SyntaxKind::Minus => (8, 9),
        SyntaxKind::Star | SyntaxKind::Slash => (9, 10),
        SyntaxKind::Concatenate => (10, 10),
        _ => return None,
    })
}

fn starts_primary(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Identifier
            | SyntaxKind::Integer
            | SyntaxKind::Float
            | SyntaxKind::Quote
            | SyntaxKind::IndentedQuote
            | SyntaxKind::Path
            | SyntaxKind::PathFragment
            | SyntaxKind::SearchPath
            | SyntaxKind::Uri
            | SyntaxKind::LeftParen
            | SyntaxKind::LeftBracket
            | SyntaxKind::LeftBrace
            | SyntaxKind::RecKeyword
            | SyntaxKind::LetKeyword
    )
}

fn starts_attribute(kind: SyntaxKind) -> bool {
    matches!(
        kind,
        SyntaxKind::Identifier
            | SyntaxKind::OrKeyword
            | SyntaxKind::Quote
            | SyntaxKind::IndentedQuote
            | SyntaxKind::InterpolationStart
    )
}

