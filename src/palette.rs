use pest::Parser;

#[derive(Parser)]
#[grammar = "palette.pest"]
struct PaletteParser;

pub fn validate_palette(palette: String) -> Result<(), String> {
    match PaletteParser::parse(Rule::palette, &palette) {
        Ok(_) => Ok(()),
        Err(e1) => match PaletteParser::parse(Rule::gradient, &palette) {
            Ok(_) => Ok(()),
            Err(e2) => Err(format!("{}\n{}", e1, e2)),
        },
    }
}

pub fn get_palette(matches: &clap::ArgMatches) -> Vec<(u8, u8, u8)> {
    let palette_string = matches.value_of("palette").unwrap();

    match PaletteParser::parse(Rule::palette, &palette_string) {
        Ok(p) => p
            .filter_map(|c| match c.as_rule() {
                Rule::color => {
                    let v = hex::decode(c.into_inner().as_str()).unwrap();
                    Some((v[0], v[1], v[2]))
                }
                _ => None,
            })
            .chain(vec![(0, 0, 0)])
            .collect(),
        Err(_) => {
            let mut len = 0;
            let cl = PaletteParser::parse(Rule::gradient, &palette_string)
                .unwrap()
                .filter_map(|c| match c.as_rule() {
                    Rule::color => Some(hex::decode(c.into_inner().as_str()).unwrap()),
                    Rule::num => {
                        len = c.as_str().parse::<u16>().unwrap();
                        None
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();

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
        }
    }
}
