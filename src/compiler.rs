use core::panic;
use std::{cell::RefCell, rc::Rc};

use crate::{
    chunk::{self, OperationCode},
    lexer::{Lexer, Token, TokenType},
    logger::Logger,
    table::Table,
    value::{FunctionObject, Value},
};

pub const INITIALIZER_NAME: &str = "init";

struct Local {
    name: Token,
    /// 0 - global scope, 1 - first top-level scope, etc
    /// -1 means special state - uninitialized (it helps handle weird edge case)
    depth: i32,
    /// True if closure has upvalue that reference this variable
    is_captured: bool,
}

/// Used for variables that are use inside closure, but are defined outside of it
#[derive(PartialEq)]
struct Upvalue {
    /// Index on the actual vm stack of the variable Upvalue is referencing
    index: u8,
    is_local: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub enum FunctionType {
    Function,    // Normal function
    Script,      // Top level function - whole global scope is put in here
    Method,      // Class method
    Initializer, // Class initializer
}

pub struct CompilingClass {
    has_base_class: bool,
}

pub struct Compiler<'a, 'b> {
    parser: Parser,
    lexer: Lexer<'a>,
    source: &'a str,
    // We store it like this to have all functions, so that we can update upvalue of any of them
    functions: Vec<Rc<RefCell<FunctionObject>>>,
    functions_types: Vec<FunctionType>,
    intern_strings: Option<&'b mut Table>,
    // We store it like this to have all locals in every function in nested function chain
    locals: Vec<Vec<Local>>,
    // Same as before - each function has it's own upvalues
    upvalues: Vec<Vec<Upvalue>>,
    current_scope_depth: i32,
    // Stack of currenlty compiling classes (classes can be nesteed one in another)
    // If empty then we aren't inside any class
    compiling_classes: Vec<CompilingClass>,
}

#[derive(Debug)]
pub enum CompilerError {
    ParserInErrorState,
    EmptyFunction,
    EmptyInternStrings,
}

enum LocalVariableError {
    NotFound,
    UsedInOwnInitializer,
}

enum UpvalueError {
    NotFound,
}

const UNINITIALIZED_DEPTH: i32 = -1;

struct PrefixFunctionsArguments {
    can_assign: bool,
}

#[derive(Clone, Copy)]
struct InfixFunctionsArguments {
    can_assign: bool,
}

impl<'a, 'b> Compiler<'a, 'b> {
    pub fn new(source: &'a str, function_type: FunctionType) -> Self {
        // First local is for VM internal use - it stores function that is currently being executed in runtime
        let locals = vec![Local {
            name: Token {
                token_type: TokenType::Identifier,
                start: 0,
                length: 0,
                line: 0,
            },
            depth: 0,
            is_captured: false,
        }];

        Compiler {
            parser: Parser::new(),
            lexer: Lexer::new(source),
            source,
            functions: vec![FunctionObject::new_rc("GLOBAL_SCRIPT")],
            functions_types: vec![function_type],
            intern_strings: None,
            locals: vec![locals],
            upvalues: vec![vec![]],
            current_scope_depth: 0,
            compiling_classes: vec![],
        }
    }

