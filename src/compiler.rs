use core::panic;

use crate::{
    chunk::{Chunk, OperationCode},
    lexer::{Lexer, Token, TokenType},
    logger::Logger,
    table::Table,
    value::Value,
};

pub struct Compiler<'a> {
    parser: Parser,
    lexer: Lexer<'a>,
    source: &'a str,
    compiling_chunk: Option<Chunk>,
    intern_strings: Option<Table>,
}

#[derive(Debug)]
pub enum CompilerError {
    EmptyChunk,
    ParserInErrorState,
    EmptyFunction,
    EmptyInternStrings,
}

pub struct CompilerResult {
    pub chunk: Chunk,
    pub intern_strings: Table,
}

impl<'a> Compiler<'a> {
    pub fn new(source: &'a str) -> Self {
        Compiler {
            parser: Parser::new(),
            lexer: Lexer::new(source),
            source,
            compiling_chunk: Some(Chunk::new()),
            intern_strings: Some(Table::new()),
        }
    }

    pub fn compile(&mut self) -> Result<CompilerResult, CompilerError> {
        self.advance();

        while !self.match_current(&TokenType::Eof) {
            self.compile_declaration();
        }

        self.end_compiler();

        if !self.parser.in_error_state {
            Logger::disassemble_chunk(self.compiling_chunk.as_ref().unwrap(), "Compiled code")
                .unwrap();
        }

        match self.parser.in_error_state {
            false => Ok(CompilerResult {
                chunk: self
                    .compiling_chunk
                    .take()
                    .ok_or(CompilerError::EmptyChunk)?,
                intern_strings: self
                    .intern_strings
                    .take()
                    .ok_or(CompilerError::EmptyInternStrings)?,
            }),
            true => Err(CompilerError::ParserInErrorState),
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

    fn check_current(&self, token_type: &TokenType) -> bool {
        self.parser
            .current
            .expect("during check_current parser should have current set")
            .token_type
            == *token_type
    }

    fn match_current(&mut self, token_type: &TokenType) -> bool {
        match self.check_current(token_type) {
            true => {
                self.advance();
                true
            }
            false => false,
        }
    }

    fn synchronize(&mut self) {
        self.parser.in_panic_state = false;

        while self
            .parser
            .current
            .expect("Current shouldn't be empty in synchronize")
            .token_type
            != TokenType::Eof
        {
            match self
                .parser
                .previous
                .expect("Previous shouldn't be empty in synchronize")
                .token_type
            {
                TokenType::Class => return,
                TokenType::Fun => return,
                TokenType::Var => return,
                TokenType::For => return,
                TokenType::If => return,
                TokenType::While => return,
                TokenType::Print => return,
                TokenType::Return => return,
                _ => {}
            }
            self.advance();
        }
    }

    fn call_prefix_function(&mut self, token_type: &TokenType) -> Result<(), CompilerError> {
        match token_type {
            TokenType::LeftParen => self.handle_grouping(),
            TokenType::RightParen => return Err(CompilerError::EmptyFunction),
            TokenType::LeftBrace => return Err(CompilerError::EmptyFunction),
            TokenType::RightBrace => return Err(CompilerError::EmptyFunction),
            TokenType::Comma => return Err(CompilerError::EmptyFunction),
            TokenType::Dot => return Err(CompilerError::EmptyFunction),
            TokenType::Minus => self.handle_unary(),
            TokenType::Plus => return Err(CompilerError::EmptyFunction),
            TokenType::Semicolon => return Err(CompilerError::EmptyFunction),
            TokenType::Star => return Err(CompilerError::EmptyFunction),
            TokenType::Slash => return Err(CompilerError::EmptyFunction),
            TokenType::Bang => self.handle_unary(),
            TokenType::BangEqual => return Err(CompilerError::EmptyFunction),
            TokenType::Equal => return Err(CompilerError::EmptyFunction),
            TokenType::EqualEqual => return Err(CompilerError::EmptyFunction),
            TokenType::Greater => return Err(CompilerError::EmptyFunction),
            TokenType::GreaterEqual => return Err(CompilerError::EmptyFunction),
            TokenType::Less => return Err(CompilerError::EmptyFunction),
            TokenType::LessEqual => return Err(CompilerError::EmptyFunction),
            TokenType::Identifier => return Err(CompilerError::EmptyFunction),
            TokenType::String => self.handle_string(),
            TokenType::Number => self.handle_number(),
            TokenType::And => return Err(CompilerError::EmptyFunction),
            TokenType::Class => return Err(CompilerError::EmptyFunction),
            TokenType::Else => return Err(CompilerError::EmptyFunction),
            TokenType::False => self.handle_literal(),
            TokenType::For => return Err(CompilerError::EmptyFunction),
            TokenType::Fun => return Err(CompilerError::EmptyFunction),
            TokenType::If => return Err(CompilerError::EmptyFunction),
            TokenType::Nil => self.handle_literal(),
            TokenType::Or => return Err(CompilerError::EmptyFunction),
            TokenType::Print => return Err(CompilerError::EmptyFunction),
            TokenType::Return => return Err(CompilerError::EmptyFunction),
            TokenType::Super => return Err(CompilerError::EmptyFunction),
            TokenType::This => return Err(CompilerError::EmptyFunction),
            TokenType::True => self.handle_literal(),
            TokenType::Var => return Err(CompilerError::EmptyFunction),
            TokenType::While => return Err(CompilerError::EmptyFunction),
            TokenType::Eof => return Err(CompilerError::EmptyFunction),
        }
        Ok(())
    }

    fn call_infix_function(&mut self, token_type: &TokenType) -> Result<(), CompilerError> {
        match token_type {
            TokenType::LeftParen => return Err(CompilerError::EmptyFunction),
            TokenType::RightParen => return Err(CompilerError::EmptyFunction),
            TokenType::LeftBrace => return Err(CompilerError::EmptyFunction),
            TokenType::RightBrace => return Err(CompilerError::EmptyFunction),
            TokenType::Comma => return Err(CompilerError::EmptyFunction),
            TokenType::Dot => return Err(CompilerError::EmptyFunction),
            TokenType::Minus => self.handle_binary(),
            TokenType::Plus => self.handle_binary(),
            TokenType::Semicolon => return Err(CompilerError::EmptyFunction),
            TokenType::Star => self.handle_binary(),
            TokenType::Slash => self.handle_binary(),
            TokenType::Bang => return Err(CompilerError::EmptyFunction),
            TokenType::BangEqual => self.handle_binary(),
            TokenType::Equal => return Err(CompilerError::EmptyFunction),
            TokenType::EqualEqual => self.handle_binary(),
            TokenType::Greater => self.handle_binary(),
            TokenType::GreaterEqual => self.handle_binary(),
            TokenType::Less => self.handle_binary(),
            TokenType::LessEqual => self.handle_binary(),
            TokenType::Identifier => return Err(CompilerError::EmptyFunction),
            TokenType::String => return Err(CompilerError::EmptyFunction),
            TokenType::Number => return Err(CompilerError::EmptyFunction),
            TokenType::And => return Err(CompilerError::EmptyFunction),
            TokenType::Class => return Err(CompilerError::EmptyFunction),
            TokenType::Else => return Err(CompilerError::EmptyFunction),
            TokenType::False => return Err(CompilerError::EmptyFunction),
            TokenType::For => return Err(CompilerError::EmptyFunction),
            TokenType::Fun => return Err(CompilerError::EmptyFunction),
            TokenType::If => return Err(CompilerError::EmptyFunction),
            TokenType::Nil => return Err(CompilerError::EmptyFunction),
            TokenType::Or => return Err(CompilerError::EmptyFunction),
            TokenType::Print => return Err(CompilerError::EmptyFunction),
            TokenType::Return => return Err(CompilerError::EmptyFunction),
            TokenType::Super => return Err(CompilerError::EmptyFunction),
            TokenType::This => return Err(CompilerError::EmptyFunction),
            TokenType::True => return Err(CompilerError::EmptyFunction),
            TokenType::Var => return Err(CompilerError::EmptyFunction),
            TokenType::While => return Err(CompilerError::EmptyFunction),
            TokenType::Eof => return Err(CompilerError::EmptyFunction),
        }
        Ok(())
    }

    fn get_lexeme_from_token(&self, token: &Token) -> &'a str {
        let start_index = token.start;
        let end_index = start_index + token.length;
        if end_index > self.source.len() {
            panic!("Token shouldn't use index outside of bounds.");
        }
        &self.source[start_index..end_index]
    }

    fn get_string_content_from_token(&self, token: &Token) -> &'a str {
        let start_index = token.start + 1;
        let end_index = start_index + token.length - 2;
        if end_index > self.source.len() || end_index < start_index {
            panic!("String token should correctly represent string with leading and ending '\"' signs. ")
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
        eprintln!("[line {}] Error at '{}': {}", token.line, location, message);
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

    fn emit_double_instruction(&mut self, first: OperationCode, second: OperationCode) {
        self.emit_instruction(first);
        self.emit_instruction(second);
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

    fn make_identifier_constant(&mut self, token: &Token) -> usize {
        let name = self.get_lexeme_from_token(token);
        let name_string_object = Value::new_string_object(
            name,
            self.intern_strings
                .as_mut()
                .expect("during compilation intern strings should be set"),
        );
        self.make_constant(name_string_object)
    }

    fn end_compiler(&mut self) {
        self.emit_instruction(OperationCode::Return);
    }

    fn handle_number(&mut self) {
        // We assume the token with number was already consumed and is stored in "previous"
        let number_str = self.get_lexeme_from_token(&self.parser.previous.unwrap());
        match number_str.parse::<f64>() {
            Ok(value) => self.emit_constant(Value::new_number(value)),
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
            TokenType::Bang => self.emit_instruction(OperationCode::Not),
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
            TokenType::BangEqual => {
                self.emit_double_instruction(OperationCode::Equal, OperationCode::Not)
            }
            TokenType::EqualEqual => self.emit_instruction(OperationCode::Equal),
            TokenType::Greater => self.emit_instruction(OperationCode::Greater),
            TokenType::GreaterEqual => {
                self.emit_double_instruction(OperationCode::Less, OperationCode::Not)
            }
            TokenType::Less => self.emit_instruction(OperationCode::Less),
            TokenType::LessEqual => {
                self.emit_double_instruction(OperationCode::Greater, OperationCode::Not)
            }
            _ => panic!("unreachable"),
        }
    }

    fn handle_literal(&mut self) {
        match self.parser.previous.unwrap().token_type {
            TokenType::Nil => self.emit_instruction(OperationCode::Nil),
            TokenType::True => self.emit_instruction(OperationCode::True),
            TokenType::False => self.emit_instruction(OperationCode::False),
            _ => panic!("Unreachable"),
        }
    }

    fn handle_string(&mut self) {
        let content = self.get_string_content_from_token(&self.parser.previous.unwrap());
        let new_string = Value::new_string_object(
            content,
            self.intern_strings
                .as_mut()
                .expect("Intern strings should never be emtpy"),
        );
        self.emit_constant(new_string);
    }

    fn handle_print_statement(&mut self) {
        self.compile_expression();
        self.consume(TokenType::Semicolon, "Expect ';' after value.");
        self.emit_instruction(OperationCode::Print);
    }

    fn handle_expression_statement(&mut self) {
        self.compile_expression();
        self.consume(TokenType::Semicolon, "Expect ';' after expression.");
        self.emit_instruction(OperationCode::PopStack);
    }

    fn handle_var_declaration(&mut self) {
        let global_index = self.parse_variable("Expect variable name.");

        if self.match_current(&TokenType::Equal) {
            self.compile_expression();
        } else {
            // If variable isn't initalized with any value we implicitly set it to nil
            self.emit_instruction(OperationCode::Nil);
        }

        self.consume(
            TokenType::Semicolon,
            "Expect ';' after varialbe declaration.",
        );

        self.define_variable(global_index);
    }

    fn parse_precendence(&mut self, precedence: Precedence) {
        self.advance();
        let prefix_fn = self.call_prefix_function(&self.parser.previous.unwrap().token_type);
        match prefix_fn {
            Ok(_) => {
                while precedence as u8
                    <= Precedence::from(&self.parser.current.unwrap().token_type) as u8
                {
                    self.advance();
                    match self.call_infix_function(&self.parser.previous.unwrap().token_type) {
                        Ok(_) => {}
                        Err(_) => panic!("Should never fail - we check precedence first"),
                    }
                }
            }
            Err(_) => {
                self.handle_error_at_token(&self.parser.previous.unwrap(), "Expect expression.")
            }
        }
    }

    fn parse_variable(&mut self, message: &str) -> usize {
        self.consume(TokenType::Identifier, message);
        self.make_identifier_constant(&self.parser.previous.unwrap())
    }

    fn define_variable(&mut self, var_index: usize) {
        self.emit_instruction(OperationCode::DefineGlobal(var_index));
    }

    fn compile_expression(&mut self) {
        self.parse_precendence(Precedence::Assignment);
    }

    fn compile_declaration(&mut self) {
        if self.match_current(&TokenType::Var) {
            self.handle_var_declaration();
        } else {
            self.compile_statement();
        }

        if self.parser.in_panic_state {
            self.synchronize();
        }
    }

    fn compile_statement(&mut self) {
        if self.match_current(&TokenType::Print) {
            self.handle_print_statement();
        } else {
            self.handle_expression_statement();
        }
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
            TokenType::Semicolon => Precedence::None,
            TokenType::Star => Precedence::Factor,
            TokenType::Slash => Precedence::Factor,
            TokenType::Bang => Precedence::None,
            TokenType::BangEqual => Precedence::Equality,
            TokenType::Equal => Precedence::None,
            TokenType::EqualEqual => Precedence::Equality,
            TokenType::Greater => Precedence::Equality,
            TokenType::GreaterEqual => Precedence::Equality,
            TokenType::Less => Precedence::Equality,
            TokenType::LessEqual => Precedence::Equality,
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
