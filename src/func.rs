use pest::iterators::Pairs;
use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest::Parser;

#[derive(Parser)]
#[grammar = "func.pest"]
struct FuncParser;

#[derive(Debug, Clone)]
pub enum Func {
    Arg,
    Num(f64),
    Im,
    Add(Box<Func>, Box<Func>),
    Sub(Box<Func>, Box<Func>),
    Mul(Box<Func>, Box<Func>),
    Div(Box<Func>, Box<Func>),
    PowC(Box<Func>, f64),
    PowI(Box<Func>, i32),
    Sqrt(Box<Func>),
    Exp(Box<Func>),
    Ln(Box<Func>),
    Sin(Box<Func>),
    Cos(Box<Func>),
    Tan(Box<Func>),
}

impl Func {
    pub fn genc(&self, fun_name: &str) -> String {
        format!(
            r#"
#include <complex.h>

double complex {}(double complex x) {{
    return {};
}}

"#,
            fun_name,
            self.gen_inner()
        )
    }

    fn gen_inner(&self) -> String {
        match self {
            Func::Arg => "x".to_owned(),
            Func::Num(n) => n.to_string(),
            Func::Im => "I".to_owned(),
            Func::Add(a, b) => format!("({}+{})", a.gen_inner(), b.gen_inner()),
            Func::Sub(a, b) => format!("({}-{})", a.gen_inner(), b.gen_inner()),
            Func::Mul(a, b) => format!("({}*{})", a.gen_inner(), b.gen_inner()),
            Func::Div(a, b) => format!("({}/{})", a.gen_inner(), b.gen_inner()),
            Func::PowI(a, n) => format!("cpow({}, {})", a.gen_inner(), n),
            Func::PowC(a, n) => format!("cpow({}, {})", a.gen_inner(), n),
            Func::Sqrt(a) => format!("csqrt({})", a.gen_inner()),
            Func::Exp(a) => format!("cexp({})", a.gen_inner()),
            Func::Ln(a) => format!("clog({})", a.gen_inner()),
            Func::Sin(a) => format!("csin({})", a.gen_inner()),
            Func::Cos(a) => format!("ccos({})", a.gen_inner()),
            Func::Tan(a) => format!("ctan({})", a.gen_inner()),
        }
    }

    pub fn diff(self) -> Func {
        match self {
            Func::Arg => Func::Num(1.0),
            Func::Num(_) | Func::Im => Func::Num(0.0),
            Func::Add(a, b) => a.diff() + b.diff(),
            Func::Sub(a, b) => a.diff() - b.diff(),
            Func::Mul(a, b) => *a.clone() * b.clone().diff() + a.diff() * *b,
            Func::Div(a, b) => (a.clone().diff() * *b.clone() - *a * b.clone().diff()) / b.powi(2),
            Func::PowI(a, n) => a.clone().diff() * a.powi(n - 1) * n as f64,
            Func::PowC(a, n) => a.clone().diff() * a.powc(n - 1.0) * n,
            Func::Sqrt(a) => a.clone().diff() / (a.sqrt() * 2.0),
            Func::Exp(a) => a.clone().diff() * a.exp(),
            Func::Ln(a) => a.clone().diff() / *a,
            Func::Sin(a) => a.clone().diff() * a.cos(),
            Func::Cos(a) => 0.0 - a.clone().diff() * a.sin(),
            Func::Tan(a) => a.clone().diff() / a.cos().powi(2),
        }
    }

    fn powi(self, n: i32) -> Func {
        if n == 0 {
            Func::Num(0.0)
        } else if n == 1 {
            self
        } else {
            Func::PowI(Box::new(self), n)
        }
    }

    fn powc(self, n: f64) -> Func {
        if let Func::Num(a) = self {
            Func::Num(a.powf(n))
        } else if n == 0.0 {
            Func::Num(1.0)
        } else if (n - 1.0).abs() < f64::EPSILON {
            self
        } else if (n - 0.5).abs() < f64::EPSILON {
            Func::Sqrt(Box::new(self))
        } else if n.fract() == 0.0 && n < i32::MAX as f64 && n > i32::MIN as f64 {
            Func::PowI(Box::new(self), n as i32)
        } else {
            Func::PowC(Box::new(self), n)
        }
    }

    fn sqrt(self) -> Func {
        if let Func::Num(n) = self {
            Func::Num(n.sqrt())
        } else {
            Func::Sqrt(Box::new(self))
        }
    }

    fn ln(self) -> Func {
        if let Func::Num(n) = self {
            Func::Num(n.ln())
        } else {
            Func::Ln(Box::new(self))
        }
    }

    fn exp(self) -> Func {
        if let Func::Num(n) = self {
            Func::Num(n.exp())
        } else {
            Func::Exp(Box::new(self))
        }
    }

    fn sin(self) -> Func {
        if let Func::Num(n) = self {
            Func::Num(n.sin())
        } else {
            Func::Sin(Box::new(self))
        }
    }

    fn cos(self) -> Func {
        if let Func::Num(n) = self {
            Func::Num(n.cos())
        } else {
            Func::Cos(Box::new(self))
        }
    }

    fn tan(self) -> Func {
        if let Func::Num(n) = self {
            Func::Num(n.tan())
        } else {
            Func::Tan(Box::new(self))
        }
    }
}

impl std::ops::Add<Func> for Func {
    type Output = Func;

