use pest::Parser;

#[derive(Parser)]
#[grammar = "palette.pest"]
struct PaletteParser;

pub fn validate_palette(palette: String) -> Result<(), String> {
    match PaletteParser::parse(Rule::palette, &palette) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("{}", e))
    }
}

pub fn validate_gradient(gradient: String) -> Result<(), String> {
    match PaletteParser::parse(Rule::gradient, &gradient) {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("{}", e))
    }
}

pub fn get_palette(matches: &clap::ArgMatches) -> Vec<(u8, u8, u8)> {
    matches
        .value_of("palette")
        .map(|p| {
            PaletteParser::parse(Rule::palette, &p).unwrap().filter_map(|c| {
                match c.as_rule() {
                    Rule::color => {
                        let v = hex::decode(c.into_inner().as_str()).unwrap();
                        Some((v[0], v[1], v[2]))
                    }
                    _ => None 
                }
            })
            .chain(vec![(0, 0, 0)])
            .collect()
        })
        .unwrap_or_else(|| {
            matches
                .value_of("gradient")
                .map(|g| {
                    let mut len = 0;                    
                    let cl = PaletteParser::parse(Rule::gradient, &g).unwrap().filter_map(|c| {
                        match c.as_rule() {
                            Rule::color => {
                                Some(hex::decode(c.into_inner().as_str()).unwrap())
                            }
                            Rule::num => {
                                len = dbg!(c.as_str()).parse::<u16>().unwrap();
                                None
                            }
                            _ => None,
                        }
                    }).collect::<Vec<_>>();               

                    (0..len)
                        .map(|i| {
                            (
                                (cl[1][0] as u16 * i / (len - 1)
                                    + cl[0][0] as u16 * (len - i - 1) / (len - 1))
                                    as u8,
                                (cl[1][1] as u16 * i / (len - 1)
                                    + cl[0][1] as u16 * (len - i - 1) / (len - 1))
                                    as u8,
                                (cl[1][2] as u16 * i / (len - 1)
                                    + cl[0][2] as u16 * (len - i - 1) / (len - 1))
                                    as u8,
                            )
                        })
                        .chain(vec![(0, 0, 0)])
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
