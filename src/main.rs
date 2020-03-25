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

mod coord;
mod func;
mod palette;

use crate::coord::*;
use crate::func::*;
use crate::palette::*;

const PRECISION: f64 = 1e-10;
const ROOT_PRECISION: f64 = 1e-5;
const ROOT_ITER: u16 = 256;
const CONTRAST: f64 = 4.0;

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
        None => {
            if colorize {
                palette[palette.len() - 1]
            } else {
                (0, 0, 0)
            }
        }
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
        Some(find_roots((x1, y1), (x2, y2), f, g, height))
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

fn main() -> Result<(), std::io::Error> {
    let matches = App::new("Фракталы Ньютона")
        .version("0.1")
        .author("timcryt <tymcrit@gmail.com>")
        .about("Генерирует фракталы Ньютона")
        .arg(
            Arg::with_name("height")
                .short("h")
                .value_name("HEIGHT")
                .help("Устанавливает высоту результирующего изображения")
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
                .help("Устанавливает файл изображения")
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
                .help("Устанавливает координаты для отображения фрактала")
                .takes_value(true)
                .validator(validate_coord),
        )
        .arg(
            Arg::with_name("palette")
                .long("palette")
                .value_name("#RRGGBB [-[(LEN1)]> #RRGGBB [-[(LEN2)]> #RRGGBB [...]]]")
                .help("Устанавливает цветной режим и палитру в нём")
                .takes_value(true)
                .validator(validate_palette),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .help("Устанавливает подробный режим")
                .takes_value(false),
        )
        .get_matches();

    let height = matches.value_of("height").unwrap().trim().parse().unwrap();
    let path = matches.value_of("output").unwrap();
    let f = parse_func(&matches.value_of("function").unwrap()).unwrap();
    let (start, end) = get_coord(&matches);
    let verbose = matches.is_present("verbose");
    let colorize = matches.is_present("palette");
    let palette = if colorize {
        get_palette(matches.value_of("palette").unwrap())
    } else {
        vec![]
    };

    let time = std::time::SystemTime::now();

    let g = f.clone().diff();

    let (w, h, v) = newton(start, end, &f, &g, colorize, &palette, height);

    if verbose {
        eprintln!("Изображение сгенерировано за {:?}", time.elapsed().unwrap());
    }

    let time = std::time::SystemTime::now();

    write_png(&path, (w, h), &v)?;

    if verbose {
        eprintln!("Изображение записано за {:?}", time.elapsed().unwrap());
    }

    Ok(())
}