    pub fn compile(
        &mut self,
        intern_strings: &'b mut Table,
    ) -> Result<Rc<RefCell<FunctionObject>>, CompilerError> {
        self.intern_strings = Some(intern_strings);
        self.advance();

        while !self.match_current(&TokenType::Eof) {
            self.compile_declaration();
        }

        self.end_compiler();

        // At this point there should be only one function on the functions stack
        if !self.parser.in_error_state {
            Logger::disassemble_chunk(
                &self.functions.last().unwrap().borrow().chunk,
                "Compiled code",
            )
            .unwrap();
        }

        match self.parser.in_error_state {
            false => Ok(self.functions.last().unwrap().clone()),
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

    fn call_prefix_function(
        &mut self,
        token_type: &TokenType,
        arguments: PrefixFunctionsArguments,
    ) -> Result<(), CompilerError> {
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
            TokenType::Identifier => self.handle_variable(arguments.can_assign),
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
            TokenType::Super => self.handle_super(),
            TokenType::This => self.handle_this(),
            TokenType::True => self.handle_literal(),
            TokenType::Var => return Err(CompilerError::EmptyFunction),
            TokenType::While => return Err(CompilerError::EmptyFunction),
            TokenType::Eof => return Err(CompilerError::EmptyFunction),
        }
        Ok(())
    }

    fn call_infix_function(
        &mut self,
        token_type: &TokenType,
        arguments: InfixFunctionsArguments,
    ) -> Result<(), CompilerError> {
        match token_type {
            TokenType::LeftParen => self.handle_call(),
            TokenType::RightParen => return Err(CompilerError::EmptyFunction),
            TokenType::LeftBrace => return Err(CompilerError::EmptyFunction),
            TokenType::RightBrace => return Err(CompilerError::EmptyFunction),
            TokenType::Comma => return Err(CompilerError::EmptyFunction),
            TokenType::Dot => self.handle_dot(arguments.can_assign),
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
            TokenType::And => self.handle_and(),
            TokenType::Class => return Err(CompilerError::EmptyFunction),
            TokenType::Else => return Err(CompilerError::EmptyFunction),
            TokenType::False => return Err(CompilerError::EmptyFunction),
            TokenType::For => return Err(CompilerError::EmptyFunction),
            TokenType::Fun => return Err(CompilerError::EmptyFunction),
            TokenType::If => return Err(CompilerError::EmptyFunction),
            TokenType::Nil => return Err(CompilerError::EmptyFunction),
            TokenType::Or => self.handle_or(),
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
        self.functions
            .last_mut()
            .unwrap()
            .borrow_mut()
            .chunk
            .add_instruction(instruction, line);
    }

    fn emit_jump_instruction(&mut self, instruction: OperationCode) -> usize {
        self.emit_instruction(instruction);
        self.functions.last().unwrap().borrow().chunk
            .get_instructions_length()
            // + 1, so we end up on the byte that starts the instruction
            - (chunk::JUMP_INSTRUCTION_ARGUMENT_LENGTH + 1)
    }

    fn patch_jump_instruction(&mut self, instruction: OperationCode, instruction_index: usize) {
        let bytes_to_skip = self
            .functions
            .last()
            .unwrap()
            .borrow()
            .chunk
            .get_instructions_length()
            - (instruction_index + chunk::JUMP_INSTRUCTION_ARGUMENT_LENGTH + 1);

        if bytes_to_skip > u16::MAX as usize {
            self.handle_error_at_token(
                &self.parser.previous.unwrap(),
                "Too much code to jump over.",
            );
            return;
        }

        let patch_jump_result = self
            .functions
            .last_mut()
            .unwrap()
            .borrow_mut()
            .chunk
            .patch_jump_instruction(instruction, instruction_index, bytes_to_skip as u16);

        match patch_jump_result {
            Ok(_) => {}
            Err(_) => {
                self.handle_error_at_token(
                    &self.parser.previous.unwrap(),
                    "Error while parsing jump instruction.",
                );
            }
        }
    }

    fn emit_jump_back_instruction(&mut self, jump_to_index: usize) {
        let bytes_to_skip = self
            .functions.last().unwrap().borrow().chunk
            .get_instructions_length()
            - jump_to_index
            // Because we must also jump over the jump instruction itself
            + OperationCode::get_instruction_bytes_length(&OperationCode::JumpBack(u16::MAX));

        if bytes_to_skip > u16::MAX as usize {
            self.handle_error_at_token(
                &self.parser.previous.unwrap(),
                "COuldn't jump back over so many bytes.",
            );
            return;
        }
        self.emit_instruction(OperationCode::JumpBack(bytes_to_skip as u16));
    }

    fn emit_double_instruction(&mut self, first: OperationCode, second: OperationCode) {
        self.emit_instruction(first);
        self.emit_instruction(second);
    }

    fn emit_return_instruction(&mut self) {
        // Implicitly returning "this" if we are inside initializer, nil otherwise
        match self.functions_types.last().unwrap() {
            // "this" is always first element in local array
            FunctionType::Initializer => self.emit_instruction(OperationCode::GetLocal(0)),
            _ => self.emit_constant(Value::new_nil()),
        }
        self.emit_instruction(OperationCode::Return);
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
        self.emit_instruction(OperationCode::Constant(index as u8));
    }

    fn make_constant(&mut self, constant: Value) -> usize {
        self.functions
            .last_mut()
            .unwrap()
            .borrow_mut()
            .chunk
            .add_constant(constant)
    }

    fn make_identifier_constant(&mut self, token: &Token) -> u8 {
        let name = self.get_lexeme_from_token(token);
        let name_string_object = Value::new_string_object(
            name,
            self.intern_strings
                .as_mut()
                .expect("during compilation intern strings should be set"),
        );
        let index = self.make_constant(name_string_object);
        if index as u8 > u8::MAX {
            // If we are handling constant then we are sure the previous must exists
            self.handle_error_at_token(
                &self.parser.previous.unwrap(),
                "Too many constants in one chunk",
            );
            return 0;
        }
        index as u8
    }

    fn end_compiler(&mut self) {
        self.emit_return_instruction();
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
        let index = self.parse_variable("Expect variable name.");

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

        self.define_variable(index);
    }

    fn handle_variable(&mut self, can_assign: bool) {
        self.handle_named_variable(&self.parser.previous.unwrap(), can_assign);
    }

    fn handle_named_variable(&mut self, name: &Token, can_assign: bool) {
        let (get_operation, set_operation) =
            match self.resolve_local_variable(name, self.locals.last().unwrap()) {
                Ok(index) => (
                    OperationCode::GetLocal(index),
                    OperationCode::SetLocal(index),
                ),
                Err(LocalVariableError::NotFound) => {
                    if self.locals.len() == 1 {
                        let global_index = self.make_identifier_constant(name);
                        (
                            OperationCode::GetGlobal(global_index),
                            OperationCode::SetGlobal(global_index),
                        )
                    } else {
                        match self.resolve_upvalue(name, self.locals.len() - 2) {
                            Ok(upvalue_index) => (
                                OperationCode::GetUpvalue(upvalue_index),
                                OperationCode::SetUpvalue(upvalue_index),
                            ),
                            Err(UpvalueError::NotFound) => {
                                let global_index = self.make_identifier_constant(name);
                                (
                                    OperationCode::GetGlobal(global_index),
                                    OperationCode::SetGlobal(global_index),
                                )
                            }
                        }
                    }
                }
                Err(LocalVariableError::UsedInOwnInitializer) => {
                    self.handle_error_at_token(
                        &self.parser.previous.unwrap(),
                        "Can't read value of local variable in its own initializer.",
                    );
                    return;
                }
            };

        if can_assign && self.match_current(&TokenType::Equal) {
            // Setter
            self.compile_expression();
            self.emit_instruction(set_operation);
        } else {
            // Getter
            self.emit_instruction(get_operation);
        }
    }

    fn handle_block_statement(&mut self) {
        while !self.check_current(&TokenType::RightBrace) && !self.check_current(&TokenType::Eof) {
            self.compile_declaration();
        }

        self.consume(TokenType::RightBrace, "Expect '}' after block.");
    }

    fn handle_if_statement(&mut self) {
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'.");
        self.compile_expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        // Instruction for skipping then branch if condition is false
        let skip_then_branch_instruction_index =
            self.emit_jump_instruction(OperationCode::JumpIfFalse(u16::MAX));

        // Both branches must pop stack from the value - the condition that was used to determine which branch to choose
        // If else isn't used then it implicitily is just POP_STACK instruction

        // Then branch
        self.emit_instruction(OperationCode::PopStack);
        self.compile_statement();

        // Instruction for skipping else branch - part of then branch
        let skip_else_branch_instruction_index =
            self.emit_jump_instruction(OperationCode::Jump(u16::MAX));
        // End of then branch

        // Patching here means that everything that was compiled between here and the point where jump instruction itself
        // was emitted will be skipped by jump instruction
        self.patch_jump_instruction(
            OperationCode::JumpIfFalse(u16::MAX),
            skip_then_branch_instruction_index,
        );

        // Else branch
        self.emit_instruction(OperationCode::PopStack);
        if self.match_current(&TokenType::Else) {
            self.compile_statement();
        }
        // End of else branch

        // Same as before
        self.patch_jump_instruction(
            OperationCode::Jump(u16::MAX),
            skip_else_branch_instruction_index,
        );
    }

    fn handle_and(&mut self) {
        let skip_right_operand_instruction_index =
            self.emit_jump_instruction(OperationCode::JumpIfFalse(u16::MAX));

        // If left operand is truthy then we pop it from the stack and leave right operand on the stack
        self.emit_instruction(OperationCode::PopStack);
        self.parse_precendence(Precedence::And);

        self.patch_jump_instruction(
            OperationCode::JumpIfFalse(u16::MAX),
            skip_right_operand_instruction_index,
        );
    }

    fn handle_or(&mut self) {
        let skip_right_operand_instruction_index =
            self.emit_jump_instruction(OperationCode::JumpIfTrue(u16::MAX));

        // If left operand is falsey then we pop it from the stack and leave right operand on the stack
        self.emit_instruction(OperationCode::PopStack);
        self.parse_precendence(Precedence::Or);

        self.patch_jump_instruction(
            OperationCode::JumpIfTrue(u16::MAX),
            skip_right_operand_instruction_index,
        )
    }

    fn handle_while_statement(&mut self) {
        // While statement body
        let while_statement_start_index = self
            .functions
            .last()
            .unwrap()
            .borrow()
            .chunk
            .get_instructions_length();
        self.consume(TokenType::LeftParen, "Expect '(' after 'while'.");
        self.compile_expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        let skip_while_body_instruction_index =
            self.emit_jump_instruction(OperationCode::JumpIfFalse(u16::MAX));

        self.emit_instruction(OperationCode::PopStack);
        self.compile_statement();
        self.emit_jump_back_instruction(while_statement_start_index);
        // End of while statement body

        self.patch_jump_instruction(
            OperationCode::JumpIfFalse(u16::MAX),
            skip_while_body_instruction_index,
        );
        self.emit_instruction(OperationCode::PopStack);
    }

    fn handle_for_statement(&mut self) {
        self.start_scope();
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'.");
        if self.match_current(&TokenType::Semicolon) {
            // No initializer
        } else if self.match_current(&TokenType::Var) {
            self.handle_var_declaration();
        } else {
            self.handle_expression_statement();
        }

        // For statement body
        // It's mutable because it might be changed to point to the incrementer
        let mut for_statement_start_index = self
            .functions
            .last()
            .unwrap()
            .borrow()
            .chunk
            .get_instructions_length();

        let mut skip_for_body_instruction_index: Option<usize> = None;
        if !self.match_current(&TokenType::Semicolon) {
            self.compile_expression();
            self.consume(TokenType::Semicolon, "Expect ';' after for loop condition.");
            skip_for_body_instruction_index =
                Some(self.emit_jump_instruction(OperationCode::JumpIfFalse(u16::MAX)));
            // Remove condition result from the stack
            self.emit_instruction(OperationCode::PopStack);
        }

        if !self.match_current(&TokenType::RightParen) {
            let jump_to_body = self.emit_jump_instruction(OperationCode::Jump(u16::MAX));
            let increment_start = self
                .functions
                .last()
                .unwrap()
                .borrow()
                .chunk
                .get_instructions_length();

            // Execute incrementer
            self.compile_expression();
            // Remove expression result from the stack - we call incrementer only for side effect
            self.emit_instruction(OperationCode::PopStack);
            self.consume(TokenType::RightParen, "Expect ')' after for clauses.");

            self.emit_jump_back_instruction(for_statement_start_index);
            for_statement_start_index = increment_start;
            self.patch_jump_instruction(OperationCode::Jump(u16::MAX), jump_to_body);
        }

        self.compile_statement();
        self.emit_jump_back_instruction(for_statement_start_index);
        // End of for statement body

        if let Some(index) = skip_for_body_instruction_index {
            self.patch_jump_instruction(OperationCode::JumpIfFalse(u16::MAX), index);
            self.emit_instruction(OperationCode::PopStack);
        }
        self.end_scope();
    }

    fn handle_return_statement(&mut self) {
        if *self.functions_types.last().unwrap() == FunctionType::Script {
            self.handle_error_at_token(
                &self.parser.previous.unwrap(),
                "Cannot return from top level script.",
            );
            return;
        }

        if self.match_current(&TokenType::Semicolon) {
            self.emit_return_instruction();
            return;
        } else {
            // If we are using "return" inside initializer, and it's followed by expression (meaning we are trying to return
            // value from initializer), we report it as error
            if *self.functions_types.last().unwrap() == FunctionType::Initializer {
                self.handle_error_at_token(
                    &self.parser.previous.unwrap(),
                    "Can't return a value from an initializer.",
                );
            }
        }

        self.compile_expression();
        self.consume(TokenType::Semicolon, "Expect ';' after return value.");
        self.emit_instruction(OperationCode::Return);
    }

    fn handle_function_declaration(&mut self) {
        let index = self.parse_variable("Expect function name.");
        if self.current_scope_depth > 0 {
            // We do it automatically because we allow function to use itself in its own initializer (needed for recursion)
            self.mark_last_initialized();
        }
        self.handle_function(FunctionType::Function);
        self.define_variable(index);
    }

    fn handle_function(&mut self, function_type: FunctionType) {
        let current_function =
            FunctionObject::new_rc(self.get_lexeme_from_token(&self.parser.previous.unwrap()));
        let special_token_type = match function_type {
            FunctionType::Method => TokenType::This,
            FunctionType::Initializer => TokenType::This,
            _ => TokenType::Identifier,
        };
        let current_locals = vec![Local {
            name: Token {
                token_type: special_token_type,
                start: 0,
                length: 0,
                line: 0,
            },
            depth: 0,
            is_captured: false,
        }];

        let previous_depth = self.current_scope_depth;

        self.functions.push(current_function);
        self.functions_types.push(function_type);
        self.locals.push(current_locals);
        self.upvalues.push(vec![]);

        self.current_scope_depth = 0;

        self.start_scope();

        self.consume(TokenType::LeftParen, "Expect '(' after function name.");

        // Consume function parameters
        if !self.check_current(&TokenType::RightParen) {
            // First parameter
            self.functions.last_mut().unwrap().borrow_mut().arity += 1;
            let index = self.parse_variable("Expect parameter name");
            self.define_variable(index);
            // Other parameters
            while self.match_current(&TokenType::Comma) {
                self.functions.last_mut().unwrap().borrow_mut().arity += 1;
                if self.functions.last_mut().unwrap().borrow_mut().arity > u8::MAX as usize {
                    self.handle_error_at_token(
                        &self.parser.current.unwrap(),
                        "Can't have more than 255 parameters.",
                    );
                }
                let index = self.parse_variable("Expect parameter name");
                self.define_variable(index);
            }
        }

        self.consume(TokenType::RightParen, "Expect ')' after parameters.");
        self.consume(TokenType::LeftBrace, "Expect '{' before function body.");

        self.handle_block_statement();

        self.end_scope();

        self.current_scope_depth = previous_depth;

        self.emit_return_instruction();

        let finished_function = self.functions.pop().unwrap();

        self.functions_types.pop().unwrap();
        self.locals.pop();

        let function_index = self.make_constant(Value::from(finished_function));
        self.emit_instruction(OperationCode::Closure(function_index as u8));

        // Emitting all closure upvalues
        let upvalues = self.upvalues.pop().unwrap();
        for upvalue in upvalues.iter() {
            match upvalue.is_local {
                true => self.emit_instruction(OperationCode::LocalUpvalue(upvalue.index)),
                false => self.emit_instruction(OperationCode::NonLocalUpvalue(upvalue.index)),
            }
        }
    }

    fn handle_call(&mut self) {
        let arguments_count = self.parse_argument_list();
        self.emit_instruction(OperationCode::Call(arguments_count));
    }

    fn handle_class_declaration(&mut self) {
        self.consume(TokenType::Identifier, "Expect class name.");
        let class_name_token = &self
            .parser
            .previous
            .expect("Shouldn't be empty after consuimg identifier");
        let name_constant = self.make_identifier_constant(
            &self
                .parser
                .previous
                .expect("Shouldn't be empty after consuming identifier"),
        );
        self.declare_variable();
        self.emit_instruction(OperationCode::Class(name_constant));
        // We do it here so that we can use class inside it's own body (for something like factory methods etc)
        self.define_variable(name_constant);

        // We add new class to currently compiling classes
        self.compiling_classes.push(CompilingClass {
            has_base_class: false,
        });

        // Handle inheritance
        if self.match_current(&TokenType::Less) {
            self.consume(TokenType::Identifier, "Expect superclass name.");
            // Load base class onto the stack
            self.handle_variable(false);

            // Handle edge case of self-inheriting
            if self.are_identifiers_equal(class_name_token, &self.parser.previous.unwrap()) {
                self.handle_error_at_token(
                    &self.parser.previous.unwrap(),
                    "Class cannot inherit from itself.",
                );
            }

            // Create new scope to properly used "super"
            self.start_scope();
            self.compiling_classes.last_mut().unwrap().has_base_class = true;
            // Fake token for "super" variable
            self.add_local_variable(Token {
                token_type: TokenType::Super,
                start: 0,
                length: 0,
                line: 0,
            });
            // Define "super"
            self.define_variable(0);

            // Load current class onto the stack
            self.handle_named_variable(class_name_token, false);

            self.emit_instruction(OperationCode::Inherit);
        }

        // We do it to load class name contant right on the top of the stack
        // This way, when we are handling methods we know which class they
        // should be bind to, as the class name is right on the stack and can be read
        self.handle_named_variable(class_name_token, false);

        // Class body
        self.consume(TokenType::LeftBrace, "Expect '{' before class body.");
        while !self.check_current(&TokenType::RightBrace) && !self.check_current(&TokenType::Eof) {
            self.handle_class_method();
        }
        self.consume(TokenType::RightBrace, "Expect '}' after class body.");
        // Remove class name from the stack
        self.emit_instruction(OperationCode::PopStack);

        // Close special "super" scope
        if self.compiling_classes.last().unwrap().has_base_class {
            self.end_scope();
        }

        // Finish compiling class
        self.compiling_classes
            .pop()
            .expect("Should not be empty during class compilation");
    }

    fn handle_class_method(&mut self) {
        self.consume(TokenType::Identifier, "Expect method name.");
        let method_name_constant = self.make_identifier_constant(
            &self
                .parser
                .previous
                .expect("Shouldn't be empty after consuming identifier."),
        );

        let function_type = match self.get_lexeme_from_token(&self.parser.previous.unwrap()) {
            INITIALIZER_NAME => FunctionType::Initializer,
            _ => FunctionType::Method,
        };
        self.handle_function(function_type);

        self.emit_instruction(OperationCode::Method(method_name_constant));
    }

    fn handle_dot(&mut self, can_assign: bool) {
        self.consume(TokenType::Identifier, "Expect property name after '.'.");
        let name_constant = self.make_identifier_constant(
            &self
                .parser
                .previous
                .expect("Shouldn't be empty after consuming identifier"),
        );

        if can_assign && self.match_current(&TokenType::Equal) {
            self.compile_expression();
            self.emit_instruction(OperationCode::SetProperty(name_constant));
        } else if self.match_current(&TokenType::LeftParen) {
            // If we encounter '(' right after property, we know it's gonna emmit later "Call" operation,
            // so we emit special optimize instruction which does both
            let arguments_count = self.parse_argument_list();
            self.emit_instruction(OperationCode::InvokeProperty(
                name_constant,
                arguments_count,
            ));
        } else {
            self.emit_instruction(OperationCode::GetProperty(name_constant));
        }
    }

    fn handle_this(&mut self) {
        // If we aren't compiling any class using this is an error
        if self.compiling_classes.is_empty() {
            self.handle_error_at_token(
                &self.parser.previous.unwrap(),
                "Can't use 'this' outside of class.",
            );
            return;
        }
        // We treat "this" as local variable so we have a lot of features for free
        // But we need to actually store it, and we do it by using
        // locals[0], as we made it a special "empty" local for our internal use
        self.handle_variable(false);
    }

    fn handle_super(&mut self) {
        match self.compiling_classes.last() {
            Some(compiling_class) => {
                if !compiling_class.has_base_class {
                    self.handle_error_at_token(
                        &self.parser.previous.unwrap(),
                        "Can't use 'super' inside of class which doesn't have base clas.",
                    );
                }
            }
            None => {
                self.handle_error_at_token(
                    &self.parser.previous.unwrap(),
                    "Can't use 'super' outside of class.",
                );
            }
        }

        self.consume(TokenType::Dot, "Expect '.' after 'super'.");
        self.consume(TokenType::Identifier, "Expect base class method name.");
        let method_name_index = self.make_identifier_constant(&self.parser.previous.unwrap());

        // Put current instance ("this") and base class ("super") on the stack
        self.handle_named_variable(
            &Token {
                token_type: TokenType::This,
                start: 0,
                length: 0,
                line: 0,
            },
            false,
        );
        let super_token = Token {
            token_type: TokenType::Super,
            start: 0,
            length: 0,
            line: 0,
        };
        // Optimize immediate super method call
        if self.match_current(&TokenType::LeftParen) {
            let arguments_count = self.parse_argument_list();
            self.handle_named_variable(&super_token, false);
            self.emit_instruction(OperationCode::InvokeSuperMethod(
                method_name_index,
                arguments_count,
            ));
        } else {
            self.handle_named_variable(&super_token, false);
            self.emit_instruction(OperationCode::GetSuper(method_name_index));
        }
    }

    /// Returns number of parsed arguments
    fn parse_argument_list(&mut self) -> u8 {
        let mut count: usize = 0;
        // Parse parameters
        if !self.check_current(&TokenType::RightParen) {
            // Parse first
            self.compile_expression();
            count += 1;
            // Other
            while self.match_current(&TokenType::Comma) {
                self.compile_expression();
                if count == u8::MAX as usize {
                    self.handle_error_at_token(
                        &self.parser.previous.unwrap(),
                        "Can't have more than 255 arguments.",
                    );
                }
                count += 1;
            }
        }
        self.consume(TokenType::RightParen, "Expect ')' after arguments.");
        count as u8
    }

    fn parse_precendence(&mut self, precedence: Precedence) {
        self.advance();
        let can_assign = precedence as u8 <= Precedence::Assignment as u8;
        let prefix_args = PrefixFunctionsArguments { can_assign };
        let infix_args = InfixFunctionsArguments { can_assign };
        let prefix_fn =
            self.call_prefix_function(&self.parser.previous.unwrap().token_type, prefix_args);
        match prefix_fn {
            Ok(_) => {
                while precedence as u8
                    <= Precedence::from(&self.parser.current.unwrap().token_type) as u8
                {
                    self.advance();
                    match self
                        .call_infix_function(&self.parser.previous.unwrap().token_type, infix_args)
                    {
                        Ok(_) => {}
                        Err(_) => panic!("Should never fail - we check precedence first"),
                    }
                }
                if can_assign && self.match_current(&TokenType::Equal) {
                    self.handle_error_at_token(
                        &self.parser.previous.unwrap(),
                        "Invalid assignment target.",
                    );
                }
            }
            Err(_) => {
                self.handle_error_at_token(&self.parser.previous.unwrap(), "Expect expression.")
            }
        }
    }

    fn parse_variable(&mut self, message: &str) -> u8 {
        self.consume(TokenType::Identifier, message);

        self.declare_variable();
        // If we are not in global scope we don't need to add the variable to the constant table
        if self.current_scope_depth > 0 {
            return 0;
        }

        self.make_identifier_constant(&self.parser.previous.unwrap())
    }

    fn are_identifiers_equal(&self, lhs: &Token, rhs: &Token) -> bool {
        // We do it so that our fake "this" and "super" work
        if lhs.token_type == rhs.token_type {
            if lhs.token_type == TokenType::This {
                return true;
            }
            if lhs.token_type == TokenType::Super {
                return true;
            }
        }
        if lhs.token_type != TokenType::Identifier || rhs.token_type != TokenType::Identifier {
            return false;
        }
        self.get_lexeme_from_token(lhs) == self.get_lexeme_from_token(rhs)
    }

    fn define_variable(&mut self, var_index: u8) {
        if self.current_scope_depth > 0 {
            self.mark_last_initialized();
            return;
        }
        self.emit_instruction(OperationCode::DefineGlobal(var_index));
    }

    fn declare_variable(&mut self) {
        // We don't declare globals
        if self.current_scope_depth == 0 {
            return;
        }
        let name = self.parser.previous.unwrap();

        // Check if variable is already defined
        let mut is_already_defined = false;
        // We iterate from the back, becasue we only need to check current scope
        for local in self.locals.last().unwrap().iter().rev() {
            // If we are in the outter scope we don't have to check the rest
            if local.depth != -1 && local.depth < self.current_scope_depth {
                break;
            }
            if self.are_identifiers_equal(&local.name, &name) {
                is_already_defined = true;
                break;
            }
        }
        if is_already_defined {
            self.handle_error_at_token(
                &self.parser.previous.unwrap(),
                "Variable with same name already exists in the scope.",
            );
        }

        self.add_local_variable(name);
    }

    fn add_local_variable(&mut self, name: Token) {
        // We use one-byte index in our vm, so we cannot have more than u8::MAX + 1
        if self.locals.last().unwrap().len() == u8::MAX as usize + 1 {
            self.handle_error_at_token(
                &self.parser.previous.unwrap(),
                "Too many local variables in scope.",
            );
        }

        self.locals.last_mut().unwrap().push(Local {
            name,
            depth: UNINITIALIZED_DEPTH,
            is_captured: false,
        });
    }

    fn add_upvalue(&mut self, index: usize, is_local: bool, depth: usize) -> usize {
        let upvalue_count = self.functions[depth].borrow().upvalues_count;

        if index > u8::MAX as usize {
            self.handle_error_at_token(
                &self.parser.previous.unwrap(),
                "Too many upvalues in the function.",
            );
            return 0;
        }

        let upvalue = Upvalue {
            index: index as u8,
            is_local,
        };

        match self
            .upvalues
            .last()
            .unwrap()
            .iter()
            .enumerate()
            .find(|(_, already_existing)| **already_existing == upvalue)
        {
            Some((non_local_index, _)) => non_local_index,
            None => {
                self.upvalues[depth].push(upvalue);
                self.functions[depth].borrow_mut().upvalues_count += 1;
                upvalue_count
            }
        }
    }

    fn resolve_local_variable(
        &self,
        name: &Token,
        locals: &[Local],
    ) -> Result<u8, LocalVariableError> {
        for (index, local) in locals.iter().enumerate().rev() {
            if self.are_identifiers_equal(&local.name, name) {
                if local.depth == UNINITIALIZED_DEPTH {
                    return Err(LocalVariableError::UsedInOwnInitializer);
                }
                return Ok(index as u8);
            }
        }

        Err(LocalVariableError::NotFound)
    }

    // It's recursive function
    // We travel from the most recent locals finding the variable for upvalue
    // We can't just iterate over all the locals, because we want to create this upvalue in each elements of the chain
    // So for example, if we have functions a > b > c (> means is inside of)
    // and we have var x declared in scope a and used in c, we want to create upvalue in b as well
    fn resolve_upvalue(&mut self, name: &Token, depth: usize) -> Result<u8, UpvalueError> {
        let locals = &self.locals[depth];

        // When adding upvalue must use depth + 1, because the depth for locals is one level higher
        if let Ok(index) = self.resolve_local_variable(name, locals) {
            let upvalue_index = self.add_upvalue(index as usize, true, depth + 1);
            // Mark local as captured by this closure
            self.locals[depth][index as usize].is_captured = true;
            return Ok(upvalue_index as u8);
        }

        // We checked all the locals
        if depth == 0 {
            return Err(UpvalueError::NotFound);
        }

        match self.resolve_upvalue(name, depth - 1) {
            Ok(upvalue_index) => {
                Ok(self.add_upvalue(upvalue_index as usize, false, depth + 1) as u8)
            }
            Err(_) => Err(UpvalueError::NotFound),
        }
    }

    fn mark_last_initialized(&mut self) {
        self.locals
            .last_mut()
            .unwrap()
            .last_mut()
            .expect("Last local shouldn't be empty when marking as initialized.")
            .depth = self.current_scope_depth;
    }

    fn start_scope(&mut self) {
        self.current_scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.current_scope_depth -= 1;

        // Remove variable which were in the scope that was just ended from the stack
        while let Some(local) = self.locals.last().unwrap().last() {
            if local.depth <= self.current_scope_depth {
                break;
            }
            let local = self.locals.last_mut().unwrap().pop().unwrap();
            match local.is_captured {
                true => self.emit_instruction(OperationCode::CloseUpvalue),
                false => self.emit_instruction(OperationCode::PopStack),
            };
        }
    }

    fn compile_expression(&mut self) {
        self.parse_precendence(Precedence::Assignment);
    }

    fn compile_declaration(&mut self) {
        if self.match_current(&TokenType::Class) {
            self.handle_class_declaration();
        } else if self.match_current(&TokenType::Fun) {
            self.handle_function_declaration();
        } else if self.match_current(&TokenType::Var) {
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
        } else if self.match_current(&TokenType::For) {
            self.handle_for_statement();
        } else if self.match_current(&TokenType::If) {
            self.handle_if_statement();
        } else if self.match_current(&TokenType::Return) {
            self.handle_return_statement();
        } else if self.match_current(&TokenType::While) {
            self.handle_while_statement();
        } else if self.match_current(&TokenType::LeftBrace) {
            self.start_scope();
            self.handle_block_statement();
            self.end_scope();
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
            TokenType::LeftParen => Precedence::Call,
            TokenType::RightParen => Precedence::None,
            TokenType::LeftBrace => Precedence::None,
            TokenType::RightBrace => Precedence::None,
            TokenType::Comma => Precedence::None,
            TokenType::Dot => Precedence::Call,
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
            TokenType::And => Precedence::And,
            TokenType::Class => Precedence::None,
            TokenType::Else => Precedence::None,
            TokenType::False => Precedence::None,
            TokenType::For => Precedence::None,
            TokenType::Fun => Precedence::None,
            TokenType::If => Precedence::None,
            TokenType::Nil => Precedence::None,
            TokenType::Or => Precedence::Or,
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
