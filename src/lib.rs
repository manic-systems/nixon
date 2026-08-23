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
pub use parser::{ParseOptions, UrlLiteralPolicy, parse, parse_bytes, parse_with_options};
pub use text::{TextRange, TextSize};
pub use tree::{Document, Element, ElementId, Node, TokenNode};
