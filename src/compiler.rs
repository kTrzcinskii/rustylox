use core::panic;

use crate::{
    chunk::{Chunk, OperationCode},
    lexer::{Lexer, Token, TokenType},
    value::Value,
};

pub struct Compiler<'a> {
    parser: Parser,
    lexer: Lexer<'a>,
    source: &'a str,
    compiling_chunk: Option<Chunk>,
}

pub enum CompilerError {
    EmptyChunk,
    ParserInErrorState,
}

impl<'a> Compiler<'a> {
    pub fn new(source: &'a str) -> Self {
        Compiler {
            parser: Parser::new(),
            lexer: Lexer::new(source),
            source,
            compiling_chunk: Some(Chunk::new()),
        }
    }

    pub fn compile(&mut self) -> Result<Chunk, CompilerError> {
        self.advance();
        self.compile_expression();
        self.consume(TokenType::Eof, "Expect end of expression.");

        self.end_compiler();
        match self.parser.in_error_state {
            true => Ok(self
                .compiling_chunk
                .take()
                .ok_or(CompilerError::EmptyChunk)?),
            false => Err(CompilerError::ParserInErrorState),
        }
    }

    fn advance(&mut self) {
        self.parser.previous = self.parser.current.take();
        loop {
            let next = self.lexer.scan_token();
            match next {
                Ok(token) => {
                    self.parser.current = Some(token);
                    break;
                }
                Err(e) => {
                    if self.parser.in_panic_state {
                        return;
                    }
                    self.parser.in_panic_state = true;
                    eprintln!("[line {}] Error: {}", e.line, e.message);
                    self.parser.in_error_state = true;
                }
            }
        }
    }

    fn consume(&mut self, token_type: TokenType, message: &str) {
        match self.parser.current {
            Some(token) => {
                if token.token_type == token_type {
                    self.advance();
                    return;
                }
                self.handle_error_at_token(&token, message);
            }
            None => panic!("Current token in parser should never be None in consume."),
        }
    }

    fn get_lexeme_from_token(&self, token: &Token) -> &'a str {
        let start_index = token.start;
        let end_index = start_index + token.length;
        if end_index >= self.source.len() {
            panic!("Token shouldn't use index outside of bounds.");
        }
        &self.source[start_index..end_index]
    }

    fn handle_error_at_token(&mut self, token: &Token, message: &str) {
        if self.parser.in_panic_state {
            return;
        }
        self.parser.in_panic_state = true;
        let location = if token.token_type == TokenType::Eof {
            "end"
        } else {
            self.get_lexeme_from_token(token)
        };
        eprint!("[line {}] Error at {}: {}", token.line, location, message);
        self.parser.in_error_state = true;
    }

    fn emit_instruction(&mut self, instruction: OperationCode) {
        let line = match self.parser.previous {
            Some(token) => token.line,
            // This might happen if there are no tokens
            None => 0,
        };
        // We assume that there is no possibility that compiling chunk will ever be unset (after calling the only public function - "compile")
        self.compiling_chunk
            .as_mut()
            .unwrap()
            .add_instruction(instruction, line);
    }

    fn emit_constant(&mut self, constant: Value) {
        let index = self.make_constant(constant);
        if index as u8 > u8::MAX {
            // If we are handling constant then we are sure the previous must exists
            self.handle_error_at_token(
                &self.parser.previous.unwrap(),
                "Too many constants in one chunk",
            );
            return;
        }
        self.emit_instruction(OperationCode::Constant(index));
    }

    fn make_constant(&mut self, constant: Value) -> usize {
        // We assume that there is no possibility that compiling chunk will ever be unset (after calling the only public function - "compile")
        self.compiling_chunk
            .as_mut()
            .unwrap()
            .add_constant(constant)
    }

    fn end_compiler(&mut self) {
        self.emit_instruction(OperationCode::Return);
    }

    fn handle_number(&mut self) {
        // We assume the token with number was already consumed and is stored in "previous"
        let number_str = self.get_lexeme_from_token(&self.parser.previous.unwrap());
        match number_str.parse::<f64>() {
            Ok(value) => self.emit_constant(value),
            Err(_) => self.handle_error_at_token(
                &self.parser.previous.unwrap(),
                "Couldn't parse provided number to double (f64)",
            ),
        }
    }

    fn handle_grouping(&mut self) {
        self.compile_expression();
        self.consume(TokenType::RightParen, "Expect ')' after expression.");
    }

    fn compile_expression(&mut self) {
        todo!()
    }
}

struct Parser {
    current: Option<Token>,
    previous: Option<Token>,
    in_error_state: bool,
    in_panic_state: bool,
}

impl Parser {
    fn new() -> Self {
        Parser {
            current: Option::None,
            previous: Option::None,
            in_error_state: false,
            in_panic_state: false,
        }
    }
}
