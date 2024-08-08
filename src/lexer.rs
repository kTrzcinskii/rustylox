use std::{iter::Peekable, str::Chars};

#[derive(PartialEq, Debug, Clone, Copy)]
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

#[derive(Clone, Copy)]
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
    /// Original source code string
    original_source: &'a str,
}

/// Represents error occured while scanning with provided error message and line on which it occured
#[derive(Debug)]
pub struct LexerError<'a> {
    pub message: &'a str,
    pub line: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Lexer {
            start: 0,
            current: 0,
            line: 1,
            source: source.chars().peekable(),
            original_source: source,
        }
    }

    pub fn scan_token(&mut self) -> Result<Token, LexerError> {
        self.skip_whitespaces();

        self.start = self.current;

        if self.is_at_end() {
            return Ok(self.make_token(TokenType::Eof));
        }

        let c = self.advance();

        if c.is_alphabetic() {
            return Ok(self.make_identifier_token(c));
        }

        if c.is_ascii_digit() {
            return self.make_number_token();
        }

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
            '/' => {
                if self.peek() == '/' {
                    self.skip_comment();
                    return self.scan_token();
                }
                return Ok(self.make_token(TokenType::Slash));
            }
            '!' => {
                return Ok(self.handle_one_or_two_character_token(
                    '=',
                    TokenType::BangEqual,
                    TokenType::Bang,
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
            '"' => return self.make_string_token(),
            _ => {}
        }

        Err(self.create_error("Unexpected character."))
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.original_source.len()
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

        if expected != self.peek() {
            return false;
        }

        self.advance();
        true
    }

    fn peek(&mut self) -> char {
        match self.source.peek() {
            Some(c) => *c,
            None => '\0',
        }
    }

    fn create_error(&self, message: &'a str) -> LexerError<'a> {
        LexerError {
            message,
            line: self.line,
        }
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

    fn skip_whitespaces(&mut self) {
        let mut c = self.peek();
        while c.is_whitespace() {
            if c == '\n' {
                self.line += 1;
            }
            self.advance();
            c = self.peek();
        }
    }

    fn skip_comment(&mut self) {
        // comment is until end of line or end of file
        while self.peek() != '\n' && !self.is_at_end() {
            self.advance();
        }
    }

    fn make_string_token(&mut self) -> Result<Token, LexerError> {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }
            self.advance();
        }

        if self.is_at_end() {
            return Err(self.create_error("Unterminated string"));
        }

        // consume closing '"'
        self.advance();
        Ok(self.make_token(TokenType::String))
    }

    fn make_number_token(&mut self) -> Result<Token, LexerError> {
        while self.peek().is_ascii_digit() {
            self.advance();
        }

        // check for fractional part
        if self.match_current('.') {
            if !self.peek().is_ascii_digit() {
                return Err(self.create_error("Expected fractional part after '.'."));
            }
            while self.peek().is_ascii_digit() {
                self.advance();
            }
        }

        Ok(self.make_token(TokenType::Number))
    }

    fn make_identifier_token(&mut self, first_letter: char) -> Token {
        let mut second_letter: Option<char> = Option::None;
        while self.peek().is_alphanumeric() {
            let c = self.advance();
            if second_letter.is_none() {
                second_letter = Option::Some(c);
            }
        }
        self.make_token(self.figure_identifier_type(first_letter, second_letter))
    }

    // This approach with explicitly getting first and second letter is pretty hacky
    // I did only because it's the only place where we need to access chars by index
    // and because it only requires first two letters
    // If one of these were false I would look for something better, but
    // I think in this context it's fine
    fn figure_identifier_type(&self, first_letter: char, second_letter: Option<char>) -> TokenType {
        // We don't have any keyword that is only 1 letter long
        if second_letter.is_none() {
            return TokenType::Identifier;
        }

        match first_letter {
            'a' => self.check_for_keyword(1, "nd", TokenType::And),
            'c' => self.check_for_keyword(1, "lass", TokenType::Class),
            'e' => self.check_for_keyword(1, "lse", TokenType::Else),
            'f' => match second_letter {
                Some('a') => self.check_for_keyword(2, "lse", TokenType::False),
                Some('o') => self.check_for_keyword(2, "r", TokenType::For),
                Some('u') => self.check_for_keyword(2, "n", TokenType::Fun),
                Some(_) => TokenType::Identifier,
                None => TokenType::Identifier,
            },
            'i' => self.check_for_keyword(1, "f", TokenType::If),
            'n' => self.check_for_keyword(1, "il", TokenType::Nil),
            'o' => self.check_for_keyword(1, "r", TokenType::Or),
            'p' => self.check_for_keyword(1, "rint", TokenType::Print),
            'r' => self.check_for_keyword(1, "eturn", TokenType::Return),
            's' => self.check_for_keyword(1, "uper", TokenType::Super),
            't' => match second_letter {
                Some('h') => self.check_for_keyword(2, "is", TokenType::This),
                Some('r') => self.check_for_keyword(2, "ue", TokenType::True),
                Some(_) => TokenType::Identifier,
                None => TokenType::Identifier,
            },
            'v' => self.check_for_keyword(1, "ar", TokenType::Var),
            'w' => self.check_for_keyword(1, "hile", TokenType::While),
            _ => TokenType::Identifier,
        }
    }

    fn check_for_keyword(&self, skip: usize, expected: &str, token_type: TokenType) -> TokenType {
        let start_index = self.start + skip;
        let end_index = start_index + expected.len();

        let actual = &self.original_source[start_index..end_index];
        if actual == expected {
            return token_type;
        }
        TokenType::Identifier
    }
}
