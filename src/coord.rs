pub fn validate_coord(coord: String) -> Result<(), String> {
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

pub fn get_coord(matches: &clap::ArgMatches) -> ((f64, f64), (f64, f64)) {
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
