use parsall::prelude::*;

fn operate(l: f64, o: Operation, r: f64) -> f64 {
    match o {
        Operation::Add => l + r,
        Operation::Mul => l * r,
        Operation::Div => l / r,
        Operation::Sub => l - r,
    }
}

enum Operation {
    Add,
    Mul,
    Div,
    Sub,
}

parser_fns! {
    digits(('0'..='9').rep(Ignore));
    float((digits, ('.', digits).opt()).slice().map(|s| s.parse().unwrap())) -> f64;

    op(pmatch! {
        "+" => Operation::Add,
        "-" => Operation::Sub,
        "/" => Operation::Div,
        "*" => Operation::Mul,
    }) -> Operation;

    add(mul.delim_by((['+', '-'].lookahead(), op).pad(sep), lfold(operate))) -> f64;
    mul(term.delim_by((['*', '/'].lookahead(), op).pad(sep), lfold(operate))) -> f64;
    neg('-', sep, term.map(|i| -i)) -> f64;
    term(neg.or(float).or(expr.pad(sep).wrapped("(", ")"))) -> f64;

    pub expr(add) -> f64;
}

fn main() {
    Parser::<ParseError>::repl(add);
}
