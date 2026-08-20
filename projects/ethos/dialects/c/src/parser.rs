use crate::lexer::Token;

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Str(String),
    Var(String),
    Binary(Box<Expr>, BinOp, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Decl(String, Expr),
    Assign(String, Expr),
    ExprStmt(Expr),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    While(Expr, Vec<Stmt>),
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        self.pos += 1;
        token
    }

    fn expect(&mut self, token: &Token) {
        if self.peek() == token {
            self.advance();
        }
        // "Quick and dirty": mismatched tokens are tolerated rather than erroring, to keep the
        // parser small. Malformed input just produces a malformed (but non-panicking) program.
    }

    pub fn parse_program(&mut self) -> Vec<Stmt> {
        let mut statements = Vec::new();
        while self.peek() != &Token::Eof {
            statements.push(self.parse_stmt());
        }
        statements
    }

    fn parse_block(&mut self) -> Vec<Stmt> {
        self.expect(&Token::LBrace);
        let mut statements = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            statements.push(self.parse_stmt());
        }
        self.expect(&Token::RBrace);
        statements
    }

    fn parse_stmt(&mut self) -> Stmt {
        match self.peek().clone() {
            Token::KwInt => {
                self.advance();
                let name = self.expect_ident();
                self.expect(&Token::Eq);
                let value = self.parse_expr();
                self.expect(&Token::Semi);
                Stmt::Decl(name, value)
            }
            Token::KwIf => {
                self.advance();
                self.expect(&Token::LParen);
                let cond = self.parse_expr();
                self.expect(&Token::RParen);
                let then_branch = self.parse_block();
                let else_branch = if self.peek() == &Token::KwElse {
                    self.advance();
                    self.parse_block()
                } else {
                    Vec::new()
                };
                Stmt::If(cond, then_branch, else_branch)
            }
            Token::KwWhile => {
                self.advance();
                self.expect(&Token::LParen);
                let cond = self.parse_expr();
                self.expect(&Token::RParen);
                let body = self.parse_block();
                Stmt::While(cond, body)
            }
            Token::Ident(name) => {
                // Lookahead: `name = expr;` is an assignment, anything else is an expression
                // statement (almost always a call, e.g. `printf(...)`).
                if self.tokens.get(self.pos + 1) == Some(&Token::Eq) {
                    self.advance();
                    self.advance();
                    let value = self.parse_expr();
                    self.expect(&Token::Semi);
                    Stmt::Assign(name, value)
                } else {
                    let expr = self.parse_expr();
                    self.expect(&Token::Semi);
                    Stmt::ExprStmt(expr)
                }
            }
            _ => {
                // Unrecognized statement start: consume one token and produce a no-op so a
                // stray/unsupported construct can't loop the parser forever.
                self.advance();
                Stmt::ExprStmt(Expr::Int(0))
            }
        }
    }

    fn expect_ident(&mut self) -> String {
        match self.advance() {
            Token::Ident(name) => name,
            _ => String::new(),
        }
    }

    fn parse_expr(&mut self) -> Expr {
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Expr {
        let mut left = self.parse_additive();
        loop {
            let op = match self.peek() {
                Token::EqEq => BinOp::Eq,
                Token::Ne => BinOp::Ne,
                Token::Lt => BinOp::Lt,
                Token::Le => BinOp::Le,
                Token::Gt => BinOp::Gt,
                Token::Ge => BinOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive();
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        left
    }

    fn parse_additive(&mut self) -> Expr {
        let mut left = self.parse_multiplicative();
        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative();
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        left
    }

    fn parse_multiplicative(&mut self) -> Expr {
        let mut left = self.parse_primary();
        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_primary();
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        left
    }

    fn parse_primary(&mut self) -> Expr {
        match self.advance() {
            Token::Int(value) => Expr::Int(value),
            Token::Str(value) => Expr::Str(value),
            Token::LParen => {
                let inner = self.parse_expr();
                self.expect(&Token::RParen);
                inner
            }
            Token::Ident(name) => {
                if self.peek() == &Token::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
                        args.push(self.parse_expr());
                        if self.peek() == &Token::Comma {
                            self.advance();
                        }
                    }
                    self.expect(&Token::RParen);
                    Expr::Call(name, args)
                } else {
                    Expr::Var(name)
                }
            }
            _ => Expr::Int(0),
        }
    }
}
