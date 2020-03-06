use clap::{App, Arg};
use num_complex::Complex;
use rayon::prelude::*;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

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

fn newton_func(n: Complex<f64>, polinom: &[f64], d: u8) -> (u8, u8, u8) {
    if d == 31 || f(n, polinom).norm() < 1e-10 {
        (255 - d * 8, 255 - d * 8, 255 - d * 8)
    } else {
        newton_func(n - f(n, polinom) / g(n, polinom), polinom, d + 1)
    }
}

fn find_newton(x: Complex<f64>, polinom: &[f64]) -> (u8, u8, u8) {
    newton_func(x, polinom, 0)
}

fn newton(
    (x1, y1): (f64, f64),
    (x2, y2): (f64, f64),
    polinom: &[f64],
    height: u32,
) -> (u32, u32, Vec<u8>) {
    let width = ((x2 - x1) / (y2 - y1) * height as f64) as u32;

    (
        width,
        height,
        (0..height)
            .into_par_iter()
            .map(|i| {
                (0..width)
                    .map(|j| {
                        let re: f64 = x1 + ((x2 - x1) * j as f64 / width as f64);
                        let im: f64 = y1 + (y2 - y1) * i as f64 / height as f64;
                        let (r, g, b) = find_newton(Complex { re, im }, polinom);
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
                .validator(|v| {
                    for n in v.split(' ') {
                        if n != "" && n.parse::<f64>().is_err() {
                            return Err("Многочлен должен состоять из чисел".to_string());
                        }
                    }
                    Ok(())
                }),
        )
        .arg(
            Arg::with_name("coord")
                .short("c")
                .value_name("X1, Y1; X2, Y2")
                .help("Устанавливает координаты для отобажения фрактала")
                .takes_value(true)
                .validator(|v| {
                    let mut t = v.split(';');
                    match (t.next(), t.next(), t.next()) {
                        (Some(a), Some(b), None) => {
                            let (mut ta, mut tb) = (a.split(','), b.split(','));
                            match (ta.next(), ta.next(), ta.next(), tb.next(), tb.next(), tb.next()) {
                                (Some(x1), Some(y1), None, Some(x2), Some(y2), None) => {
                                    match (
                                        x1.trim().parse::<f64>(),
                                        y1.trim().parse::<f64>(),
                                        x2.trim().parse::<f64>(),
                                        y2.trim().parse::<f64>(),
                                    ) {
                                        (Ok(x1), Ok(y1), Ok(x2), Ok(y2)) => {
                                            if x1 >= x2 || y1 >= y2 {
                                                Err("Конечные координаты должны быть больше начальных"
                                                    .to_string())
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
                        _ => Err("Неправильный формат координат".to_string())
                    }
                }),
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

    let t = std::time::SystemTime::now();

    let (w, h, v) = newton(start, end, &polinom, h);

    println!("Изображение сгенерировано за {:?}", t.elapsed().unwrap());

    let t = std::time::SystemTime::now();

    write_png(&path, (w, h), &v)?;
    println!("Изображение записано за {:?}", t.elapsed().unwrap());

    Ok(())
}
