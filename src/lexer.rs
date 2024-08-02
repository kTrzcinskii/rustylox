use std::{iter::Peekable, str::Chars};

#[derive(PartialEq, Debug)]
pub enum TokenType {
    // Single character tokens
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Star,
    Slash,
    // One or two characters tokens
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    // Literals
    Identifier,
    String,
    Number,
    // Keywords
    And,
    Class,
    Else,
    False,
    For,
    Fun,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,
    Eof,
}

pub struct Token {
    pub token_type: TokenType,
    pub start: usize,
    pub length: usize,
    pub line: usize,
}

pub struct Lexer<'a> {
    /// Index of beginning of currently processed lexeme
    start: usize,
    /// Index of currently processed character
    current: usize,
    /// Currently processed line
    line: usize,
    /// Source code characters
    source: Peekable<Chars<'a>>,
    /// Source code length
    length: usize,
}

/// Represents error occured while scanning with provided error message and line on which it occured
#[derive(Debug)]
pub struct LexerError(String, usize);

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Lexer {
            start: 0,
            current: 0,
            line: 1,
            source: source.chars().peekable(),
            length: source.len(),
        }
    }

    pub fn scan_token(&mut self) -> Result<Token, LexerError> {
        self.start = self.current;

        if self.is_at_end() {
            return Ok(self.make_token(TokenType::Eof));
        }

        let c = self.advance();
        match c {
            '(' => return Ok(self.make_token(TokenType::LeftParen)),
            ')' => return Ok(self.make_token(TokenType::RightParen)),
            '{' => return Ok(self.make_token(TokenType::LeftBrace)),
            '}' => return Ok(self.make_token(TokenType::RightBrace)),
            ';' => return Ok(self.make_token(TokenType::Semicolon)),
            ',' => return Ok(self.make_token(TokenType::Comma)),
            '.' => return Ok(self.make_token(TokenType::Dot)),
            '-' => return Ok(self.make_token(TokenType::Minus)),
            '+' => return Ok(self.make_token(TokenType::Plus)),
            '*' => return Ok(self.make_token(TokenType::Star)),
            '/' => return Ok(self.make_token(TokenType::Slash)),
            '!' => {
                return Ok(self.handle_one_or_two_character_token(
                    '=',
                    TokenType::BangEqual,
                    TokenType::Equal,
                ))
            }
            '=' => {
                return Ok(self.handle_one_or_two_character_token(
                    '=',
                    TokenType::EqualEqual,
                    TokenType::Equal,
                ))
            }
            '<' => {
                return Ok(self.handle_one_or_two_character_token(
                    '=',
                    TokenType::LessEqual,
                    TokenType::Less,
                ))
            }
            '>' => {
                return Ok(self.handle_one_or_two_character_token(
                    '=',
                    TokenType::GreaterEqual,
                    TokenType::Greater,
                ))
            }
            _ => {}
        }

        Err(LexerError(String::from("Unexpected character."), self.line))
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.length
    }

    fn advance(&mut self) -> char {
        self.current += 1;
        self.source
            .next()
            .expect("This should never go outside of bounds as we check 'is_at_end'.")
    }

    fn match_current(&mut self, expected: char) -> bool {
        if self.is_at_end() {
            return false;
        }

        // This should never panic as we check 'is_at_end' before this action
        if expected != *self.source.peek().unwrap() {
            return false;
        }

        self.advance();
        true
    }

    fn make_token(&self, token_type: TokenType) -> Token {
        Token {
            token_type,
            start: self.start,
            length: self.current - self.start,
            line: self.line,
        }
    }

    fn handle_one_or_two_character_token(
        &mut self,
        next_expected: char,
        two_chararter_case: TokenType,
        one_character_case: TokenType,
    ) -> Token {
        if self.match_current(next_expected) {
            return self.make_token(two_chararter_case);
        }
        self.make_token(one_character_case)
    }
}
