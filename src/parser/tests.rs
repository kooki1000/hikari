use super::error::ParseError;
use super::*;
use crate::lexer::{Lexer, TokenKind};

fn parse_helper(src: &str) -> Vec<Stmt> {
    Parser::new(Lexer::new(src).tokenize()).parse().unwrap()
}

mod control_errors;
mod core;
mod records_enums;
