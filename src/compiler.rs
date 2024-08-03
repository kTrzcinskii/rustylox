use crate::lexer::{Lexer, TokenType};

pub fn compile(source: &str) {
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
