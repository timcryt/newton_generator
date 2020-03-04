use num_complex::Complex;
use rayon::prelude::*;
use std::fs::File;
use std::io::{stdin, stdout, BufWriter, Write};
use std::path::Path;

fn f(x: Complex<f64>) -> Complex<f64> {
    x.powi(4) + 2.0 * x.powi(2) - 1.0
}

fn g(x: Complex<f64>) -> Complex<f64> {
    4.0 * x.powi(3) + 4.0 * x.powi(1)
}

fn newton_func(n: Complex<f64>,  d: u8) -> (u8, u8, u8) {
   if d == 31 || f(n).norm() < 1e-10 {
        (255 - d * 8, 255 - d * 8, 255 - d * 8)
    } else {
        newton_func(n - f(n) / g(n), d + 1)
    }
}

fn find_newton(x: Complex<f64>) -> (u8, u8, u8) {
    newton_func(x, 0)
}

fn newton((x1, y1): (f64, f64), (x2, y2): (f64, f64), h: u32) -> (u32, u32, Vec<u8>) {
    let w = ((x2 - x1) / (y2 - y1) * h as f64) as u32;

    (
        w,
        h,
        (0..h)
            .into_par_iter()
            .map(|i| {
                (0..w)
                    .map(|j| {
                        let re: f64 = x1 + ((x2 - x1) * j as f64 / w as f64);
                        let im: f64 = y1 + (y2 - y1) * i as f64 / h as f64;
                        let (r, g, b) = find_newton(Complex { re, im });
                        vec![
                            r,
                            g,
                            b,
                        ]
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
    let h: u32 = {
        print!("Введите высоту изображения: ");
        stdout().flush()?;
        let mut t = String::new();
        stdin().read_line(&mut t)?;
        t.trim().parse().unwrap()
    };

    let t = std::time::SystemTime::now();

    let (w, h, v) = newton((-1.0, -1.0), (1.0, 1.0), h);

    println!("{:?}", t.elapsed());

    let path = {
        print!("Введите имя файла для сохранения: ");
        stdout().flush()?;
        let mut t = String::new();
        std::io::stdin().read_line(&mut t)?;
        t.trim().to_string()
    };

    let t = std::time::SystemTime::now();

    write_png(&path, (w, h), &v)?;
    println!("{:?}", t.elapsed());

    Ok(())
}
