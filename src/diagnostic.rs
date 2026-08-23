use core::fmt;

use crate::{SyntaxKind, TextRange};

/// An error that prevents Nixon from representing an input source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum InputError {
    /// The source is too large for compact 32-bit offsets.
    TooLarge,
    /// The input is not valid UTF-8.
    InvalidUtf8 {
        /// The first byte that is not valid UTF-8.
        valid_up_to: usize,
    },
}

impl fmt::Display for InputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge => formatter.write_str("Nix source exceeds the 4 GiB input limit"),
            Self::InvalidUtf8 { valid_up_to } => {
                write!(formatter, "Nix source is not UTF-8 at byte {valid_up_to}")
            }
        }
    }
}

impl std::error::Error for InputError {}

/// The severity of a parse diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Severity {
    /// The source is not a valid Nix expression.
    Error,
    /// The source is accepted but discouraged.
    Warning,
}

/// A stable category of parser diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DiagnosticKind {
    /// The parser found a token that does not fit the grammar.
    UnexpectedToken,
    /// The source ended before an expression or delimiter was complete.
    UnexpectedEof,
    /// The lexer found an invalid character or unterminated comment.
    InvalidToken,
    /// Valid input followed the complete root expression.
    TrailingInput,
}

/// A syntax or validation problem found while parsing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub(crate) kind: DiagnosticKind,
    pub(crate) severity: Severity,
    pub(crate) range: TextRange,
    pub(crate) expected: Option<SyntaxKind>,
    pub(crate) found: SyntaxKind,
}

impl Diagnostic {
    pub(crate) const fn syntax(
        kind: DiagnosticKind,
        range: TextRange,
        expected: Option<SyntaxKind>,
        found: SyntaxKind,
    ) -> Self {
        Self {
            kind,
            severity: Severity::Error,
            range,
            expected,
            found,
        }
    }

    /// Returns the diagnostic category.
    #[must_use]
    pub const fn kind(self) -> DiagnosticKind {
        self.kind
    }

    /// Returns the diagnostic severity.
    #[must_use]
    pub const fn severity(self) -> Severity {
        self.severity
    }

    /// Returns the source range associated with the diagnostic.
    #[must_use]
    pub const fn range(self) -> TextRange {
        self.range
    }

    /// Returns the expected token when one specific token was required.
    #[must_use]
    pub const fn expected(self) -> Option<SyntaxKind> {
        self.expected
    }

    /// Returns the token found by the parser.
    #[must_use]
    pub const fn found(self) -> SyntaxKind {
        self.found
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.kind, self.expected) {
            (DiagnosticKind::UnexpectedEof, Some(expected)) => {
                write!(formatter, "expected {expected:?}, found end of input")
            }
            (DiagnosticKind::UnexpectedToken, Some(expected)) => {
                write!(formatter, "expected {expected:?}, found {:?}", self.found)
            }
            (DiagnosticKind::InvalidToken, _) => formatter.write_str("invalid Nix token"),
            (DiagnosticKind::TrailingInput, _) => {
                formatter.write_str("unexpected input after the root expression")
            }
            _ => formatter.write_str("invalid Nix syntax"),
        }
    }
}
