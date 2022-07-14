use clap::{App, Arg};
use num_complex::Complex;
use rayon::prelude::*;
use std::cmp::max;
use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[macro_use]
extern crate pest_derive;

#[macro_use]
extern crate lazy_static;

#[derive(Clone, Copy)]
pub struct Color(u8, u8, u8);

impl std::ops::Mul<f64> for Color {
    type Output = Color;

    fn mul(self, other: f64) -> Color {
        debug_assert!((0.0..=1.0).contains(&other));
        Color(
            (self.0 as f64 * other).round() as u8,
            (self.1 as f64 * other).round() as u8,
            (self.2 as f64 * other).round() as u8,
        )
    }
}

impl std::ops::Add<Color> for Color {
    type Output = Color;

    fn add(self, other: Color) -> Color {
        Color(self.0 + other.0, self.1 + other.1, self.2 + other.2)
    }
}

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

const PIXEL_COUNT_FREQ: Duration = Duration::from_millis(1000);

fn find_newton(
    x: Complex<f64>,
    roots: &Option<Vec<Complex<f64>>>,
    palette: Option<&(Vec<Color>, Color)>,
    shadow: f64,
) -> Color {
    let (root, dep) = find_root(x);

    match root {
        None => {
            if let Some((_, defcol)) = palette {
                *defcol
            } else {
                Color(0, 0, 0)
            }
        }
        Some(root) => match palette {
            Some((palette, defcol)) => match roots
                .as_ref()
                .unwrap()
                .iter()
                .enumerate()
                .find(|x| (*x.1 - root).norm() < ROOT_PRECISION)
            {
                Some((x, _)) => palette[x % palette.len()] * (1.0 - shadow) + *defcol * shadow,
                None => *defcol,
            },
            None => {
                Color(255, 255, 255) * (1.0 - dep as f64 / ROOT_ITER as f64 * CONTRAST).max(0.0)
            }
        },
    }
}

