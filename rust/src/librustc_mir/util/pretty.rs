// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use rustc::hir;
use rustc::hir::def_id::{DefId, LOCAL_CRATE};
use rustc::mir::*;
use rustc::mir::visit::Visitor;
use rustc::ty::{self, TyCtxt};
use rustc::ty::item_path;
use rustc_data_structures::fx::FxHashMap;
use rustc_data_structures::indexed_vec::Idx;
use std::fmt::Display;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use super::graphviz::write_mir_fn_graphviz;
use transform::MirSource;

const INDENT: &'static str = "    ";
/// Alignment for lining up comments following MIR statements
pub(crate) const ALIGN: usize = 40;

/// An indication of where we are in the control flow graph. Used for printing
/// extra information in `dump_mir`
pub enum PassWhere {
    /// We have not started dumping the control flow graph, but we are about to.
    BeforeCFG,

    /// We just finished dumping the control flow graph. This is right before EOF
    AfterCFG,

    /// We are about to start dumping the given basic block.
    BeforeBlock(BasicBlock),

    /// We are just about to dump the given statement or terminator.
    BeforeLocation(Location),

    /// We just dumped the given statement or terminator.
    AfterLocation(Location),
}

/// If the session is properly configured, dumps a human-readable
/// representation of the mir into:
///
/// ```text
/// rustc.node<node_id>.<pass_num>.<pass_name>.<disambiguator>
/// ```
///
/// Output from this function is controlled by passing `-Z dump-mir=<filter>`,
/// where `<filter>` takes the following forms:
///
/// - `all` -- dump MIR for all fns, all passes, all everything
/// - a filter defined by a set of substrings combined with `&` and `|`
///   (`&` has higher precedence). At least one of the `|`-separated groups
///   must match; an `|`-separated group matches if all of its `&`-separated
///   substrings are matched.
///
/// Example:
///
/// - `nll` == match if `nll` appears in the name
/// - `foo & nll` == match if `foo` and `nll` both appear in the name
/// - `foo & nll | typeck` == match if `foo` and `nll` both appear in the name
///   or `typeck` appears in the name.
/// - `foo & nll | bar & typeck` == match if `foo` and `nll` both appear in the name
///   or `typeck` and `bar` both appear in the name.
pub fn dump_mir<'a, 'gcx, 'tcx, F>(
    tcx: TyCtxt<'a, 'gcx, 'tcx>,
    pass_num: Option<&dyn Display>,
    pass_name: &str,
    disambiguator: &dyn Display,
    source: MirSource,
    mir: &Mir<'tcx>,
    extra_data: F,
) where
    F: FnMut(PassWhere, &mut dyn Write) -> io::Result<()>,
{
    if !dump_enabled(tcx, pass_name, source) {
        return;
    }

    let node_path = item_path::with_forced_impl_filename_line(|| {
        // see notes on #41697 below
        tcx.item_path_str(source.def_id)
    });
    dump_matched_mir_node(
        tcx,
        pass_num,
        pass_name,
        &node_path,
        disambiguator,
        source,
        mir,
        extra_data,
    );
}

pub fn dump_enabled<'a, 'gcx, 'tcx>(
    tcx: TyCtxt<'a, 'gcx, 'tcx>,
    pass_name: &str,
    source: MirSource,
) -> bool {
    let filters = match tcx.sess.opts.debugging_opts.dump_mir {
        None => return false,
        Some(ref filters) => filters,
    };
    let node_path = item_path::with_forced_impl_filename_line(|| {
        // see notes on #41697 below
        tcx.item_path_str(source.def_id)
    });
    filters.split("|").any(|or_filter| {
        or_filter.split("&").all(|and_filter| {
            and_filter == "all" || pass_name.contains(and_filter) || node_path.contains(and_filter)
        })
    })
}

// #41697 -- we use `with_forced_impl_filename_line()` because
// `item_path_str()` would otherwise trigger `type_of`, and this can
// run while we are already attempting to evaluate `type_of`.