    fn add(self, other: Func) -> Func {
        if let (Func::Num(n), Func::Num(m)) = (&self, &other) {
            return Func::Num(n + m);
        } else if let Func::Num(n) = self {
            if n == 0.0 {
                return other;
            }
        } else if let Func::Num(n) = other {
            if n == 0.0 {
                return self;
            }
        }
        Func::Add(Box::new(self), Box::new(other))
    }
}

impl std::ops::Add<f64> for Func {
    type Output = Func;

    fn add(self, other: f64) -> Func {
        self + Func::Num(other)
    }
}

impl std::ops::Sub<Func> for Func {
    type Output = Func;

    fn sub(self, other: Func) -> Func {
        if let (Func::Num(n), Func::Num(m)) = (&self, &other) {
            return Func::Num(n - m);
        } else if let Func::Num(n) = other {
            if n == 0.0 {
                return self;
            }
        }
        Func::Sub(Box::new(self), Box::new(other))
    }
}

impl std::ops::Sub<f64> for Func {
    type Output = Func;

    fn sub(self, other: f64) -> Func {
        self - Func::Num(other)
    }
}

impl std::ops::Sub<Func> for f64 {
    type Output = Func;

    fn sub(self, other: Func) -> Func {
        Func::Num(self) - other
    }
}

impl std::ops::Mul<Func> for Func {
    type Output = Func;

    fn mul(self, other: Func) -> Func {
        if let (Func::Num(n), Func::Num(m)) = (&self, &other) {
            return Func::Num(n * m);
        } else if let Func::Num(n) = self {
            if n == 0.0 {
                return Func::Num(0.0);
            } else if (n - 1.0).abs() < f64::EPSILON {
                return other;
            }
        } else if let Func::Num(n) = other {
            if n == 0.0 {
                return Func::Num(0.0);
            } else if (n - 1.0).abs() < f64::EPSILON {
                return self;
            }
        }
        Func::Mul(Box::new(self), Box::new(other))
    }
}

impl std::ops::Mul<f64> for Func {
    type Output = Func;

    fn mul(self, other: f64) -> Func {
        self * Func::Num(other)
    }
}

impl std::ops::Div<Func> for Func {
    type Output = Func;

    fn div(self, other: Func) -> Func {
        if let (Func::Num(n), Func::Num(m)) = (&self, &other) {
            return Func::Num(n / m);
        } else if let Func::Num(n) = self {
            if n == 0.0 {
                return Func::Num(0.0);
            }
        } else if let Func::Num(n) = other {
            if (n - 1.0).abs() < f64::EPSILON {
                return self;
            }
        }
        Func::Div(Box::new(self), Box::new(other))
    }
}

impl std::ops::Div<f64> for Func {
    type Output = Func;

    fn div(self, other: f64) -> Func {
        self / Func::Num(other)
    }
}

impl std::ops::Div<Func> for f64 {
    type Output = Func;

    fn div(self, other: Func) -> Func {
        Func::Num(self) / other
    }
}

lazy_static! {
    static ref PRATT_PARSER: PrattParser<Rule> = PrattParser::new()
        .op(Op::infix(Rule::add, Assoc::Left) | Op::infix(Rule::subtract, Assoc::Left))
        .op(Op::infix(Rule::multiply, Assoc::Left) | Op::infix(Rule::divide, Assoc::Left))
        .op(Op::infix(Rule::power_c, Assoc::Right))
        .op(Op::postfix(Rule::EOI));
}

fn eval_func(expression: Pairs<Rule>) -> Func {
    PRATT_PARSER
        .map_primary(|pair| match pair.as_rule() {
            Rule::negated_term => Func::Sub(
                Box::new(Func::Num(0.0)),
                Box::new(eval_func(pair.into_inner())),
            ),
            Rule::arg => Func::Arg,
            Rule::num => Func::Num(pair.as_str().parse::<f64>().unwrap()),
            Rule::pi => Func::Num(std::f64::consts::PI),
            Rule::e => Func::Num(std::f64::consts::E),
            Rule::im => Func::Im,
            Rule::expr => eval_func(pair.into_inner()),
            Rule::func_call => {
                let mut inner = pair.into_inner();
                let (func_name, func_arg) = (inner.next().unwrap(), inner.next().unwrap());
                match func_name.as_rule() {
                    Rule::ln => eval_func(func_arg.into_inner()).ln(),
                    Rule::sqrt => eval_func(func_arg.into_inner()).sqrt(),
                    Rule::exp => eval_func(func_arg.into_inner()).exp(),
                    Rule::sin => eval_func(func_arg.into_inner()).sin(),
                    Rule::cos => eval_func(func_arg.into_inner()).cos(),
                    Rule::tan => eval_func(func_arg.into_inner()).tan(),
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        })
        .map_infix(|lhs, op, rhs| match op.as_rule() {
            Rule::add => lhs + rhs,
            Rule::subtract => lhs - rhs,
            Rule::multiply => lhs * rhs,
            Rule::divide => lhs / rhs,
            Rule::power_c => lhs.powc(match rhs {
                Func::Num(n) => n,
                _ => unreachable!(),
            }),
            _ => unreachable!(),
        })
        .map_postfix(|lhs, op| match op.as_rule() {
            Rule::EOI => lhs,
            _ => unreachable!(),
        })
        .parse(expression)
}

pub fn parse_func(func_str: &str) -> Result<Func, Box<dyn std::error::Error>> {
    Ok(FuncParser::parse(Rule::function, func_str).map(eval_func)?)
}
