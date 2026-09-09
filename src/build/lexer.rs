use std::{
    io::{Error, ErrorKind},
    iter::Peekable,
    path::Path,
    str::Chars,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    EOL,
    EOF,

    Mut,
    Immut,
    Ref,
    Func,
    Private,
    Colon,
    DoubleColon,
    ColonArrow,

    If,
    Else,
    While,
    Break,
    Continue,
    Ret,
    Object,
    Namespace,
    Entry,
    Constructor,

    Identifier(String),
    Integer(i64),
    Float(f64),
    String(String),
    Char(char),
    Boolean(bool),
    Get(Vec<Token>),

    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,

    Equals,
    DoubleEquals,
    BangEquals,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,

    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Exponent,
    Bang,
    DoubleAmpersand,
    DoublePipe,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: Type,
    pub line: usize,
    pub column: usize,
    pub file: String,
}

pub struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
    current_line: usize,
    current_column: usize,
    current_file: String,
}

impl<'a> Lexer<'a> {
    pub fn new(code: &'a str, file_name: &str) -> Self {
        Lexer {
            chars: code.chars().peekable(),
            current_line: 1,
            current_column: 0,
            current_file: file_name.to_string(),
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = vec![];
        loop {
            let result = self.next_token();
            if let Ok(token) = result {
                if token.token_type == Type::EOF {
                    break;
                }
                tokens.push(token);
            } else {
                eprintln!(
                    "ran into an error when tokenizing {}:\n{}",
                    self.current_file,
                    result.err().unwrap().to_string()
                );
                std::process::exit(1);
            }
        }
        tokens
    }

    fn next_token(&mut self) -> Result<Token, Error> {
        self.skip_whitespace();
        self.current_column += 1;
        let c = match self.chars.next() {
            Some(c) => c,
            None => {
                return Ok(Token {
                    token_type: Type::EOF,
                    line: self.current_line,
                    column: self.current_column,
                    file: self.current_file.clone(),
                });
            }
        };
        match c {
            '#' => {
                while let Some(&next_char) = self.chars.peek() {
                    if next_char == '\n' {
                        break;
                    }
                    self.chars.next();
                    self.current_column += 1;
                }
                self.next_token()
            }
            '\n' => {
                self.current_line += 1;
                self.current_column = 0;
                Ok(Token {
                    token_type: Type::EOL,
                    line: self.current_line,
                    column: self.current_column,
                    file: self.current_file.clone(),
                })
            }
            '|' if self.chars.peek() == Some(&'|') => {
                self.chars.next();
                self.current_column += 1;
                Ok(Token {
                    token_type: Type::DoublePipe,
                    line: self.current_line,
                    column: self.current_column,
                    file: self.current_file.clone(),
                })
            }
            '&' if self.chars.peek() == Some(&'&') => {
                self.chars.next();
                self.current_column += 1;
                Ok(Token {
                    token_type: Type::DoubleAmpersand,
                    line: self.current_line,
                    column: self.current_column,
                    file: self.current_file.clone(),
                })
            }
            '=' => {
                if self.chars.peek() == Some(&'=') {
                    self.chars.next();
                    self.current_column += 1;
                    Ok(Token {
                        token_type: Type::DoubleEquals,
                        line: self.current_line,
                        column: self.current_column,
                        file: self.current_file.clone(),
                    })
                } else {
                    Ok(Token {
                        token_type: Type::Equals,
                        line: self.current_line,
                        column: self.current_column,
                        file: self.current_file.clone(),
                    })
                }
            }
            '!' => {
                if self.chars.peek() == Some(&'=') {
                    self.chars.next();
                    self.current_column += 1;
                    Ok(Token {
                        token_type: Type::BangEquals,
                        line: self.current_line,
                        column: self.current_column,
                        file: self.current_file.clone(),
                    })
                } else {
                    Ok(Token {
                        token_type: Type::Bang,
                        line: self.current_line,
                        column: self.current_column,
                        file: self.current_file.clone(),
                    })
                }
            }
            '<' => {
                if self.chars.peek() == Some(&'=') {
                    self.chars.next();
                    self.current_column += 1;
                    Ok(Token {
                        token_type: Type::LessThanOrEqual,
                        line: self.current_line,
                        column: self.current_column,
                        file: self.current_file.clone(),
                    })
                } else {
                    Ok(Token {
                        token_type: Type::LessThan,
                        line: self.current_line,
                        column: self.current_column,
                        file: self.current_file.clone(),
                    })
                }
            }
            '>' => {
                if self.chars.peek() == Some(&'=') {
                    self.chars.next();
                    self.current_column += 1;
                    Ok(Token {
                        token_type: Type::GreaterThanOrEqual,
                        line: self.current_line,
                        column: self.current_column,
                        file: self.current_file.clone(),
                    })
                } else {
                    Ok(Token {
                        token_type: Type::GreaterThan,
                        line: self.current_line,
                        column: self.current_column,
                        file: self.current_file.clone(),
                    })
                }
            }
            ':' => {
                if self.chars.peek() == Some(&':') {
                    self.chars.next();
                    self.current_column += 1;
                    Ok(Token {
                        token_type: Type::DoubleColon,
                        line: self.current_line,
                        column: self.current_column,
                        file: self.current_file.clone(),
                    })
                } else if self.chars.peek() == Some(&'>') {
                    self.chars.next();
                    self.current_column += 1;
                    Ok(Token {
                        token_type: Type::ColonArrow,
                        line: self.current_line,
                        column: self.current_column,
                        file: self.current_file.clone(),
                    })
                } else {
                    Ok(Token {
                        token_type: Type::Colon,
                        line: self.current_line,
                        column: self.current_column,
                        file: self.current_file.clone(),
                    })
                }
            }
            '+' => Ok(Token {
                token_type: Type::Plus,
                line: self.current_line,
                column: self.current_column,
                file: self.current_file.clone(),
            }),
            '-' => Ok(Token {
                token_type: Type::Minus,
                line: self.current_line,
                column: self.current_column,
                file: self.current_file.clone(),
            }),
            '*' => Ok(Token {
                token_type: Type::Star,
                line: self.current_line,
                column: self.current_column,
                file: self.current_file.clone(),
            }),
            '/' => Ok(Token {
                token_type: Type::Slash,
                line: self.current_line,
                column: self.current_column,
                file: self.current_file.clone(),
            }),
            '%' => Ok(Token {
                token_type: Type::Percent,
                line: self.current_line,
                column: self.current_column,
                file: self.current_file.clone(),
            }),
            '^' => Ok(Token {
                token_type: Type::Exponent,
                line: self.current_line,
                column: self.current_column,
                file: self.current_file.clone(),
            }),
            '(' => Ok(Token {
                token_type: Type::LParen,
                line: self.current_line,
                column: self.current_column,
                file: self.current_file.clone(),
            }),
            ')' => Ok(Token {
                token_type: Type::RParen,
                line: self.current_line,
                column: self.current_column,
                file: self.current_file.clone(),
            }),
            '{' => Ok(Token {
                token_type: Type::LBrace,
                line: self.current_line,
                column: self.current_column,
                file: self.current_file.clone(),
            }),
            '}' => Ok(Token {
                token_type: Type::RBrace,
                line: self.current_line,
                column: self.current_column,
                file: self.current_file.clone(),
            }),
            ',' => Ok(Token {
                token_type: Type::Comma,
                line: self.current_line,
                column: self.current_column,
                file: self.current_file.clone(),
            }),
            '\'' => self.tokenize_char(),
            '"' => self.tokenize_string(),
            c if c.is_alphabetic() || c == '_' => self.tokenize_word(c),
            c if c.is_digit(10) => self.tokenize_number(c),
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "unexpected character: {} at {} {}:{}",
                        c, self.current_file, self.current_line, self.current_column
                    ),
                ));
            }
        }
    }

    fn tokenize_char(&mut self) -> Result<Token, Error> {
        let mut char_value = String::new();

        while let Some(&c) = self.chars.peek() {
            if c == '\'' {
                self.chars.next();
                self.current_column += 1;
                break;
            } else {
                char_value.push(c);
                self.chars.next();
                self.current_column += 1;
            }
        }

        if char_value.len() != 1 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "invalid character literal: '{}' at {} {}:{}",
                    char_value, self.current_file, self.current_line, self.current_column
                ),
            ));
        }

        Ok(Token {
            token_type: Type::Char(char_value.chars().next().unwrap()),
            line: self.current_line,
            column: self.current_column - char_value.len() - 2,
            file: self.current_file.clone(),
        })
    }

    fn tokenize_string(&mut self) -> Result<Token, Error> {
        let mut string_value = String::new();

        while let Some(&c) = self.chars.peek() {
            if c == '"' {
                self.chars.next();
                self.current_column += 1;
                break;
            } else {
                string_value.push(c);
                self.chars.next();
                self.current_column += 1;
            }
        }

        Ok(Token {
            token_type: Type::String(string_value.clone()),
            line: self.current_line,
            column: self.current_column - string_value.len() - 2,
            file: self.current_file.clone(),
        })
    }

    fn tokenize_word(&mut self, start: char) -> Result<Token, Error> {
        let mut word = String::new();
        word.push(start);
        while let Some(&c) = self.chars.peek() {
            if c.is_alphanumeric() || c == '_' {
                word.push(c);
                self.chars.next();
                self.current_column += 1;
            } else {
                break;
            }
        }

        if word == "get" {
            return self.tokenize_get();
        }

        let token_type = match word.as_str() {
            "mut" => Type::Mut,
            "immut" => Type::Immut,
            "ref" => Type::Ref,
            "func" => Type::Func,
            "private" => Type::Private,
            "if" => Type::If,
            "else" => Type::Else,
            "while" => Type::While,
            "break" => Type::Break,
            "continue" => Type::Continue,
            "ret" => Type::Ret,
            "object" => Type::Object,
            "namespace" => Type::Namespace,
            "entry" => Type::Entry,
            "constructor" => Type::Constructor,
            "true" => Type::Boolean(true),
            "false" => Type::Boolean(false),
            _ => Type::Identifier(word.clone()),
        };

        Ok(Token {
            token_type,
            line: self.current_line,
            column: self.current_column - word.len() + 1,
            file: self.current_file.clone(),
        })
    }

    fn tokenize_number(&mut self, start: char) -> Result<Token, Error> {
        let mut number = String::new();
        number.push(start);
        let mut is_float = false;

        while let Some(&c) = self.chars.peek() {
            if c.is_digit(10) {
                number.push(c);
                self.chars.next();
                self.current_column += 1;
            } else if c == '.' && !is_float {
                is_float = true;
                number.push(c);
                self.chars.next();
                self.current_column += 1;
            } else {
                break;
            }
        }

        let token_type = if is_float {
            Type::Float(number.parse::<f64>().map_err(|_| {
                Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "invalid float literal: {} at {} {}:{}",
                        number, self.current_file, self.current_line, self.current_column
                    ),
                )
            })?)
        } else {
            Type::Integer(number.parse::<i64>().map_err(|_| {
                Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "invalid integer literal: {} at {} {}:{}",
                        number, self.current_file, self.current_line, self.current_column
                    ),
                )
            })?)
        };

        Ok(Token {
            token_type,
            line: self.current_line,
            column: self.current_column - number.len() + 1,
            file: self.current_file.clone(),
        })
    }

    fn tokenize_get(&mut self) -> Result<Token, Error> {
        self.skip_whitespace();
        let mut path = String::new();

        while self.chars.peek() != Some(&'\n') && self.chars.peek().is_some() {
            path.push(self.chars.next().unwrap());
        }

        let current_file_path = Path::new(&self.current_file);
        path = current_file_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .join(path)
            .to_string_lossy()
            .to_string();
        let file = std::fs::read_to_string(&path).map_err(|_| {
            Error::new(
                ErrorKind::NotFound,
                format!(
                    "file not found: {} at {} {}:{}",
                    path, self.current_file, self.current_line, self.current_column
                ),
            )
        })?;

        let mut lexer = Lexer::new(&file, &path);
        Ok(Token {
            token_type: Type::Get(lexer.tokenize()),
            line: self.current_line,
            column: self.current_column,
            file: self.current_file.clone(),
        })
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() && c != '\n' {
                self.current_column += 1;
                self.chars.next();
            } else {
                break;
            }
        }
    }
}