fn dump_matched_mir_node<'a, 'gcx, 'tcx, F>(
    tcx: TyCtxt<'a, 'gcx, 'tcx>,
    pass_num: Option<&dyn Display>,
    pass_name: &str,
    node_path: &str,
    disambiguator: &dyn Display,
    source: MirSource,
    mir: &Mir<'tcx>,
    mut extra_data: F,
) where
    F: FnMut(PassWhere, &mut dyn Write) -> io::Result<()>,
{
    let _: io::Result<()> = do catch {
        let mut file = create_dump_file(tcx, "mir", pass_num, pass_name, disambiguator, source)?;
        writeln!(file, "// MIR for `{}`", node_path)?;
        writeln!(file, "// source = {:?}", source)?;
        writeln!(file, "// pass_name = {}", pass_name)?;
        writeln!(file, "// disambiguator = {}", disambiguator)?;
        if let Some(ref layout) = mir.generator_layout {
            writeln!(file, "// generator_layout = {:?}", layout)?;
        }
        writeln!(file, "")?;
        extra_data(PassWhere::BeforeCFG, &mut file)?;
        write_mir_fn(tcx, source, mir, &mut extra_data, &mut file)?;
        extra_data(PassWhere::AfterCFG, &mut file)?;
    };

    if tcx.sess.opts.debugging_opts.dump_mir_graphviz {
        let _: io::Result<()> = do catch {
            let mut file =
                create_dump_file(tcx, "dot", pass_num, pass_name, disambiguator, source)?;
            write_mir_fn_graphviz(tcx, source.def_id, mir, &mut file)?;
        };
    }
}

/// Returns the path to the filename where we should dump a given MIR.
/// Also used by other bits of code (e.g., NLL inference) that dump
/// graphviz data or other things.
fn dump_path(
    tcx: TyCtxt<'_, '_, '_>,
    extension: &str,
    pass_num: Option<&dyn Display>,
    pass_name: &str,
    disambiguator: &dyn Display,
    source: MirSource,
) -> PathBuf {
    let promotion_id = match source.promoted {
        Some(id) => format!("-{:?}", id),
        None => String::new(),
    };

    let pass_num = if tcx.sess.opts.debugging_opts.dump_mir_exclude_pass_number {
        format!("")
    } else {
        match pass_num {
            None => format!(".-------"),
            Some(pass_num) => format!(".{}", pass_num),
        }
    };

    let mut file_path = PathBuf::new();
    file_path.push(Path::new(&tcx.sess.opts.debugging_opts.dump_mir_dir));

    let item_name = tcx.hir
        .def_path(source.def_id)
        .to_filename_friendly_no_crate();

    let file_name = format!(
        "rustc.{}{}{}.{}.{}.{}",
        item_name,
        promotion_id,
        pass_num,
        pass_name,
        disambiguator,
        extension,
    );

    file_path.push(&file_name);

    file_path
}

/// Attempts to open a file where we should dump a given MIR or other
/// bit of MIR-related data. Used by `mir-dump`, but also by other
/// bits of code (e.g., NLL inference) that dump graphviz data or
/// other things, and hence takes the extension as an argument.
pub(crate) fn create_dump_file(
    tcx: TyCtxt<'_, '_, '_>,
    extension: &str,
    pass_num: Option<&dyn Display>,
    pass_name: &str,
    disambiguator: &dyn Display,
    source: MirSource,
) -> io::Result<fs::File> {
    let file_path = dump_path(tcx, extension, pass_num, pass_name, disambiguator, source);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::File::create(&file_path)
}

/// Write out a human-readable textual representation for the given MIR.
pub fn write_mir_pretty<'a, 'gcx, 'tcx>(
    tcx: TyCtxt<'a, 'gcx, 'tcx>,
    single: Option<DefId>,
    w: &mut dyn Write,
) -> io::Result<()> {
    writeln!(
        w,
        "// WARNING: This output format is intended for human consumers only"
    )?;
    writeln!(
        w,
        "// and is subject to change without notice. Knock yourself out."
    )?;

    let mut first = true;
    for def_id in dump_mir_def_ids(tcx, single) {
        let mir = &tcx.optimized_mir(def_id);

        if first {
            first = false;
        } else {
            // Put empty lines between all items
            writeln!(w, "")?;
        }

        write_mir_fn(tcx, MirSource::item(def_id), mir, &mut |_, _| Ok(()), w)?;

        for (i, mir) in mir.promoted.iter_enumerated() {
            writeln!(w, "")?;
            let src = MirSource {
                def_id,
                promoted: Some(i),
            };
            write_mir_fn(tcx, src, mir, &mut |_, _| Ok(()), w)?;
        }
    }
    Ok(())
}

