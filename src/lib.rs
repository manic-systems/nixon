//! A small, lossless parser for the Nix language.

pub mod ast;
mod diagnostic;
mod kind;
mod lexer;
mod parser;
mod text;
mod tree;
mod validation;

pub use diagnostic::{Diagnostic, DiagnosticKind, InputError, Severity};
pub use kind::SyntaxKind;
pub use lexer::{Token, Tokenizer, tokenize};
pub use parser::{
    ParseOptions, UrlLiteralPolicy, parse, parse_bytes, parse_syntax, parse_with_options,
};
pub use text::{TextRange, TextSize};
pub use tree::{Document, Element, ElementId, Node, TokenNode};

/// Maximum recursive syntax nesting accepted by the parser and validator.
///
/// Flat constructs such as lists, attribute sets, and function application do
/// not consume this budget unless their expressions are themselves nested.
pub const MAX_NESTING_DEPTH: u16 = 256;