fn sort_float(v: &mut Vec<Complex<f64>>) {
    let mut i = 0;
    let mut j = 0;
    while i < v.len() {
        if !v[i].re.is_nan() && !v[i].im.is_nan() {
            v[j] = v[i];
            j += 1;
        }
        i += 1;
    }

    v.resize(j, Complex::default());

    //println!("{:?}", v);

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

fn sort_float_rev(v: &mut [Complex<f64>]) {
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

fn find_root(mut x: Complex<f64>) -> (Option<Complex<f64>>, u16) {
    match (0..ROOT_ITER)
        .map(|i| {
            let t = x;

            let fc = unsafe { F_FUNC(t) };
            let gc = unsafe { G_FUNC(t) };

            x = t - fc / gc;
            (i, fc)
        })
        .find(|(_, x)| x.norm() < PRECISION)
    {
        Some((i, _)) => (Some(x), i),
        None => (None, ROOT_ITER),
    }
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
    height: u32,
    verbose: bool,
) -> Vec<Complex<f64>> {
    let width = calculate_width((x1, y1), (x2, y2), height);

    let counter = if verbose {
        Some(count_pixels("Поиск корней: ", (height * width) as usize))
    } else {
        None
    };

    let mut roots = uniq_vec(
        (0..height)
            .into_par_iter()
            .flat_map(|i| {
                uniq_vec(
                    (0..width)
                        .filter_map(|j| {
                            if let Some(ref counter) = counter {
                                counter.fetch_add(1, Ordering::Relaxed);
                            }
                            find_root(complex_by_coord(
                                (i, height),
                                (j, width),
                                (x1, y1),
                                (x2, y2),
                            ))
                            .0
                        })
                        .collect::<Vec<_>>(),
                )
                .into_par_iter()
            })
            .collect::<Vec<_>>(),
    );

    sort_float(&mut roots);

    roots
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

fn count_pixels(intro: &'static str, max: usize) -> Arc<AtomicUsize> {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    thread::spawn(move || {
        while counter.load(Ordering::Relaxed) < max {
            let count = counter.load(Ordering::Relaxed);
            eprintln!(
                "{} {:5.2}% ({:10}/{:10})",
                intro,
                100.0 * count as f64 / max as f64,
                count,
                max
            );
            thread::sleep(PIXEL_COUNT_FREQ);
        }
    });
    counter_clone
}

fn get_shadow(
    z1: (f64, f64),
    z2: (f64, f64),
    height: u32,
    verbose: bool,
) -> HashMap<(u32, u32), u32> {
    let width = calculate_width(z1, z2, height);

    let counter = if verbose {
        Some(count_pixels("Рассчёт теней: ", (height * width) as usize))
    } else {
        None
    };

    let mut buf: VecDeque<_> = (0..height)
        .into_par_iter()
        .flat_map(|i| {
            (0..width)
                .filter_map(|j| {
                    if let Some(counter) = counter.as_ref() {
                        counter.fetch_add(1, Ordering::Relaxed);
                    }
                    if find_root(complex_by_coord((i, height), (j, width), z1, z2))
                        .0
                        .is_none()
                    {
                        Some(((i, j), 0))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .into_par_iter()
        })
        .collect();

    let mut res: HashMap<(u32, u32), u32> = buf.iter().copied().collect();

    while !buf.is_empty() {
        let ((i, j), dist) = buf.pop_back().unwrap();

        if i > 0 && !res.contains_key(&(i - 1, j)) {
            buf.push_front(((i - 1, j), dist + 1));
            res.insert((i - 1, j), dist + 1);
        }
        if j > 0 && !res.contains_key(&(i, j - 1)) {
            buf.push_front(((i, j - 1), dist + 1));
            res.insert((i, j - 1), dist + 1);
        }
        if i < height - 1 && !res.contains_key(&(i + 1, j)) {
            buf.push_front(((i + 1, j), dist + 1));
            res.insert((i + 1, j), dist + 1);
        }
        if j < width - 1 && !res.contains_key(&(i, j + 1)) {
            buf.push_front(((i, j + 1), dist + 1));
            res.insert((i, j + 1), dist + 1);
        }
    }

    res
}

fn newton(
    z1: (f64, f64),
    z2: (f64, f64),
    palette: Option<&(Vec<Color>, Color)>,
    needs_shadow: Option<f64>,
    height: u32,
    verbose: bool,
    negate: bool,
) -> (u32, u32, Vec<u8>) {
    let width = calculate_width(z1, z2, height);
    let roots = if palette.is_some() {
        Some(find_roots(z1, z2, height, verbose))
    } else {
        None
    };

    let shadow = if needs_shadow.is_some() {
        get_shadow(z1, z2, height, verbose)
    } else {
        HashMap::new()
    };

    let counter = if verbose {
        Some(count_pixels(
            "Генерация фрактала: ",
            (height * width) as usize,
        ))
    } else {
        None
    };

    (
        width,
        height,
        (0..height)
            .into_par_iter()
            .flat_map(|i| {
                (0..width)
                    .flat_map(|j| {
                        let Color(r, g, b) = find_newton(
                            complex_by_coord((i, height), (j, width), z1, z2),
                            &roots,
                            palette,
                            match shadow.get(&(i, j)) {
                                Some(&x) => {
                                    (-(x as f64) * needs_shadow.unwrap() / height as f64).exp()
                                }
                                None => 0.0,
                            },
                        );
                        if let Some(ref counter) = counter {
                            counter.fetch_add(1, Ordering::Relaxed);
                        }
                        if negate {
                            vec![255 - r, 255 - g, 255 - b]
                        } else {
                            vec![r, g, b]
                        }
                    })
                    .collect::<Vec<_>>()
                    .into_par_iter()
            })
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

static mut LIB_FUNC: Option<libloading::Library> = None;

static mut F_FUNC: libloading::Symbol<unsafe extern "C" fn(Complex<f64>) -> Complex<f64>> =
    unsafe { std::mem::transmute(0usize) };
static mut G_FUNC: libloading::Symbol<unsafe extern "C" fn(Complex<f64>) -> Complex<f64>> =
    unsafe { std::mem::transmute(0usize) };

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
            Arg::with_name("shadow")
                .long("shadow")
                .requires("palette")
                .help("Включает режим тени")
                .takes_value(true)
                .validator(|x| match x.trim().parse::<f64>() {
                    Ok(x) if x > 0.0 => Ok(()),
                    _ => Err("Параметр должен быть положительным числом".to_string()),
                }),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .help("Устанавливает подробный режим"),
        )
        .arg(
            Arg::with_name("negate")
                .short("n")
                .conflicts_with("palette")
                .help("Инвертирует цвета"),
        )
        .get_matches();

    let height = matches.value_of("height").unwrap().trim().parse().unwrap();
    let path = matches.value_of("output").unwrap();
    let f = parse_func(matches.value_of("function").unwrap()).unwrap();
    let (start, end) = get_coord(&matches);
    let verbose = matches.is_present("verbose");
    let palette = matches.value_of("palette").map(get_palette);
    let shadow = matches
        .value_of("shadow")
        .map(|x| x.trim().parse().unwrap());
    let negate = matches.is_present("negate");

    let g = f.clone().diff();

    let time = std::time::Instant::now();

    {
        let mut file = File::create("jit.c").unwrap();
        writeln!(file, "{}", f.genc("func")).unwrap();
        writeln!(file, "{}", g.genc("diff")).unwrap();
        std::mem::drop(file);

        Command::new("sh")
            .arg("-c")
            .arg("gcc -O3 -fPIC -c jit.c -lm && gcc -shared -o jit.so jit.o")
            .output()
            .unwrap();

        unsafe {
            LIB_FUNC = Some(libloading::Library::new("./jit.so").unwrap());

            F_FUNC = LIB_FUNC.as_mut().unwrap().get(b"func").unwrap();
            G_FUNC = LIB_FUNC.as_mut().unwrap().get(b"diff").unwrap();
        };
    }

    Command::new("sh")
        .arg("-c")
        .arg("rm jit.c jit.o jit.so")
        .output()
        .unwrap();

    if verbose {
        eprintln!("Функции скомпилированы за {:?}", time.elapsed());
    }

    let time = std::time::Instant::now();

    let (w, h, v) = newton(
        start,
        end,
        palette.as_ref(),
        shadow,
        height,
        verbose,
        negate,
    );

    if verbose {
        eprintln!("Изображение сгенерировано за {:?}", time.elapsed());
    }

    let time = std::time::Instant::now();

    write_png(path, (w, h), &v)?;

    if verbose {
        eprintln!("Изображение записано за {:?}", time.elapsed());
    }

    Ok(())
}