pub fn write_mir_fn<'a, 'gcx, 'tcx, F>(
    tcx: TyCtxt<'a, 'gcx, 'tcx>,
    src: MirSource,
    mir: &Mir<'tcx>,
    extra_data: &mut F,
    w: &mut dyn Write,
) -> io::Result<()>
where
    F: FnMut(PassWhere, &mut dyn Write) -> io::Result<()>,
{
    write_mir_intro(tcx, src, mir, w)?;
    for block in mir.basic_blocks().indices() {
        extra_data(PassWhere::BeforeBlock(block), w)?;
        write_basic_block(tcx, block, mir, extra_data, w)?;
        if block.index() + 1 != mir.basic_blocks().len() {
            writeln!(w, "")?;
        }
    }

    writeln!(w, "}}")?;
    Ok(())
}

/// Write out a human-readable textual representation for the given basic block.
pub fn write_basic_block<'cx, 'gcx, 'tcx, F>(
    tcx: TyCtxt<'cx, 'gcx, 'tcx>,
    block: BasicBlock,
    mir: &Mir<'tcx>,
    extra_data: &mut F,
    w: &mut dyn Write,
) -> io::Result<()>
where
    F: FnMut(PassWhere, &mut dyn Write) -> io::Result<()>,
{
    let data = &mir[block];

    // Basic block label at the top.
    let cleanup_text = if data.is_cleanup { " // cleanup" } else { "" };
    let lbl = format!("{}{:?}: {{", INDENT, block);
    writeln!(w, "{0:1$}{2}", lbl, ALIGN, cleanup_text)?;

    // List of statements in the middle.
    let mut current_location = Location {
        block: block,
        statement_index: 0,
    };
    for statement in &data.statements {
        extra_data(PassWhere::BeforeLocation(current_location), w)?;
        let indented_mir = format!("{0}{0}{1:?};", INDENT, statement);
        writeln!(
            w,
            "{:A$} // {:?}: {}",
            indented_mir,
            current_location,
            comment(tcx, statement.source_info),
            A = ALIGN,
        )?;

        write_extra(tcx, w, |visitor| {
            visitor.visit_statement(current_location.block, statement, current_location);
        })?;

        extra_data(PassWhere::AfterLocation(current_location), w)?;

        current_location.statement_index += 1;
    }

    // Terminator at the bottom.
    extra_data(PassWhere::BeforeLocation(current_location), w)?;
    let indented_terminator = format!("{0}{0}{1:?};", INDENT, data.terminator().kind);
    writeln!(
        w,
        "{:A$} // {:?}: {}",
        indented_terminator,
        current_location,
        comment(tcx, data.terminator().source_info),
        A = ALIGN,
    )?;

    write_extra(tcx, w, |visitor| {
        visitor.visit_terminator(current_location.block, data.terminator(), current_location);
    })?;

    extra_data(PassWhere::AfterLocation(current_location), w)?;

    writeln!(w, "{}}}", INDENT)
}

/// After we print the main statement, we sometimes dump extra
/// information. There's often a lot of little things "nuzzled up" in
/// a statement.
fn write_extra<'cx, 'gcx, 'tcx, F>(
    tcx: TyCtxt<'cx, 'gcx, 'tcx>,
    write: &mut dyn Write,
    mut visit_op: F,
) -> io::Result<()>
where
    F: FnMut(&mut ExtraComments<'cx, 'gcx, 'tcx>),
{
    let mut extra_comments = ExtraComments {
        _tcx: tcx,
        comments: vec![],
    };
    visit_op(&mut extra_comments);
    for comment in extra_comments.comments {
        writeln!(write, "{:A$} // {}", "", comment, A = ALIGN)?;
    }
    Ok(())
}

struct ExtraComments<'cx, 'gcx: 'tcx, 'tcx: 'cx> {
    _tcx: TyCtxt<'cx, 'gcx, 'tcx>, // don't need it now, but bet we will soon
    comments: Vec<String>,
}

