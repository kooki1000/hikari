use super::error::TypeError;
use super::*;
use crate::lexer::Lexer;
use crate::parser::HikariType;
use crate::parser::{Parser, Stmt};

fn parse(src: &str) -> Vec<Stmt> {
    Parser::new(Lexer::new(src).tokenize()).parse().unwrap()
}

mod builtins;
mod language;
mod records_enums;
mod returns;
mod scoping;
