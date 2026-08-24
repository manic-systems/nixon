use core::fmt;

use crate::{
  SyntaxKind,
  TextRange,
};

/// An error that prevents Nixon from representing an input source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum InputError {
  /// The source is too large for compact 32-bit offsets.
  TooLarge,
  /// The syntax tree has too many elements for compact links.
  TooManyElements,
  /// The input is not valid UTF-8.
  InvalidUtf8 {
    /// The first byte that is not valid UTF-8.
    valid_up_to: usize,
  },
}

impl fmt::Display for InputError {
  fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::TooLarge => {
        formatter.write_str("Nix source exceeds the 4 GiB input limit")
      },
      Self::TooManyElements => {
        formatter.write_str("Nix syntax tree exceeds the compact element limit")
      },
      Self::InvalidUtf8 { valid_up_to } => {
        write!(formatter, "Nix source is not UTF-8 at byte {valid_up_to}")
      },
    }
  }
}

impl std::error::Error for InputError {}

/// The severity of a parse diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
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
  /// The expression exceeds Nixon's safe recursive nesting limit.
  NestingLimit,
  /// An experimental syntax feature is disabled.
  ExperimentalFeatureDisabled,
  /// A URI literal is discouraged or disabled.
  UriLiteral,
  /// An integer literal is outside Nix's signed 64-bit range.
  IntegerOverflow,
  /// A floating-point literal is outside the finite range.
  FloatOutOfRange,
  /// A path literal ends with a slash.
  TrailingSlashPath,
  /// A function formal is declared more than once.
  DuplicateFormal,
  /// An attribute is bound more than once.
  DuplicateAttribute,
  /// A binding conflicts with a nested attribute path.
  ConflictingAttribute,
  /// A dynamic attribute occurs in a `let` binding.
  DynamicAttributeInLet,
  /// A dynamic attribute occurs in an `inherit` clause.
  DynamicAttributeInInherit,
  /// An identifier is not defined in the current static scope.
  UndefinedVariable,
}

/// A syntax or validation problem found while parsing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Diagnostic {
  pub(crate) kind:     DiagnosticKind,
  pub(crate) severity: Severity,
  pub(crate) range:    TextRange,
  pub(crate) expected: Option<SyntaxKind>,
  pub(crate) found:    SyntaxKind,
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

  pub(crate) const fn validation(
    kind: DiagnosticKind,
    severity: Severity,
    range: TextRange,
    found: SyntaxKind,
  ) -> Self {
    Self {
      kind,
      severity,
      range,
      expected: None,
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
      },
      (DiagnosticKind::UnexpectedToken, Some(expected)) => {
        write!(formatter, "expected {expected:?}, found {:?}", self.found)
      },
      (DiagnosticKind::InvalidToken, _) => {
        formatter.write_str("invalid Nix token")
      },
      (DiagnosticKind::TrailingInput, _) => {
        formatter.write_str("unexpected input after the root expression")
      },
      (DiagnosticKind::NestingLimit, _) => {
        formatter.write_str("expression nesting limit exceeded")
      },
      (DiagnosticKind::ExperimentalFeatureDisabled, _) => {
        formatter.write_str("experimental pipe operators are disabled")
      },
      (DiagnosticKind::UriLiteral, _) => {
        formatter.write_str("URI literals are discouraged")
      },
      (DiagnosticKind::IntegerOverflow, _) => {
        formatter
          .write_str("integer literal is outside the signed 64-bit range")
      },
      (DiagnosticKind::FloatOutOfRange, _) => {
        formatter
          .write_str("floating-point literal is outside the finite range")
      },
      (DiagnosticKind::TrailingSlashPath, _) => {
        formatter.write_str("path literal has a trailing slash")
      },
      (DiagnosticKind::DuplicateFormal, _) => {
        formatter.write_str("function formal is declared more than once")
      },
      (DiagnosticKind::DuplicateAttribute, _) => {
        formatter.write_str("attribute is already defined")
      },
      (DiagnosticKind::ConflictingAttribute, _) => {
        formatter.write_str("attribute conflicts with a nested binding")
      },
      (DiagnosticKind::DynamicAttributeInLet, _) => {
        formatter
          .write_str("dynamic attributes are not allowed in let bindings")
      },
      (DiagnosticKind::DynamicAttributeInInherit, _) => {
        formatter
          .write_str("dynamic attributes are not allowed in inherit clauses")
      },
      (DiagnosticKind::UndefinedVariable, _) => {
        formatter.write_str("undefined variable")
      },
      _ => formatter.write_str("invalid Nix syntax"),
    }
  }
}