impl<'cx, 'gcx, 'tcx> ExtraComments<'cx, 'gcx, 'tcx> {
    fn push(&mut self, lines: &str) {
        for line in lines.split("\n") {
            self.comments.push(line.to_string());
        }
    }
}

impl<'cx, 'gcx, 'tcx> Visitor<'tcx> for ExtraComments<'cx, 'gcx, 'tcx> {
    fn visit_constant(&mut self, constant: &Constant<'tcx>, location: Location) {
        self.super_constant(constant, location);
        let Constant { span, ty, literal } = constant;
        self.push(&format!("mir::Constant"));
        self.push(&format!("+ span: {:?}", span));
        self.push(&format!("+ ty: {:?}", ty));
        self.push(&format!("+ literal: {:?}", literal));
    }

    fn visit_const(&mut self, constant: &&'tcx ty::Const<'tcx>, _: Location) {
        self.super_const(constant);
        let ty::Const { ty, val, .. } = constant;
        self.push(&format!("ty::Const"));
        self.push(&format!("+ ty: {:?}", ty));
        self.push(&format!("+ val: {:?}", val));
    }

    fn visit_rvalue(&mut self, rvalue: &Rvalue<'tcx>, location: Location) {
        self.super_rvalue(rvalue, location);
        match rvalue {
            Rvalue::Aggregate(kind, _) => match **kind {
                AggregateKind::Closure(def_id, substs) => {
                    self.push(&format!("closure"));
                    self.push(&format!("+ def_id: {:?}", def_id));
                    self.push(&format!("+ substs: {:#?}", substs));
                }

                AggregateKind::Generator(def_id, substs, movability) => {
                    self.push(&format!("generator"));
                    self.push(&format!("+ def_id: {:?}", def_id));
                    self.push(&format!("+ substs: {:#?}", substs));
                    self.push(&format!("+ movability: {:?}", movability));
                }

                _ => {}
            },

            _ => {}
        }
    }
}

fn comment(tcx: TyCtxt, SourceInfo { span, scope }: SourceInfo) -> String {
    format!(
        "scope {} at {}",
        scope.index(),
        tcx.sess.codemap().span_to_string(span)
    )
}

/// Prints user-defined variables in a scope tree.
///
/// Returns the total number of variables printed.
fn write_scope_tree(
    tcx: TyCtxt,
    mir: &Mir,
    scope_tree: &FxHashMap<SourceScope, Vec<SourceScope>>,
    w: &mut dyn Write,
    parent: SourceScope,
    depth: usize,
) -> io::Result<()> {
    let indent = depth * INDENT.len();

    let children = match scope_tree.get(&parent) {
        Some(childs) => childs,
        None => return Ok(()),
    };

    for &child in children {
        let data = &mir.source_scopes[child];
        assert_eq!(data.parent_scope, Some(parent));
        writeln!(w, "{0:1$}scope {2} {{", "", indent, child.index())?;

        // User variable types (including the user's name in a comment).
        for local in mir.vars_iter() {
            let var = &mir.local_decls[local];
            let (name, source_info) = if var.source_info.scope == child {
                (var.name.unwrap(), var.source_info)
            } else {
                // Not a variable or not declared in this scope.
                continue;
            };

            let mut_str = if var.mutability == Mutability::Mut {
                "mut "
            } else {
                ""
            };

            let indent = indent + INDENT.len();
            let indented_var = format!(
                "{0:1$}let {2}{3:?}: {4:?};",
                INDENT,
                indent,
                mut_str,
                local,
                var.ty
            );
            writeln!(
                w,
                "{0:1$} // \"{2}\" in {3}",
                indented_var,
                ALIGN,
                name,
                comment(tcx, source_info)
            )?;
        }

        write_scope_tree(tcx, mir, scope_tree, w, child, depth + 1)?;

        writeln!(w, "{0:1$}}}", "", depth * INDENT.len())?;
    }

    Ok(())
}

