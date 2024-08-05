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

#[derive(Debug)]
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

    fn call_prefix_function(&mut self, token_type: &TokenType) {
        match token_type {
            TokenType::LeftParen => self.handle_grouping(),
            TokenType::RightParen => {}
            TokenType::LeftBrace => {}
            TokenType::RightBrace => {}
            TokenType::Comma => {}
            TokenType::Dot => {}
            TokenType::Minus => self.handle_unary(),
            TokenType::Plus => {}
            TokenType::Semicolon => {}
            TokenType::Star => {}
            TokenType::Slash => {}
            TokenType::Bang => {}
            TokenType::BangEqual => {}
            TokenType::Equal => {}
            TokenType::EqualEqual => {}
            TokenType::Greater => {}
            TokenType::GreaterEqual => {}
            TokenType::Less => {}
            TokenType::LessEqual => {}
            TokenType::Identifier => {}
            TokenType::String => {}
            TokenType::Number => self.handle_number(),
            TokenType::And => {}
            TokenType::Class => {}
            TokenType::Else => {}
            TokenType::False => {}
            TokenType::For => {}
            TokenType::Fun => {}
            TokenType::If => {}
            TokenType::Nil => {}
            TokenType::Or => {}
            TokenType::Print => {}
            TokenType::Return => {}
            TokenType::Super => {}
            TokenType::This => {}
            TokenType::True => {}
            TokenType::Var => {}
            TokenType::While => {}
            TokenType::Eof => {}
        }
    }

    fn call_infix_function(&mut self, token_type: &TokenType) {
        match token_type {
            TokenType::LeftParen => {}
            TokenType::RightParen => {}
            TokenType::LeftBrace => {}
            TokenType::RightBrace => {}
            TokenType::Comma => {}
            TokenType::Dot => {}
            TokenType::Minus => self.handle_binary(),
            TokenType::Plus => self.handle_binary(),
            TokenType::Semicolon => {}
            TokenType::Star => self.handle_binary(),
            TokenType::Slash => self.handle_binary(),
            TokenType::Bang => {}
            TokenType::BangEqual => {}
            TokenType::Equal => {}
            TokenType::EqualEqual => {}
            TokenType::Greater => {}
            TokenType::GreaterEqual => {}
            TokenType::Less => {}
            TokenType::LessEqual => {}
            TokenType::Identifier => {}
            TokenType::String => {}
            TokenType::Number => {}
            TokenType::And => {}
            TokenType::Class => {}
            TokenType::Else => {}
            TokenType::False => {}
            TokenType::For => {}
            TokenType::Fun => {}
            TokenType::If => {}
            TokenType::Nil => {}
            TokenType::Or => {}
            TokenType::Print => {}
            TokenType::Return => {}
            TokenType::Super => {}
            TokenType::This => {}
            TokenType::True => {}
            TokenType::Var => {}
            TokenType::While => {}
            TokenType::Eof => {}
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

    fn handle_unary(&mut self) {
        // We asume operator was already consumed
        let operator_type = &self.parser.previous.unwrap().token_type;

        // Compile operand
        self.parse_precendence(Precedence::Unary);

        match operator_type {
            TokenType::Minus => self.emit_instruction(OperationCode::Negate),
            _ => panic!("unreachable"),
        }
    }

    fn handle_binary(&mut self) {
        // We asume operator was already consumed
        let operator_type = &self.parser.previous.unwrap().token_type;
        let precedence = Precedence::from(operator_type);

        self.parse_precendence(precedence.get_higher());

        match operator_type {
            TokenType::Plus => self.emit_instruction(OperationCode::Add),
            TokenType::Minus => self.emit_instruction(OperationCode::Substract),
            TokenType::Star => self.emit_instruction(OperationCode::Multiply),
            TokenType::Slash => self.emit_instruction(OperationCode::Divide),
            _ => panic!("unreachable"),
        }
    }

    fn parse_precendence(&mut self, precedence: Precedence) {}

    fn compile_expression(&mut self) {
        self.parse_precendence(Precedence::Assignment);
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

#[derive(Clone, Copy, PartialEq)]
enum Precedence {
    None,
    Assignment,
    Or,
    And,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

#[derive(Debug)]
pub enum PrecedenceError {
    OutOfBounds,
}

impl From<Precedence> for u8 {
    fn from(value: Precedence) -> Self {
        match value {
            Precedence::None => 0,
            Precedence::Assignment => 1,
            Precedence::Or => 2,
            Precedence::And => 3,
            Precedence::Equality => 4,
            Precedence::Comparison => 5,
            Precedence::Term => 6,
            Precedence::Factor => 7,
            Precedence::Unary => 8,
            Precedence::Call => 9,
            Precedence::Primary => 10,
        }
    }
}

impl TryFrom<u8> for Precedence {
    type Error = PrecedenceError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Precedence::None),
            1 => Ok(Precedence::Assignment),
            2 => Ok(Precedence::Or),
            3 => Ok(Precedence::And),
            4 => Ok(Precedence::Equality),
            5 => Ok(Precedence::Comparison),
            6 => Ok(Precedence::Term),
            7 => Ok(Precedence::Factor),
            8 => Ok(Precedence::Unary),
            9 => Ok(Precedence::Call),
            10 => Ok(Precedence::Primary),
            _ => Err(PrecedenceError::OutOfBounds),
        }
    }
}

impl From<&TokenType> for Precedence {
    fn from(value: &TokenType) -> Self {
        match value {
            TokenType::LeftParen => Precedence::None,
            TokenType::RightParen => Precedence::None,
            TokenType::LeftBrace => Precedence::None,
            TokenType::RightBrace => Precedence::None,
            TokenType::Comma => Precedence::None,
            TokenType::Dot => Precedence::None,
            TokenType::Minus => Precedence::Term,
            TokenType::Plus => Precedence::Term,
            TokenType::Semicolon => Precedence::Factor,
            TokenType::Star => Precedence::Factor,
            TokenType::Slash => Precedence::None,
            TokenType::Bang => Precedence::None,
            TokenType::BangEqual => Precedence::None,
            TokenType::Equal => Precedence::None,
            TokenType::EqualEqual => Precedence::None,
            TokenType::Greater => Precedence::None,
            TokenType::GreaterEqual => Precedence::None,
            TokenType::Less => Precedence::None,
            TokenType::LessEqual => Precedence::None,
            TokenType::Identifier => Precedence::None,
            TokenType::String => Precedence::None,
            TokenType::Number => Precedence::None,
            TokenType::And => Precedence::None,
            TokenType::Class => Precedence::None,
            TokenType::Else => Precedence::None,
            TokenType::False => Precedence::None,
            TokenType::For => Precedence::None,
            TokenType::Fun => Precedence::None,
            TokenType::If => Precedence::None,
            TokenType::Nil => Precedence::None,
            TokenType::Or => Precedence::None,
            TokenType::Print => Precedence::None,
            TokenType::Return => Precedence::None,
            TokenType::Super => Precedence::None,
            TokenType::This => Precedence::None,
            TokenType::True => Precedence::None,
            TokenType::Var => Precedence::None,
            TokenType::While => Precedence::None,
            TokenType::Eof => Precedence::None,
        }
    }
}

impl Precedence {
    fn get_higher(&self) -> Precedence {
        let current = u8::from(*self);
        let higher = current + 1;
        match Precedence::try_from(higher) {
            Ok(p) => p,
            Err(e) => match e {
                // If we have the highest precedence level we just gonna still return it from here
                PrecedenceError::OutOfBounds => *self,
            },
        }
    }
}
