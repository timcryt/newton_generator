use clap::{App, Arg};
use num_complex::Complex;
use rayon::prelude::*;
use std::cmp::max;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;


#[macro_use]
extern crate pest_derive;

#[macro_use]
extern crate lazy_static;

use pest::Parser;
use pest::iterators::{Pair, Pairs};
use pest::prec_climber::*;

const PRECISION: f64 = 1e-10;
const ROOT_PRECISION: f64 = 1e-5;
const ROOT_ITER: u16 = 256;
const CONTRAST: f64 = 4.0;

#[derive(Parser)]
#[grammar = "func.pest"]
struct FuncParser;

#[derive(Debug, Clone)]
enum Func {
    Arg,
    Num(f64),
    Add(Box<Func>, Box<Func>),
    Sub(Box<Func>, Box<Func>),
    Mul(Box<Func>, Box<Func>),
    Div(Box<Func>, Box<Func>),
    PowC(Box<Func>, f64),
    PowI(Box<Func>, i32),
}

impl Func {
    fn calc(&self, x: Complex<f64>) -> Complex<f64> {
        match self {
            Func::Arg => x,
            Func::Num(n) => Complex { re: *n, im: 0.0 },
            Func::Add(a, b) => a.calc(x) + b.calc(x),
            Func::Sub(a, b) => a.calc(x) - b.calc(x),
            Func::Mul(a, b) => a.calc(x) * b.calc(x),
            Func::Div(a, b) => a.calc(x) / b.calc(x),
            Func::PowI(a, n) => a.calc(x).powi(*n),
            Func::PowC(a, n) => a.calc(x).powf(*n),
        }
    }

    fn diff(self) -> Func {
        match self {
            Func::Arg => Func::Num(1.0),
            Func::Num(_) => Func::Num(0.0),
            Func::Add(a, b) => a.diff() + b.diff(),
            Func::Sub(a, b) => a.diff() - b.diff(),
            Func::Mul(a, b) => *a.clone() * b.clone().diff() + a.diff() * *b,
            Func::Div(a, b) => {
                (a.clone().diff() * *b.clone() - *a * b.clone().diff()) / (*b.clone() * *b)
            }
            Func::PowI(a, n) => a.clone().diff() * a.powi(n - 1) * n as f64,
            Func::PowC(a, n) => a.clone().diff() * a.powc(n - 1.0) * n as f64,
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
        if n == 0.0 {
            Func::Num(1.0)
        } else if (n - 1.0).abs() < std::f64::EPSILON {
            self
        } else if n.fract() == 0.0 && n < std::i32::MAX as f64 && n > std::i32::MIN as f64 {
            Func::PowI(Box::new(self), n as i32)
        } else {
            Func::PowC(Box::new(self), n)
        }
    }
}

impl std::ops::Add<Func> for Func {
    type Output = Func;

