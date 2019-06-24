//! Write colored strings with ANSI escape code into a `termcolor` terminal.
//!
//! This package provides a single function, [`write_ansi`], which parses ANSI
//! escape codes in the provided byte string and transforms them into the
//! corresponding `termcolor` commands. The colors will be supported even on a
//! Windows console.
//!
//! The main purpose of this package is to forward colored output from a child
//! process.
//!
//! ```rust
// #![doc(include = "../examples/rustc.rs")] // still unstable, see issue 44732
//! extern crate termcolor;
//! extern crate fwdansi;
//!
//! use termcolor::*;
//! use std::io;
//! use std::process::Command;
//! use fwdansi::write_ansi;
//!
//! fn main() -> io::Result<()> {
//!     let output = Command::new("rustc").args(&["--color", "always"]).output()?;
//!
//!     let mut stderr = StandardStream::stderr(ColorChoice::Always);
//!     write_ansi(&mut stderr, &output.stderr)?;
//!     //^ should print "error: no input filename given" with appropriate color everywhere.
//!
//!     Ok(())
//! }
//! ```

extern crate termcolor;
extern crate memchr;

use memchr::memchr;
use termcolor::{Color, ColorSpec, WriteColor};

use std::io;
use std::mem;

/// Writes a string with ANSI escape code into the colored output stream.
///
/// Only SGR (`\x1b[…m`) is supported. Other input will be printed as-is.
pub fn write_ansi<W: WriteColor>(mut writer: W, mut ansi: &[u8]) -> io::Result<()> {
    while let Some(index) = memchr(0x1b, ansi) {
        let (left, right) = ansi.split_at(index);
        writer.write_all(left)?;
        if right.is_empty() {
            return Ok(());
        }

        let mut parser = ColorSpecParser::new(right);
        parser.parse();
        if parser.ansi.as_ptr() == right.as_ptr() {
            writer.write_all(&right[..1])?;
            ansi = &right[1..];
        } else {
            writer.set_color(&parser.spec)?;
            ansi = parser.ansi;
        }
    }
    writer.write_all(ansi)
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum State {
    Normal,
    PrepareCustomColor,
    Ansi256,
    Rgb,
}
struct ColorSpecParser<'a> {
    spec: ColorSpec,
    ansi: &'a [u8],
    reset: bool,
    state: State,
    is_bg: bool,
    red: Option<u8>,
    green: Option<u8>,
}
impl<'a> ColorSpecParser<'a> {
    fn new(ansi: &'a [u8]) -> Self {
        Self {
            spec: ColorSpec::new(),
            ansi,
            reset: false,
            state: State::Normal,
            is_bg: false,
            red: None,
            green: None,
        }
    }

    fn parse(&mut self) {
        #[derive(PartialEq, Eq)]
        enum Expected {
            Escape,
            OpenBracket,
            Number(u8),
        }

        while !self.ansi.is_empty() {
            let mut expected = Expected::Escape;
            let mut it = self.ansi.iter();
            for b in &mut it {
                match (*b, expected) {
                    (0x1b, Expected::Escape) => {
                        expected = Expected::OpenBracket;
                        continue;
                    }
                    (b'[', Expected::OpenBracket) => {
                        expected = Expected::Number(0);
                        continue;
                    }
                    (b'0'..=b'9', Expected::Number(number)) => {
                        if let Some(n) = number.checked_mul(10).and_then(|n| n.checked_add(b - b'0')) {
                            expected = Expected::Number(n);
                            continue;
                        }
                    }
                    (b':', Expected::Number(number))
                    | (b';', Expected::Number(number))
                    | (b'm', Expected::Number(number)) => {
                        if self.apply_number(number) {
                            if *b == b'm' {
                                expected = Expected::Escape;
                                break;
                            } else {
                                expected = Expected::Number(0);
                                continue;
                            }
                        }
                    }
                    _ => {}
                }
                return;
            }
            if let Expected::Escape = expected {
                self.ansi = it.as_slice();
            } else {
                break;
            }
        }
    }

    fn set_color(&mut self, color: Color) {
        if self.is_bg {
            self.spec.set_bg(Some(color));
        } else {
            self.spec.set_fg(Some(color));
        }
    }

    fn apply_number(&mut self, number: u8) -> bool {
        match (number, self.state) {
            (0, State::Normal) => {
                if mem::replace(&mut self.reset, true) {
                    return false;
                }
            }
            (1, State::Normal) => {
                self.spec.set_bold(true);
            }
            (4, State::Normal) => {
                self.spec.set_underline(true);
            }
            (21, State::Normal) => {
                self.spec.set_bold(false);
            }
            (24, State::Normal) => {
                self.spec.set_underline(false);
            }
            (38, State::Normal) | (48, State::Normal) => {
                self.is_bg = number == 48;
                self.state = State::PrepareCustomColor;
            }
            (30..=39, State::Normal) => {
                self.spec.set_fg(parse_color(number - 30));
            }
            (40..=49, State::Normal) => {
                self.spec.set_bg(parse_color(number - 40));
            }
            (90..=97, State::Normal) => {
                self.spec.set_intense(true).set_fg(parse_color(number - 90));
            }
            (100..=107, State::Normal) => {
                self.spec.set_intense(true).set_bg(parse_color(number - 100));
            }
            (5, State::PrepareCustomColor) => {
                self.state = State::Ansi256;
            }
            (2, State::PrepareCustomColor) => {
                self.state = State::Rgb;
                self.red = None;
                self.green = None;
            }
            (n, State::Ansi256) => {
                self.set_color(Color::Ansi256(n));
                self.state = State::Normal;
            }
            (b, State::Rgb) => {
                match (self.red, self.green) {
                    (None, _) => {
                        self.red = Some(b);
                    }
                    (Some(_), None) => {
                        self.green = Some(b);
                    }
                    (Some(r), Some(g)) => {
                        self.set_color(Color::Rgb(r, g, b));
                        self.state = State::Normal;
                    }
                }
            }
            _ => {
                self.state = State::Normal;
            }
        }
        true
    }
}

fn parse_color(digit: u8) -> Option<Color> {
    match digit {
        0 => Some(Color::Black),
        1 => Some(Color::Red),
        2 => Some(Color::Green),
        3 => Some(Color::Yellow),
        4 => Some(Color::Blue),
        5 => Some(Color::Magenta),
        6 => Some(Color::Cyan),
        7 => Some(Color::White),
        _ => None,
    }
}
