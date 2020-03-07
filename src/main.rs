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

fn newton_func(n: Complex<f64>, roots: &[Complex<f64>], polinom: &[f64], d: u8) -> (u8, u8, u8) {
    let colors = [(255, 0, 0), (0, 255, 0), (0, 0, 255), (0, 127, 127), (127, 0, 127), (127, 127, 0), (0, 0, 0)];    
    if d == 255 || f(n, polinom).norm() < 1e-10 {
        colors[
            match roots.iter()
                .enumerate()
                .find(|x| (*x.1 - n).norm() < 1e-10)
                .unwrap_or((std::usize::MAX, &Complex::default()))
                .0 {
                std::usize::MAX => colors.len() - 1,
                x => x % (colors.len() - 1),
            }
            ]
    } else {
        newton_func(n - f(n, polinom) / g(n, polinom), roots, polinom, d + 1)
    }
}

fn find_newton(x: Complex<f64>, roots: &[Complex<f64>], polinom: &[f64]) -> (u8, u8, u8) {
    newton_func(x, roots, polinom, 0)
}

fn sort_float(v: &mut Vec<Complex<f64>>) {
    v.sort_by(|a, b|
        if a.re.partial_cmp(&b.re).unwrap() == std::cmp::Ordering::Equal || (a.re - b.re).abs() < 1e-10 {
            if a.im.partial_cmp(&b.im).unwrap() == std::cmp::Ordering::Equal || (a.im - b.im).abs() < 1e-10 {
                std::cmp::Ordering::Equal
            } else {
                a.im.partial_cmp(&b.re).unwrap()
            }
        } else {
            a.re.partial_cmp(&b.im).unwrap()
        }

    );
}
fn sort_float_rev(v: &mut Vec<Complex<f64>>) {
    v.sort_by(|a, b|
        if a.im.partial_cmp(&b.im).unwrap() == std::cmp::Ordering::Equal || (a.im - b.im).abs() < 1e-10 {
            if a.re.partial_cmp(&b.re).unwrap() == std::cmp::Ordering::Equal || (a.re - b.re).abs() < 1e-10 {
                std::cmp::Ordering::Equal
            } else {
                a.re.partial_cmp(&b.re).unwrap()
            }
        } else {
            a.im.partial_cmp(&b.im).unwrap()
        }

    );
}

fn find_root_func(x: Complex<f64>, polinom: &[f64], d: u16) -> Option<Complex<f64>> {
    if f(x, polinom).norm() < 1e-10 {
        Some(x)
    } else if d == 1023 {
        None
    } else {
        find_root_func(x - f(x, polinom) / g(x, polinom), polinom, d + 1)
    }
}

fn find_root(x: Complex<f64>, polinom: &[f64]) -> Option<Complex<f64>> {
    find_root_func(x, polinom, 0)
}

fn uniq(x: &mut Option<Complex<f64>>, n: Complex<f64>) -> Option<Complex<f64>> {
    let r = if let Some(x) = x {
        if (n - *x).norm() < 1e-3 {
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
            v = v.into_iter()
                .filter_map(|root| uniq(&mut x, root))
                .collect();


            sort_float_rev(&mut v);

    
            x = None;
            v.into_iter()
                .filter_map(|root| uniq(&mut x, root))
                .collect()
}

fn find_roots((x1, y1): (f64, f64), (x2, y2): (f64, f64), polinom: &[f64]) -> Vec<Complex<f64>> {
    let height = 10;
    let width = ((x2 - x1) / (y2 - y1) * height as f64) as u32;

    let v = (0..height)
        .into_par_iter()
        .map(|i| {
            let v = (0..width)
                .filter_map(|j| {
                    let re: f64 = x1 + (x2 - x1) * j as f64 / width as f64;
                    let im: f64 = y1 + (y2 - y1) * i as f64 / height as f64;
                    find_root(Complex { re, im }, polinom)
                })
                .collect::<Vec<_>>();
           uniq_vec(v).into_par_iter()
        })
        .flatten()
        .collect::<Vec<_>>();

    uniq_vec(v)

}

fn newton(
    (x1, y1): (f64, f64),
    (x2, y2): (f64, f64),
    polinom: &[f64],
    height: u32,
) -> (u32, u32, Vec<u8>) {
    let width = ((x2 - x1) / (y2 - y1) * height as f64) as u32;

    let roots = find_roots((x1, y1), (x2, y2), polinom);

    (
        width,
        height,
        (0..height)
            .into_par_iter()
            .map(|i| {
                (0..width)
                    .map(|j| {
                        let re: f64 = x1 + (x2 - x1) * j as f64 / width as f64;
                        let im: f64 = y1 + (y2 - y1) * i as f64 / height as f64;
                        let (r, g, b) = find_newton(Complex { re, im }, &roots, polinom);
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
                    if v.split(' ').find(|n| *n != "" && n.parse::<f64>().is_err()).is_some() {
                            Err("Многочлен должен состоять из чисел".to_string())
                    } else {
                        Ok(())
                    }
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
