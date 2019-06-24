#[macro_use]
extern crate proptest;
extern crate fwdansi;
extern crate termcolor;

use fwdansi::write_ansi;
use proptest::prelude::*;
use std::io;
use termcolor::{Ansi, Color, ColorSpec, WriteColor};

proptest! {
    #[test]
    fn check_no_crash(ref ansi_string in any::<Vec<u8>>()) {
        let mut ansi = Ansi::new(Vec::with_capacity(ansi_string.len()));
        prop_assert!(write_ansi(&mut ansi, &ansi_string).is_ok());
    }

    #[test]
    fn ansi_idempotent(ref elements in prop::collection::vec(Element::any(), 0..100)) {
        // first, write the test string into an ANSI buffer.
        let mut original = Ansi::new(Vec::new());
        for e in elements {
            e.write(&mut original).unwrap();
        }

        // recover the original string, and forward it using `write_ansi`.
        let original = original.into_inner();
        let mut forwarded = Ansi::new(Vec::with_capacity(original.len()));
        prop_assert!(write_ansi(&mut forwarded, &original).is_ok());
        prop_assert_eq!(original, forwarded.into_inner());
    }
}

#[derive(Debug, Clone)]
enum Element {
    ColorSpec(ColorSpec),
    Reset,
    Text(Vec<u8>),
}

fn any_opt_color() -> impl Strategy<Value = Option<Color>> {
    let color = prop_oneof![
        Just(Color::Black),
        Just(Color::Red),
        Just(Color::Green),
        Just(Color::Yellow),
        Just(Color::Blue),
        Just(Color::Magenta),
        Just(Color::Cyan),
        Just(Color::White),
        any::<u8>().prop_map(Color::Ansi256),
        any::<[u8; 3]>().prop_map(|[r, g, b]| Color::Rgb(r, g, b)),
    ];
    prop::option::weighted(0.9, color)
}

prop_compose! {
    fn any_color_spec()(
        bold in any::<bool>(),
        underline in any::<bool>(),
        intense in any::<bool>(),
        fg_color in any_opt_color(),
        bg_color in any_opt_color(),
    ) -> ColorSpec {
        let mut spec = ColorSpec::new();
        spec.set_bold(bold)
            .set_underline(underline)
            .set_intense(intense)
            .set_fg(fg_color)
            .set_bg(bg_color);
        spec
    }
}

impl Element {
    fn any() -> impl Strategy<Value = Self> {
        prop_oneof![
            Just(Element::Reset),
            any_color_spec().prop_map(Element::ColorSpec),
            any::<Vec<u8>>()
                .prop_filter_map(
                    "ignored empty SGR",
                    |v| if v.windows(3).find(|w| w == b"\x1b[m").is_some() {
                        None
                    } else {
                        Some(Element::Text(v))
                    }
                ),
        ]
    }

    fn write<W: WriteColor>(&self, mut w: W) -> io::Result<()> {
        match self {
            Element::ColorSpec(cs) => w.set_color(cs),
            Element::Reset => w.reset(),
            Element::Text(text) => w.write_all(text),
        }
    }
}
