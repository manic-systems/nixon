//! A small, lossless parser for the Nix language.

mod kind;
mod lexer;
mod text;

pub use kind::SyntaxKind;
pub use lexer::{Token, Tokenizer, tokenize};
pub use text::{TextRange, TextSize};
