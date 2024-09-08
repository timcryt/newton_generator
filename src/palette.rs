use pest::iterators::Pair;
use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest::Parser;

use crate::Color;

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
    static ref PRATT_PARSER: PrattParser<Rule> = PrattParser::new()
        .op(
            Op::infix(Rule::simple_separator, Assoc::Left)
            | Op::infix(Rule::full_separator, Assoc::Left)
            | Op::infix(Rule::default_separator, Assoc::Left)
        )
        .op(Op::postfix(Rule::EOI));
}

pub fn get_palette(palette_string: &str) -> (Vec<Color>, Color) {
    let (palette, defcol) = PRATT_PARSER
        .map_primary(|pair: Pair<Rule>| match pair.as_rule() {
            Rule::color => {
                let v = hex::decode(pair.into_inner().as_str()).unwrap();
                (vec![(Color(v[0], v[1], v[2]), false)], None)
            }
            Rule::hidden_color => {
                let v = hex::decode(pair.into_inner().as_str()).unwrap();
                (vec![(Color(v[0], v[1], v[2]), true)], None)
            }
            _ => unreachable!(),
        })
        .map_infix(|lhs, op, rhs| match op.as_rule() {
            Rule::simple_separator => (lhs.0.into_iter().chain(rhs.0.into_iter()).collect(), None),
                   Rule::full_separator => {
                       let (lf, fr, len) = (
                           lhs.0[lhs.0.len() - 1].0,
                                            rhs.0[0].0,
                                            op.into_inner().as_str().parse::<u16>().unwrap(),
                       );
                       (
                           lhs.0
                           .iter()
                           .copied()
                           .chain((1..=len).map(|i| {
                               (
                                   Color(
                                       (fr.0 as u16 * i / (len + 1)
                                       + lf.0 as u16 * (len - i + 1) / (len + 1))
                                       as u8,
                                       (fr.1 as u16 * i / (len + 1)
                                       + lf.1 as u16 * (len - i + 1) / (len + 1))
                                       as u8,
                                       (fr.2 as u16 * i / (len + 1)
                                       + lf.2 as u16 * (len - i + 1) / (len + 1))
                                       as u8,
                                   ),
                                false,
                               )
                           }))
                           .chain(rhs.0)
                           .collect(),
                        None,
                       )
                   }
                   Rule::default_separator => (lhs.0, Some(rhs.0[0])),
                   _ => unreachable!(),
        })
        .map_postfix(|lhs, op| match op.as_rule() {
            Rule::EOI => lhs,
            _ => unreachable!(),
        })
        .parse(PaletteParser::parse(Rule::palette, palette_string).unwrap());

    (
        palette
            .into_iter()
            .filter_map(|(c, h)| if h { None } else { Some(c) })
            .collect(),
        match defcol {
            None => Color(0, 0, 0),
            Some((c, _)) => c,
        },
    )
}
