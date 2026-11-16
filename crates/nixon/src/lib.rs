//! A small, lossless parser for the Nix language.

pub mod ast;
mod diagnostic;
mod kind;
mod lexer;
mod parser;
mod text;
mod tree;
mod validation;

pub use diagnostic::{
  Diagnostic,
  DiagnosticKind,
  InputError,
  Severity,
};
pub use kind::SyntaxKind;
pub use lexer::{
  Token,
  Tokenizer,
  tokenize,
};
pub use parser::{
  ParseOptions,
  UrlLiteralPolicy,
  parse,
  parse_bytes,
  parse_syntax,
  parse_with_options,
};
pub use text::{
  TextRange,
  TextSize,
};
pub use tree::{
  Ancestors,
  Descendants,
  Document,
  Element,
  ElementId,
  Node,
  TokenAtOffset,
  TokenNode,
};
pub use validation::BASE_GLOBALS;

/// Maximum recursive syntax nesting accepted by the parser and validator.
///
/// The budget is consumed by expressions nested inside other expressions:
/// parentheses, list items, attribute values, interpolations, operator
/// operands, and the non-tail parts of compound expressions (an `assert`
/// condition, a `with` scope, `let` bindings, formal defaults, and an `if`
/// condition and `then` branch).
///
/// Tail positions do not consume it: the body of a function, `assert`,
/// `with`, or `let ... in`, and the `else` branch of an `if`. Chains such as
/// `assert a; assert b; ...`, `x: y: ...`, or `else if` ladders are parsed
/// and validated iteratively at the level of their first construct, so they
/// may be arbitrarily long. Flat constructs such as lists, attribute sets,
/// and function application do not consume the budget either unless their
/// expressions are themselves nested.
pub const MAX_NESTING_DEPTH: u16 = 256;
