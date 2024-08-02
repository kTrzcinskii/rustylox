use crate::lexer::{Lexer, TokenType};

pub struct Compiler {}

impl Compiler {
    pub fn new() -> Self {
        Compiler {}
    }

    pub fn compile(&self, source: &str) {
        let mut lexer = Lexer::new(source);
        // Just for developing purposes - TODO: remove me
        let line = 0;

        loop {
            let token = lexer.scan_token().unwrap();
            if token.line != line {
                print!("{:04} - ", line);
            } else {
                print!("   | ");
            }
            println!("{:?}", token.token_type);
            if token.token_type == TokenType::Eof {
                break;
            }
        }
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}
