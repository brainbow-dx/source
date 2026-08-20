#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Int(i64),
    Ident(String),
    Str(String),

    KwInt,
    KwIf,
    KwElse,
    KwWhile,
    KwReturn,

    Plus,
    Minus,
    Star,
    Slash,
    Percent,

    Eq,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    LParen,
    RParen,
    LBrace,
    RBrace,
    Semi,
    Comma,

    Eof,
}

pub fn lex(source: &str) -> Vec<Token> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        if c.is_whitespace() {
            i += 1;
            continue;
        }

        // Line comments only — enough for this dialect's own examples.
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }

        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && chars[i].is_ascii_digit() {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            tokens.push(Token::Int(text.parse().unwrap_or(0)));
            continue;
        }

        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            tokens.push(match text.as_str() {
                "int" => Token::KwInt,
                "if" => Token::KwIf,
                "else" => Token::KwElse,
                "while" => Token::KwWhile,
                "return" => Token::KwReturn,
                _ => Token::Ident(text),
            });
            continue;
        }

        if c == '"' {
            let mut text = String::new();
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' && chars.get(i + 1) == Some(&'n') {
                    text.push('\n');
                    i += 2;
                    continue;
                }
                text.push(chars[i]);
                i += 1;
            }
            i += 1; // closing quote
            tokens.push(Token::Str(text));
            continue;
        }

        macro_rules! two_char {
            ($second:expr, $with:expr, $without:expr) => {{
                if chars.get(i + 1) == Some(&$second) {
                    i += 2;
                    tokens.push($with);
                } else {
                    i += 1;
                    tokens.push($without);
                }
            }};
        }

        match c {
            '+' => {
                i += 1;
                tokens.push(Token::Plus);
            }
            '-' => {
                i += 1;
                tokens.push(Token::Minus);
            }
            '*' => {
                i += 1;
                tokens.push(Token::Star);
            }
            '/' => {
                i += 1;
                tokens.push(Token::Slash);
            }
            '%' => {
                i += 1;
                tokens.push(Token::Percent);
            }
            '=' => two_char!('=', Token::EqEq, Token::Eq),
            '!' if chars.get(i + 1) == Some(&'=') => {
                i += 2;
                tokens.push(Token::Ne);
            }
            '<' => two_char!('=', Token::Le, Token::Lt),
            '>' => two_char!('=', Token::Ge, Token::Gt),
            '(' => {
                i += 1;
                tokens.push(Token::LParen);
            }
            ')' => {
                i += 1;
                tokens.push(Token::RParen);
            }
            '{' => {
                i += 1;
                tokens.push(Token::LBrace);
            }
            '}' => {
                i += 1;
                tokens.push(Token::RBrace);
            }
            ';' => {
                i += 1;
                tokens.push(Token::Semi);
            }
            ',' => {
                i += 1;
                tokens.push(Token::Comma);
            }
            _ => {
                // Unrecognized characters are skipped rather than erroring — this is a "quick
                // and dirty" dialect, not a conformant C lexer.
                i += 1;
            }
        }
    }

    tokens.push(Token::Eof);
    tokens
}
