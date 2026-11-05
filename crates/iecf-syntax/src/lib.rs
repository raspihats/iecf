//! iecf's front end: IEC 61131-3 text to a syntax tree, with diagnostics.
//!
//! No dependencies beyond iecf's own diagnostics, so it builds for wasm
//! and runs in the Workbench as it runs in the build.

pub mod ast;
mod lexer;
mod parser;
mod token;

pub use lexer::lex;
pub use parser::{Parse, parse};
pub use token::{Kw, Token, TokenKind};
