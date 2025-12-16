use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum TokenKind {
    Nil,
    True,
    False,

    Dot,
    Ident,
    String,
    Number,
    LParen,
    RParen,
    Comma,
    Semicolon,
    Equal,
    Minus,

    If,
    Then,
    Else,
    ElseIf,
    While,
    Do,
    End,
    Alloc,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub data: String,
    pub col: usize,
    pub line: usize,
}

pub fn lex<'s>(src: &'s str) -> Result<Vec<Token>, Error> {
    let mut tokens = vec![];

    let mut chars = src.chars().peekable();
    let mut col = 0;
    let mut line = 1;

    while let Some(char) = chars.next() {
        let token = match char {
            '.' => Token {
                kind: TokenKind::Dot,
                data: char.to_string(),
                col,
                line,
            },
            '=' => Token {
                kind: TokenKind::Equal,
                data: char.to_string(),
                col,
                line,
            },
            ';' => Token {
                kind: TokenKind::Semicolon,
                data: char.to_string(),
                col,
                line,
            },
            '(' => Token {
                kind: TokenKind::LParen,
                data: char.to_string(),
                col,
                line,
            },
            ')' => Token {
                kind: TokenKind::RParen,
                data: char.to_string(),
                col,
                line,
            },
            ',' => Token {
                kind: TokenKind::Comma,
                data: char.to_string(),
                col,
                line,
            },
            '-' => Token {
                kind: TokenKind::Minus,
                data: char.to_string(),
                col,
                line,
            },
            '\n' => {
                line += 1;
                col = 0;
                continue;
            }
            '\r' => {
                col += 1;
                continue;
            }
            ' ' => {
                // Skip whitespace.
                col += 1;
                continue;
            }
            '♥' => {
                // Skip single line comments.
                loop {
                    if let Some(next_char) = chars.peek() {
                        match next_char {
                            '\n' => break,
                            _ => {
                                chars.next();
                                continue;
                            }
                        }
                    }
                }

                line += 1;
                continue;
            }
            c @ '\"' => {
                let mut str_chars = vec![c];
                loop {
                    if let Some(next_char) = chars.next() {
                        if next_char != '\"' {
                            str_chars.push(next_char);
                            // chars.next();
                            continue;
                        } else {
                            str_chars.push(next_char);
                            // chars.next();
                            break;
                        }
                    } else {
                        break;
                    }
                }

                let string = str_chars
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<String>>()
                    .join("");

                Token {
                    kind: TokenKind::String,
                    data: string,
                    col,
                    line,
                }
            }
            c if c.is_ascii_digit() => {
                let mut id = vec![c];
                loop {
                    if let Some(next_char) = chars.peek() {
                        if next_char.is_ascii_digit() {
                            id.push(*next_char);
                            chars.next();
                            continue;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                let ident = id
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<String>>()
                    .join("");

                Token {
                    kind: TokenKind::Number,
                    data: ident,
                    col,
                    line,
                }
            }
            c if c.is_ascii_alphabetic() => {
                let mut id = vec![c];
                loop {
                    if let Some(next_char) = chars.peek() {
                        if next_char.is_ascii_alphanumeric() || *next_char == '_' {
                            id.push(*next_char);
                            chars.next();
                            continue;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                let ident = id
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<String>>()
                    .join("");

                let kind = match ident.as_str() {
                    "true" => TokenKind::True,
                    "false" => TokenKind::False,
                    "nil" => TokenKind::Nil,
                    "IF" => TokenKind::If,
                    "ELSE" => TokenKind::Else,
                    "ELSEIF" => TokenKind::ElseIf,
                    "THEN" => TokenKind::Then,
                    "WHILE" => TokenKind::While,
                    "DO" => TokenKind::Do,
                    "END" => TokenKind::End,
                    "ALLOC" => TokenKind::Alloc,
                    _ => TokenKind::Ident,
                };

                Token {
                    kind,
                    data: ident,
                    col,
                    line,
                }
            }
            _ => {
                return Err(Error::UnexpectedCharacter(char.to_string()));
            }
        };

        col += 1;
        tokens.push(token);
    }

    Ok(tokens)
}
