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

pub fn validate_palette(palette: String) -> Result<(), String> {
    palette
        .split(';')
        .map(validate_color)
        .find(|res| res.is_err())
        .unwrap_or(Ok(()))
}

pub fn validate_gradient(gradient: String) -> Result<(), String> {
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


pub fn get_palette(matches: &clap::ArgMatches) -> Vec<(u8, u8, u8)> {
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
