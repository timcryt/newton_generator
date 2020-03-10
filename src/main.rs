use clap::{App, Arg};
use num_complex::Complex;
use rayon::prelude::*;
use std::cmp::max;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

const PRECISION: f64 = 1e-10;
const ROOT_PRECISION: f64 = 1e-5;
const ROOT_ITER: u16 = 256;
const CONTRAST: f64 = 4.0;

fn f(x: Complex<f64>, polinom: &[f64]) -> Complex<f64> {
    polinom
        .iter()
        .enumerate()
        .map(|(i, k)| k * x.powi(i as i32))
        .sum()
}

fn g(x: Complex<f64>, polinom: &[f64]) -> Complex<f64> {
    polinom
        .iter()
        .enumerate()
        .skip(0)
        .map(|(i, k)| k * i as f64 * x.powi(i as i32 - 1))
        .sum()
}

fn find_newton(
    x: Complex<f64>,
    roots: &Option<Vec<Complex<f64>>>,
    polinom: &[f64],
    colorize: bool,
    palette: &[(u8, u8, u8)],
) -> (u8, u8, u8) {
    let (root, d) = find_root(x, polinom);

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
                    - (d as f64 / ROOT_ITER as f64 * std::u8::MAX as f64 * CONTRAST)
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

fn find_root_func(x: Complex<f64>, polinom: &[f64], d: u16) -> (Option<Complex<f64>>, u16) {
    if f(x, polinom).norm() < PRECISION {
        (Some(x), d)
    } else if d == ROOT_ITER {
        (None, d)
    } else {
        find_root_func(x - f(x, polinom) / g(x, polinom), polinom, d + 1)
    }
}

fn find_root(x: Complex<f64>, polinom: &[f64]) -> (Option<Complex<f64>>, u16) {
    find_root_func(x, polinom, 0)
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
    polinom: &[f64],
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
                                polinom,
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
    polinom: &[f64],
    colorize: bool,
    palette: &[(u8, u8, u8)],
    height: u32,
) -> (u32, u32, Vec<u8>) {
    let width = calculate_width((x1, y1), (x2, y2), height);
    let roots = if colorize {
        Some(find_roots((x1, y1), (x2, y2), polinom, max(height / 4, 1)))
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
                            polinom,
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

fn validate_polinom(polinom: String) -> Result<(), String> {
    if polinom
        .split(' ')
        .any(|n| n != "" && n.parse::<f64>().is_err())
    {
        Err("Многочлен должен состоять из чисел".to_string())
    } else if polinom.split(' ').filter(|&n| n != "").count() <= 1 {
        Err("Многочлен должен иметь хотя бы первую степень".to_string())
    } else {
        Ok(())
    }
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
            Arg::with_name("polinom")
                .short("p")
                .value_name("POLINOM")
                .help("Устанавливает многочлен, по которуму строится фрактал")
                .required(true)
                .takes_value(true)
                .validator(validate_polinom),
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
                .takes_value(true)
                .validator(validate_palette),
        )
        .get_matches();

    let h = matches.value_of("height").unwrap().trim().parse().unwrap();
    let path = matches.value_of("output").unwrap();
    let mut polinom = matches
        .value_of("polinom")
        .unwrap()
        .split(' ')
        .filter_map(|x| {
            if x == "" {
                None
            } else {
                Some(x.trim().parse::<f64>().unwrap())
            }
        })
        .collect::<Vec<_>>();
    polinom.reverse();

    let (start, end) = matches
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
        .unwrap_or(((-1.0, -1.0), (1.0, 1.0)));

    let colorize = matches.is_present("color");
    let palette = matches
        .value_of("palette")
        .map(|p| {
            p.split(';')
                .map(|c| {
                    let rgb = c
                        .split(',')
                        .map(|v| v.trim().parse::<u8>().unwrap())
                        .collect::<Vec<_>>();
                    (rgb[0], rgb[1], rgb[2])
                })
                .chain(vec![(0, 0, 0)].into_iter())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![
            (255, 0, 0),
            (0, 255, 0),
            (0, 0, 255),
            (0, 255, 255),
            (255, 0, 255),
            (255, 255, 0),
            (0, 0, 0),
        ]);

    let t = std::time::SystemTime::now();

    let (w, h, v) = newton(start, end, &polinom, colorize, &palette, h);

    println!("Изображение сгенерировано за {:?}", t.elapsed().unwrap());

    let t = std::time::SystemTime::now();

    write_png(&path, (w, h), &v)?;
    println!("Изображение записано за {:?}", t.elapsed().unwrap());

    Ok(())
}
