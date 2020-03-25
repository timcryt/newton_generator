use pest::iterators::Pair;
use pest::prec_climber::*;
use pest::Parser;

#[derive(Parser)]
#[grammar = "palette.pest"]
struct PaletteParser;

type IsHidden = bool;

pub fn validate_palette(palette: String) -> Result<(), String> {
    match PaletteParser::parse(Rule::palette, &palette) {
        Ok(_) => Ok(()),
        Err(e1) => match PaletteParser::parse(Rule::gradient, &palette) {
            Ok(_) => Ok(()),
            Err(e2) => Err(format!("{}\n{}", e1, e2)),
        },
    }
}

lazy_static! {
    static ref PREC_CLIMBER: PrecClimber<Rule> = {
        use Assoc::*;
        use Rule::*;

        PrecClimber::new(vec![
            Operator::new(simple_separator, Left) | Operator::new(full_separator, Left),
        ])
    };
}

fn palette_climber(palette_string: &str) -> Vec<(u8, u8, u8, IsHidden)> {
    PREC_CLIMBER.climb(
        PaletteParser::parse(Rule::palette, palette_string).unwrap(),
        |pair: Pair<Rule>| match pair.as_rule() {
            Rule::color => {
                let v = hex::decode(pair.into_inner().as_str()).unwrap();
                vec![(v[0], v[1], v[2], false)]
            }
            Rule::hidden_color => {
                let v = hex::decode(pair.into_inner().as_str()).unwrap();
                vec![(v[0], v[1], v[2], true)]
            }

            _ => unreachable!(),
        },
        |lhs: Vec<(u8, u8, u8, IsHidden)>, op: Pair<Rule>, rhs: Vec<(u8, u8, u8, IsHidden)>| {
            match op.as_rule() {
                Rule::simple_separator => lhs.into_iter().chain(rhs.into_iter()).collect(),
                Rule::full_separator => {
                    let (lf, fr, len) = (
                        lhs[lhs.len() - 1],
                        rhs[0],
                        op.into_inner().as_str().parse::<u16>().unwrap(),
                    );
                    lhs.iter()
                        .copied()
                        .take(lhs.len() - if lf.3 { 1 } else { 0 })
                        .chain((1..=len).map(|i| {
                            (
                                (fr.0 as u16 * i / (len + 1)
                                    + lf.0 as u16 * (len - i + 1) / (len + 1))
                                    as u8,
                                (fr.1 as u16 * i / (len + 1)
                                    + lf.1 as u16 * (len - i + 1) / (len + 1))
                                    as u8,
                                (fr.2 as u16 * i / (len + 1)
                                    + lf.2 as u16 * (len - i + 1) / (len + 1))
                                    as u8,
                                false,
                            )
                        }))
                        .chain(rhs.into_iter().skip(if fr.3 { 1 } else { 0 }))
                        .collect()
                }
                _ => unreachable!(),
            }
        },
    )
}

pub fn get_palette(palette_string: &str) -> Vec<(u8, u8, u8)> {
    palette_climber(palette_string)
        .into_iter()
        .map(|(r, g, b, _)| (r, g, b))
        .chain(vec![(0, 0, 0)])
        .collect()
}
