extern crate termcolor;
extern crate fwdansi;

use termcolor::*;
use std::io;
use std::process::Command;
use fwdansi::write_ansi;

fn main() -> io::Result<()> {
    let output = Command::new("rustc").args(&["--color", "always"]).output()?;

    let mut stderr = StandardStream::stderr(ColorChoice::Always);
    write_ansi(&mut stderr, &output.stderr)?;
    //^ should print "error: no input filename given" with appropriate color everywhere.

    Ok(())
}
