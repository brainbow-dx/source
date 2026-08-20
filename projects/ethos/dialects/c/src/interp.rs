use std::collections::HashMap;

use crate::parser::BinOp;
use crate::parser::Expr;
use crate::parser::Stmt;

#[derive(Debug, Clone)]
enum Value {
    Int(i64),
    Str(String),
}

impl Value {
    fn as_int(&self) -> i64 {
        match self {
            Value::Int(n) => *n,
            Value::Str(_) => 0,
        }
    }
}

/// A tree-walking interpreter for a small, deliberately reduced C-like subset: `int` variable
/// declarations, arithmetic/comparisons, `if`/`else`, `while`, and a builtin `printf` that only
/// understands `%d` substitution — not a conformant C implementation, just enough to prove out
/// "parse a source string, run it, capture its output" as a dialect boundary. See the module
/// doc on `ethos_c` for how this relates to Ethos's actual LLVM-module-compilation goal.
pub struct Interpreter {
    vars: HashMap<String, Value>,
    output: Vec<String>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter { vars: HashMap::new(), output: Vec::new() }
    }

    pub fn run(&mut self, program: &[Stmt]) -> &[String] {
        self.output.clear();
        self.exec_block(program);
        &self.output
    }

    fn exec_block(&mut self, statements: &[Stmt]) {
        for stmt in statements {
            self.exec_stmt(stmt);
        }
    }

    fn exec_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Decl(name, expr) => {
                let value = self.eval(expr);
                self.vars.insert(name.clone(), value);
            }
            Stmt::Assign(name, expr) => {
                let value = self.eval(expr);
                self.vars.insert(name.clone(), value);
            }
            Stmt::ExprStmt(expr) => {
                self.eval(expr);
            }
            Stmt::If(cond, then_branch, else_branch) => {
                if self.eval(cond).as_int() != 0 {
                    self.exec_block(then_branch);
                } else {
                    self.exec_block(else_branch);
                }
            }
            Stmt::While(cond, body) => {
                let mut guard = 0;
                while self.eval(cond).as_int() != 0 {
                    self.exec_block(body);
                    guard += 1;
                    if guard > 1_000_000 {
                        // A runaway loop shouldn't hang the interpreter forever.
                        break;
                    }
                }
            }
        }
    }

    fn eval(&mut self, expr: &Expr) -> Value {
        match expr {
            Expr::Int(n) => Value::Int(*n),
            Expr::Str(s) => Value::Str(s.clone()),
            Expr::Var(name) => self.vars.get(name).cloned().unwrap_or(Value::Int(0)),
            Expr::Binary(left, op, right) => {
                let left = self.eval(left).as_int();
                let right = self.eval(right).as_int();
                Value::Int(match op {
                    BinOp::Add => left + right,
                    BinOp::Sub => left - right,
                    BinOp::Mul => left * right,
                    BinOp::Div => {
                        if right == 0 {
                            0
                        } else {
                            left / right
                        }
                    }
                    BinOp::Mod => {
                        if right == 0 {
                            0
                        } else {
                            left % right
                        }
                    }
                    BinOp::Eq => (left == right) as i64,
                    BinOp::Ne => (left != right) as i64,
                    BinOp::Lt => (left < right) as i64,
                    BinOp::Le => (left <= right) as i64,
                    BinOp::Gt => (left > right) as i64,
                    BinOp::Ge => (left >= right) as i64,
                })
            }
            Expr::Call(name, args) => {
                let values: Vec<Value> = args.iter().map(|arg| self.eval(arg)).collect();
                match name.as_str() {
                    "printf" => {
                        self.builtin_printf(&values);
                        Value::Int(0)
                    }
                    _ => Value::Int(0),
                }
            }
        }
    }

    fn builtin_printf(&mut self, args: &[Value]) {
        let Some(Value::Str(format)) = args.first() else {
            return;
        };

        let mut rendered = String::new();
        let mut next_arg = args[1..].iter();
        let mut chars = format.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '%' && chars.peek() == Some(&'d') {
                chars.next();
                if let Some(value) = next_arg.next() {
                    rendered.push_str(&value.as_int().to_string());
                }
                continue;
            }
            rendered.push(c);
        }

        self.output.push(rendered);
    }
}