    fn add(self, other: Func) -> Func {
        if let Func::Num(n) = self {
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
        if let Func::Num(n) = other {
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
        if let Func::Num(n) = self {
            if n == 0.0 {
                return Func::Num(0.0);
            } else if (n - 1.0).abs() < std::f64::EPSILON {
                return other;
            }
        } else if let Func::Num(n) = other {
            if n == 0.0 {
                return Func::Num(0.0);
            } else if (n - 1.0).abs() < std::f64::EPSILON {
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
        if let Func::Num(n) = self {
            if n == 0.0 {
                return Func::Num(0.0);
            }
        } else if let Func::Num(n) = other {
            if (n - 1.0).abs() < std::f64::EPSILON {
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

fn find_newton(
    x: Complex<f64>,
    roots: &Option<Vec<Complex<f64>>>,
    f: &Func,
    f_diff: &Func,
    colorize: bool,
    palette: &[(u8, u8, u8)],
) -> (u8, u8, u8) {
    let (root, dep) = find_root(x, f, f_diff);

    match root {
        None => (0, 0, 0),
        Some(root) => {
            if colorize {
                palette[match roots
                    .as_ref()
                    .unwrap()
                    .iter()
                    .enumerate()
                    .find(|x| (*x.1 - root).norm() < ROOT_PRECISION)
                    .unwrap_or((std::usize::MAX, &Complex::default()))
                    .0
                {
                    std::usize::MAX => palette.len() - 1,
                    x => x % (palette.len() - 1),
                }]
            } else {
                let c = 255
                    - (dep as f64 / ROOT_ITER as f64 * std::u8::MAX as f64 * CONTRAST)
                        .floor()
                        .min(255.0) as u8;
                (c, c, c)
            }
        }
    }
}

fn sort_float(v: &mut Vec<Complex<f64>>) {
    v.sort_by(|a, b| {
        if a.re.partial_cmp(&b.re).unwrap() == std::cmp::Ordering::Equal
            || (a.re - b.re).abs() < ROOT_PRECISION
        {
            if a.im.partial_cmp(&b.im).unwrap() == std::cmp::Ordering::Equal
                || (a.im - b.im).abs() < ROOT_PRECISION
            {
                std::cmp::Ordering::Equal
            } else {
                a.im.partial_cmp(&b.re).unwrap()
            }
        } else {
            a.re.partial_cmp(&b.im).unwrap()
        }
    });
}

fn sort_float_rev(v: &mut Vec<Complex<f64>>) {
    v.sort_by(|a, b| {
        if a.im.partial_cmp(&b.im).unwrap() == std::cmp::Ordering::Equal
            || (a.im - b.im).abs() < ROOT_PRECISION
        {
            if a.re.partial_cmp(&b.re).unwrap() == std::cmp::Ordering::Equal
                || (a.re - b.re).abs() < ROOT_PRECISION
            {
                std::cmp::Ordering::Equal
            } else {
                a.re.partial_cmp(&b.re).unwrap()
            }
        } else {
            a.im.partial_cmp(&b.im).unwrap()
        }
    });
}

fn find_root_func(x: Complex<f64>, f: &Func, g: &Func, d: u16) -> (Option<Complex<f64>>, u16) {
    if f.calc(x).norm() < PRECISION {
        (Some(x), d)
    } else if d == ROOT_ITER {
        (None, d)
    } else {
        find_root_func(x - f.calc(x) / g.calc(x), f, g, d + 1)
    }
}

fn find_root(x: Complex<f64>, f: &Func, g: &Func) -> (Option<Complex<f64>>, u16) {
    find_root_func(x, f, g, 0)
}

fn uniq(x: &mut Option<Complex<f64>>, n: Complex<f64>) -> Option<Complex<f64>> {
    let r = if let Some(x) = x {
        if (n - *x).norm() < ROOT_PRECISION {
            None
        } else {
            Some(n)
        }
    } else {
        Some(n)
    };
    *x = Some(n);
    r
}

fn uniq_vec(mut v: Vec<Complex<f64>>) -> Vec<Complex<f64>> {
    sort_float(&mut v);
    let mut x = None;
    v = v
        .into_iter()
        .filter_map(|root| uniq(&mut x, root))
        .collect();

    sort_float_rev(&mut v);
    x = None;
    v.into_iter()
        .filter_map(|root| uniq(&mut x, root))
        .collect()
}

fn find_roots(
    (x1, y1): (f64, f64),
    (x2, y2): (f64, f64),
    f: &Func,
    g: &Func,
    height: u32,
) -> Vec<Complex<f64>> {
    let width = calculate_width((x1, y1), (x2, y2), height);

    uniq_vec(
        (0..height)
            .into_par_iter()
            .map(|i| {
                uniq_vec(
                    (0..width)
                        .filter_map(|j| {
                            find_root(
                                complex_by_coord((i, height), (j, width), (x1, y1), (x2, y2)),
                                f,
                                g,
                            )
                            .0
                        })
                        .collect::<Vec<_>>(),
                )
                .into_par_iter()
            })
            .flatten()
            .collect::<Vec<_>>(),
    )
}

fn complex_by_coord(
    (i, h): (u32, u32),
    (j, w): (u32, u32),
    (x1, y1): (f64, f64),
    (x2, y2): (f64, f64),
) -> Complex<f64> {
    Complex {
        re: x1 + (x2 - x1) * j as f64 / w as f64,
        im: y1 + (y2 - y1) * i as f64 / h as f64,
    }
}

fn calculate_width((x1, y1): (f64, f64), (x2, y2): (f64, f64), height: u32) -> u32 {
    max(((x2 - x1) / (y2 - y1) * height as f64) as u32, 1)
}

fn newton(
    (x1, y1): (f64, f64),
    (x2, y2): (f64, f64),
    f: &Func,
    g: &Func,
    colorize: bool,
    palette: &[(u8, u8, u8)],
    height: u32,
) -> (u32, u32, Vec<u8>) {
    let width = calculate_width((x1, y1), (x2, y2), height);
    let roots = if colorize {
        Some(find_roots((x1, y1), (x2, y2), f, g, max(height / 4, 1)))
    } else {
        None
    };

    (
        width,
        height,
        (0..height)
            .into_par_iter()
            .map(|i| {
                (0..width)
                    .map(|j| {
                        let (r, g, b) = find_newton(
                            complex_by_coord((i, height), (j, width), (x1, y1), (x2, y2)),
                            &roots,
                            f,
                            g,
                            colorize,
                            palette,
                        );
                        vec![r, g, b]
                    })
                    .flatten()
                    .collect::<Vec<_>>()
                    .into_par_iter()
            })
            .flatten()
            .collect::<Vec<_>>(),
    )
}

fn write_png(path: &str, (w, h): (u32, u32), data: &[u8]) -> Result<(), std::io::Error> {
    let path = Path::new(path);
    let file = File::create(path)?;
    let wr = &mut BufWriter::new(file);

    let mut encoder = png::Encoder::new(wr, w, h);
    encoder.set_color(png::ColorType::RGB);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;

    writer.write_image_data(data)?;

    Ok(())
}

fn validate_coord(coord: String) -> Result<(), String> {
    let mut coord_pair = coord.split(';');
    match (coord_pair.next(), coord_pair.next(), coord_pair.next()) {
        (Some(a), Some(b), None) => {
            let (mut coorda, mut coordb) = (a.split(','), b.split(','));
            match (
                coorda.next(),
                coorda.next(),
                coorda.next(),
                coordb.next(),
                coordb.next(),
                coordb.next(),
            ) {
                (Some(x1), Some(y1), None, Some(x2), Some(y2), None) => {
                    match (
                        x1.trim().parse::<f64>(),
                        y1.trim().parse::<f64>(),
                        x2.trim().parse::<f64>(),
                        y2.trim().parse::<f64>(),
                    ) {
                        (Ok(x1), Ok(y1), Ok(x2), Ok(y2)) => {
                            if x1 >= x2 || y1 >= y2 {
                                Err("Конечные координаты должны быть больше начальных".to_string())
                            } else {
                                Ok(())
                            }
                        }
                        _ => Err("Координаты должны быть числами".to_string()),
                    }
                }
                _ => Err("Неправильный формат координат".to_string()),
            }
        }
        _ => Err("Неправильный формат координат".to_string()),
    }
}

fn validate_color(color: &str) -> Result<(), String> {
    let mut rgb = color.split(',');
    match (rgb.next(), rgb.next(), rgb.next(), rgb.next()) {
        (Some(r), Some(g), Some(b), None) => {
            if r.trim().parse::<u8>().is_ok()
                && g.trim().parse::<u8>().is_ok()
                && b.trim().parse::<u8>().is_ok()
            {
                Ok(())
            } else {
                Err("Параметры цвета должны быть целыми от 0 до 255".to_string())
            }
        }
        _ => Err("Неправльный формат цвета".to_string()),
    }
}

fn validate_palette(palette: String) -> Result<(), String> {
    palette
        .split(';')
        .map(validate_color)
        .find(|res| res.is_err())
        .unwrap_or(Ok(()))
}

fn validate_gradient(gradient: String) -> Result<(), String> {
    let mut grad_parts = gradient.split(';');
    match (
        grad_parts.next(),
        grad_parts.next(),
        grad_parts.next(),
        grad_parts.next(),
    ) {
        (Some(c1), Some(c2), Some(len), None) => {
            validate_color(c1)?;
            validate_color(c2)?;
            match len.trim().parse::<u8>() {
                Ok(n) if n != 0 => Ok(()),
                _ => Err("Длина градиента должна быть целым положительным числом".to_string()),
            }
        }
        _ => Err("Неправильный формат градиента".to_string()),
    }
}

fn color_from(c: &str) -> (u8, u8, u8) {
    let t = c
        .split(',')
        .map(|c| c.trim().parse::<u8>().unwrap())
        .collect::<Vec<_>>();
    (t[0], t[1], t[2])
}

fn get_coord(matches: &clap::ArgMatches) -> ((f64, f64), (f64, f64)) {
    matches
        .value_of("coord")
        .map(|v| {
            let x = v
                .split(';')
                .map(|a| {
                    a.split(',')
                        .map(|c| c.trim().parse::<f64>().unwrap())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            ((x[0][0], x[0][1]), (x[1][0], x[1][1]))
        })
        .unwrap_or(((-1.0, -1.0), (1.0, 1.0)))
}

fn get_palette(matches: &clap::ArgMatches) -> Vec<(u8, u8, u8)> {
    matches
        .value_of("palette")
        .map(|p| {
            p.split(';')
                .map(color_from)
                .chain(vec![(0, 0, 0)].into_iter())
                .collect()
        })
        .unwrap_or_else(|| {
            matches
                .value_of("gradient")
                .map(|g| {
                    let mut g = g.split(';');
                    let (c1, c2, len) = (
                        color_from(g.next().unwrap()),
                        color_from(g.next().unwrap()),
                        g.next().unwrap().trim().parse::<u16>().unwrap(),
                    );
                    (0..len)
                        .map(|i| {
                            (
                                (c2.0 as u16 * i / (len - 1)
                                    + c1.0 as u16 * (len - i - 1) / (len - 1))
                                    as u8,
                                (c2.1 as u16 * i / (len - 1)
                                    + c1.1 as u16 * (len - i - 1) / (len - 1))
                                    as u8,
                                (c2.2 as u16 * i / (len - 1)
                                    + c1.2 as u16 * (len - i - 1) / (len - 1))
                                    as u8,
                            )
                        })
                        .chain(vec![(0, 0, 0)].into_iter())
                        .collect()
                })
                .unwrap_or_else(|| {
                    vec![
                        (255, 0, 0),
                        (0, 255, 0),
                        (0, 0, 255),
                        (0, 255, 255),
                        (255, 0, 255),
                        (255, 255, 0),
                        (0, 0, 0),
                    ]
                })
        })
}

lazy_static! {
    static ref PREC_CLIMBER: PrecClimber<Rule> = {
        use Rule::*;
        use Assoc::*;

        PrecClimber::new(vec![
            Operator::new(add, Left) | Operator::new(subtract, Left),
            Operator::new(multiply, Left) | Operator::new(divide, Left),
            Operator::new(power_c, Right),
        ])
    };
}

fn eval_func(expression: Pairs<Rule>) -> Func {
    PREC_CLIMBER.climb(
        expression,
        |pair: Pair<Rule>| match pair.as_rule() {
            Rule::arg => Func::Arg,
            Rule::num => Func::Num(pair.as_str().parse::<f64>().unwrap()),
            Rule::expr => eval_func(pair.into_inner()),
            _ => unreachable!(),
        },
        |lhs: Func, op: Pair<Rule>, rhs: Func| match op.as_rule() {
            Rule::add      => lhs + rhs,
            Rule::subtract => lhs - rhs,
            Rule::multiply => lhs * rhs,
            Rule::divide   => lhs / rhs,
            Rule::power_c  => lhs.powc(match rhs {
                Func::Num(n) => n,
                _ => unreachable!(),
            }),   
            _ => unreachable!(),
        },
    )
}

fn parse_func(func_str: &str) -> Result<Func, impl std::error::Error> {
    match FuncParser::parse(Rule::function, func_str) {
        Ok(f) => Ok(eval_func(f)),
        Err(e) => Err(e),
    }
}

fn main() -> Result<(), std::io::Error> {
    let matches = App::new("Фракталы Ньютона")
        .version("0.1")
        .author("timcryt <tymcrit@gmail.com>")
        .about("Генерирует фракталы Ньютона")
        .arg(
            Arg::with_name("height")
                .short("h")
                .value_name("HEIGHT")
                .help("Устанавливает высоту результурющего изображения")
                .required(true)
                .takes_value(true)
                .validator(|v| match v.trim().parse::<u32>() {
                    Ok(x) if x > 0 => Ok(()),
                    _ => Err(String::from("Высота должна быть целым положтельным числом")),
                }),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .value_name("OUTPUT")
                .help("Уставливает файл изображения")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("function")
                .short("f")
                .value_name("function")
                .help("Устанавливает функцию, по которой строится фрактал")
                .required(true)
                .takes_value(true)
                .validator(|f| match parse_func(&f) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("{}", e)),
                }),
        )
        .arg(
            Arg::with_name("coord")
                .short("c")
                .value_name("X1, Y1; X2, Y2")
                .help("Устанавливает координаты для отобажения фрактала")
                .takes_value(true)
                .validator(validate_coord),
        )
        .arg(
            Arg::with_name("color")
                .long("color")
                .help("Утсанавливливает режим расцветки"),
        )
        .arg(
            Arg::with_name("palette")
                .long("palette")
                .value_name("R, G, B [; R, G, B [...]]")
                .help("Устанавливает палитру в цветном режиме")
                .requires("color")
                .conflicts_with("gradient")
                .takes_value(true)
                .validator(validate_palette),
        )
        .arg(
            Arg::with_name("gradient")
                .long("gradient")
                .value_name("R1, G1, B1; R2, G2, B2; LEN")
                .help("Устанавливает градиент в цветном режиме")
                .requires("color")
                .conflicts_with("palette")
                .takes_value(true)
                .validator(validate_gradient),
        )
        .get_matches();

    let h = matches.value_of("height").unwrap().trim().parse().unwrap();
    let path = matches.value_of("output").unwrap();
    let f = parse_func(&matches.value_of("function").unwrap()).unwrap();

    let (start, end) = get_coord(&matches);

    let colorize = matches.is_present("color");
    let palette = get_palette(&matches);

    let t = std::time::SystemTime::now();

    let g = f.clone().diff();

    let (w, h, v) = newton(start, end, &f, &g, colorize, &palette, h);

    println!("Изображение сгенерировано за {:?}", t.elapsed().unwrap());

    let t = std::time::SystemTime::now();

    write_png(&path, (w, h), &v)?;
    println!("Изображение записано за {:?}", t.elapsed().unwrap());

    Ok(())
}
