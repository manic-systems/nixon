//! A small, lossless parser for the Nix language.

mod kind;
mod text;

pub use kind::SyntaxKind;
pub use text::{TextRange, TextSize};
