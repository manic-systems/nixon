//! A small, lossless parser for the Nix language.

mod diagnostic;
mod kind;
mod lexer;
mod parser;
mod text;
mod tree;

pub use diagnostic::{Diagnostic, DiagnosticKind, InputError, Severity};
pub use kind::SyntaxKind;
pub use lexer::{Token, Tokenizer, tokenize};
pub use parser::{parse, parse_bytes};
pub use text::{TextRange, TextSize};
pub use tree::{Document, Element, ElementId, Node, TokenNode};