/// Write out a human-readable textual representation of the MIR's `fn` type and the types of its
/// local variables (both user-defined bindings and compiler temporaries).
pub fn write_mir_intro<'a, 'gcx, 'tcx>(
    tcx: TyCtxt<'a, 'gcx, 'tcx>,
    src: MirSource,
    mir: &Mir,
    w: &mut dyn Write,
) -> io::Result<()> {
    write_mir_sig(tcx, src, mir, w)?;
    writeln!(w, "{{")?;

    // construct a scope tree and write it out
    let mut scope_tree: FxHashMap<SourceScope, Vec<SourceScope>> = FxHashMap();
    for (index, scope_data) in mir.source_scopes.iter().enumerate() {
        if let Some(parent) = scope_data.parent_scope {
            scope_tree
                .entry(parent)
                .or_insert(vec![])
                .push(SourceScope::new(index));
        } else {
            // Only the argument scope has no parent, because it's the root.
            assert_eq!(index, OUTERMOST_SOURCE_SCOPE.index());
        }
    }

    // Print return place
    let indented_retptr = format!("{}let mut {:?}: {};",
                                  INDENT,
                                  RETURN_PLACE,
                                  mir.local_decls[RETURN_PLACE].ty);
    writeln!(w, "{0:1$} // return place",
             indented_retptr,
             ALIGN)?;

    write_scope_tree(tcx, mir, &scope_tree, w, OUTERMOST_SOURCE_SCOPE, 1)?;

    write_temp_decls(mir, w)?;

    // Add an empty line before the first block is printed.
    writeln!(w, "")?;

    Ok(())
}

fn write_mir_sig(tcx: TyCtxt, src: MirSource, mir: &Mir, w: &mut dyn Write) -> io::Result<()> {
    let id = tcx.hir.as_local_node_id(src.def_id).unwrap();
    let body_owner_kind = tcx.hir.body_owner_kind(id);
    match (body_owner_kind, src.promoted) {
        (_, Some(i)) => write!(w, "{:?} in", i)?,
        (hir::BodyOwnerKind::Fn, _) => write!(w, "fn")?,
        (hir::BodyOwnerKind::Const, _) => write!(w, "const")?,
        (hir::BodyOwnerKind::Static(hir::MutImmutable), _) => write!(w, "static")?,
        (hir::BodyOwnerKind::Static(hir::MutMutable), _) => write!(w, "static mut")?,
    }

    item_path::with_forced_impl_filename_line(|| {
        // see notes on #41697 elsewhere
        write!(w, " {}", tcx.item_path_str(src.def_id))
    })?;

    match (body_owner_kind, src.promoted) {
        (hir::BodyOwnerKind::Fn, None) => {
            write!(w, "(")?;

            // fn argument types.
            for (i, arg) in mir.args_iter().enumerate() {
                if i != 0 {
                    write!(w, ", ")?;
                }
                write!(w, "{:?}: {}", Place::Local(arg), mir.local_decls[arg].ty)?;
            }

            write!(w, ") -> {}", mir.return_ty())?;
        }
        (hir::BodyOwnerKind::Const, _) | (hir::BodyOwnerKind::Static(_), _) | (_, Some(_)) => {
            assert_eq!(mir.arg_count, 0);
            write!(w, ": {} =", mir.return_ty())?;
        }
    }

    if let Some(yield_ty) = mir.yield_ty {
        writeln!(w)?;
        writeln!(w, "yields {}", yield_ty)?;
    }

    Ok(())
}

fn write_temp_decls(mir: &Mir, w: &mut dyn Write) -> io::Result<()> {
    // Compiler-introduced temporary types.
    for temp in mir.temps_iter() {
        writeln!(
            w,
            "{}let mut {:?}: {};",
            INDENT,
            temp,
            mir.local_decls[temp].ty
        )?;
    }

    Ok(())
}

pub fn dump_mir_def_ids(tcx: TyCtxt, single: Option<DefId>) -> Vec<DefId> {
    if let Some(i) = single {
        vec![i]
    } else {
        tcx.mir_keys(LOCAL_CRATE).iter().cloned().collect()
    }
}
