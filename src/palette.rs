use pest::iterators::Pair;
use pest::prec_climber::*;
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

lazy_static! {
    static ref PREC_CLIMBER: PrecClimber<Rule> = {
        use Assoc::*;
        use Rule::*;

        PrecClimber::new(vec![
            Operator::new(simple_separator, Left)
                | Operator::new(full_separator, Left)
                | Operator::new(default_separator, Left),
        ])
    };
}

pub fn get_palette(palette_string: &str) -> Vec<(u8, u8, u8)> {
    let (palette, needs_default) = PREC_CLIMBER.climb(
        PaletteParser::parse(Rule::palette, palette_string).unwrap(),
        |pair: Pair<Rule>| match pair.as_rule() {
            Rule::color => {
                let v = hex::decode(pair.into_inner().as_str()).unwrap();
                (vec![(v[0], v[1], v[2], false)], true)
            }
            Rule::hidden_color => {
                let v = hex::decode(pair.into_inner().as_str()).unwrap();
                (vec![(v[0], v[1], v[2], true)], true)
            }

            _ => unreachable!(),
        },
        |lhs, op, rhs| match op.as_rule() {
            Rule::simple_separator => (lhs.0.into_iter().chain(rhs.0.into_iter()).collect(), true),
            Rule::full_separator => {
                let (lf, fr, len) = (
                    lhs.0[lhs.0.len() - 1],
                    rhs.0[0],
                    op.into_inner().as_str().parse::<u16>().unwrap(),
                );
                (
                    lhs.0
                        .iter()
                        .copied()
                        .take(lhs.0.len() - if lf.3 { 1 } else { 0 })
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
                        .chain(rhs.0.into_iter().skip(if fr.3 { 1 } else { 0 }))
                        .collect(),
                    true,
                )
            }
            Rule::default_separator => {
                (lhs.0.into_iter().chain(rhs.0.into_iter()).collect(), false)
            }
            _ => unreachable!(),
        },
    );
    palette
        .into_iter()
        .map(|(r, g, b, _)| (r, g, b))
        .chain(if needs_default {
            vec![(0, 0, 0)]
        } else {
            vec![]
        })
        .collect()
}
