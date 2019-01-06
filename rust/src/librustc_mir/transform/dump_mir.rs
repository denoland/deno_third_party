//! This pass just dumps MIR at a specified point.

use std::borrow::Cow;
use std::fmt;
use std::fs::File;
use std::io;

use rustc::mir::Mir;
use rustc::session::config::{OutputFilenames, OutputType};
use rustc::ty::TyCtxt;
use transform::{MirPass, MirSource};
use util as mir_util;

pub struct Marker(pub &'static str);

impl MirPass for Marker {
    fn name<'a>(&'a self) -> Cow<'a, str> {
        Cow::Borrowed(self.0)
    }

    fn run_pass<'a, 'tcx>(&self,
                          _tcx: TyCtxt<'a, 'tcx, 'tcx>,
                          _source: MirSource,
                          _mir: &mut Mir<'tcx>)
    {
    }
}

pub struct Disambiguator {
    is_after: bool
}

impl fmt::Display for Disambiguator {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let title = if self.is_after { "after" } else { "before" };
        write!(formatter, "{}", title)
    }
}


pub fn on_mir_pass<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                             pass_num: &dyn fmt::Display,
                             pass_name: &str,
                             source: MirSource,
                             mir: &Mir<'tcx>,
                             is_after: bool) {
    if mir_util::dump_enabled(tcx, pass_name, source) {
        mir_util::dump_mir(tcx,
                           Some(pass_num),
                           pass_name,
                           &Disambiguator { is_after },
                           source,
                           mir,
                           |_, _| Ok(()) );
    }
}

pub fn emit_mir<'a, 'tcx>(
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    outputs: &OutputFilenames)
    -> io::Result<()>
{
    let path = outputs.path(OutputType::Mir);
    let mut f = File::create(&path)?;
    mir_util::write_mir_pretty(tcx, None, &mut f)?;
    Ok(())
}
