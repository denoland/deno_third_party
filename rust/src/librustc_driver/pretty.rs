//! The various pretty-printing routines.

use rustc::cfg;
use rustc::cfg::graphviz::LabelledCFG;
use rustc::hir;
use rustc::hir::map as hir_map;
use rustc::hir::map::blocks;
use rustc::hir::print as pprust_hir;
use rustc::session::Session;
use rustc::session::config::{Input, OutputFilenames};
use rustc::ty::{self, TyCtxt, Resolutions, AllArenas};
use rustc_borrowck as borrowck;
use rustc_borrowck::graphviz as borrowck_dot;
use rustc_data_structures::thin_vec::ThinVec;
use rustc_metadata::cstore::CStore;
use rustc_mir::util::{write_mir_pretty, write_mir_graphviz};

use syntax::ast::{self, BlockCheckMode};
use syntax::fold::{self, Folder};
use syntax::print::{pprust};
use syntax::print::pprust::PrintState;
use syntax::ptr::P;
use syntax_pos::{self, FileName};

use graphviz as dot;
use smallvec::SmallVec;

use std::cell::Cell;
use std::fs::File;
use std::io::{self, Write};
use std::option;
use std::path::Path;
use std::str::FromStr;
use std::mem;

pub use self::UserIdentifiedItem::*;
pub use self::PpSourceMode::*;
pub use self::PpMode::*;
use self::NodesMatchingUII::*;
use {abort_on_err, driver};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum PpSourceMode {
    PpmNormal,
    PpmEveryBodyLoops,
    PpmExpanded,
    PpmIdentified,
    PpmExpandedIdentified,
    PpmExpandedHygiene,
    PpmTyped,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum PpFlowGraphMode {
    Default,
    /// Drops the labels from the edges in the flowgraph output. This
    /// is mostly for use in the -Z unpretty flowgraph run-make tests,
    /// since the labels are largely uninteresting in those cases and
    /// have become a pain to maintain.
    UnlabelledEdges,
}
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum PpMode {
    PpmSource(PpSourceMode),
    PpmHir(PpSourceMode),
    PpmHirTree(PpSourceMode),
    PpmFlowGraph(PpFlowGraphMode),
    PpmMir,
    PpmMirCFG,
}

impl PpMode {
    pub fn needs_ast_map(&self, opt_uii: &Option<UserIdentifiedItem>) -> bool {
        match *self {
            PpmSource(PpmNormal) |
            PpmSource(PpmEveryBodyLoops) |
            PpmSource(PpmIdentified) => opt_uii.is_some(),

            PpmSource(PpmExpanded) |
            PpmSource(PpmExpandedIdentified) |
            PpmSource(PpmExpandedHygiene) |
            PpmHir(_) |
            PpmHirTree(_) |
            PpmMir |
            PpmMirCFG |
            PpmFlowGraph(_) => true,
            PpmSource(PpmTyped) => panic!("invalid state"),
        }
    }

    pub fn needs_analysis(&self) -> bool {
        match *self {
            PpmMir | PpmMirCFG | PpmFlowGraph(_) => true,
            _ => false,
        }
    }
}

pub fn parse_pretty(sess: &Session,
                    name: &str,
                    extended: bool)
                    -> (PpMode, Option<UserIdentifiedItem>) {
    let mut split = name.splitn(2, '=');
    let first = split.next().unwrap();
    let opt_second = split.next();
    let first = match (first, extended) {
        ("normal", _) => PpmSource(PpmNormal),
        ("identified", _) => PpmSource(PpmIdentified),
        ("everybody_loops", true) => PpmSource(PpmEveryBodyLoops),
        ("expanded", _) => PpmSource(PpmExpanded),
        ("expanded,identified", _) => PpmSource(PpmExpandedIdentified),
        ("expanded,hygiene", _) => PpmSource(PpmExpandedHygiene),
        ("hir", true) => PpmHir(PpmNormal),
        ("hir,identified", true) => PpmHir(PpmIdentified),
        ("hir,typed", true) => PpmHir(PpmTyped),
        ("hir-tree", true) => PpmHirTree(PpmNormal),
        ("mir", true) => PpmMir,
        ("mir-cfg", true) => PpmMirCFG,
        ("flowgraph", true) => PpmFlowGraph(PpFlowGraphMode::Default),
        ("flowgraph,unlabelled", true) => PpmFlowGraph(PpFlowGraphMode::UnlabelledEdges),
        _ => {
            if extended {
                sess.fatal(&format!("argument to `unpretty` must be one of `normal`, \
                                     `expanded`, `flowgraph[,unlabelled]=<nodeid>`, \
                                     `identified`, `expanded,identified`, `everybody_loops`, \
                                     `hir`, `hir,identified`, `hir,typed`, or `mir`; got {}",
                                    name));
            } else {
                sess.fatal(&format!("argument to `pretty` must be one of `normal`, `expanded`, \
                                     `identified`, or `expanded,identified`; got {}",
                                    name));
            }
        }
    };
    let opt_second = opt_second.and_then(|s| s.parse::<UserIdentifiedItem>().ok());
    (first, opt_second)
}



// This slightly awkward construction is to allow for each PpMode to
// choose whether it needs to do analyses (which can consume the
// Session) and then pass through the session (now attached to the
// analysis results) on to the chosen pretty-printer, along with the
// `&PpAnn` object.
//
// Note that since the `&PrinterSupport` is freshly constructed on each
// call, it would not make sense to try to attach the lifetime of `self`
// to the lifetime of the `&PrinterObject`.
//
// (The `use_once_payload` is working around the current lack of once
// functions in the compiler.)

impl PpSourceMode {
    /// Constructs a `PrinterSupport` object and passes it to `f`.
    fn call_with_pp_support<'tcx, A, F>(&self,
                                        sess: &'tcx Session,
                                        hir_map: Option<&hir_map::Map<'tcx>>,
                                        f: F)
                                        -> A
        where F: FnOnce(&dyn PrinterSupport) -> A
    {
        match *self {
            PpmNormal | PpmEveryBodyLoops | PpmExpanded => {
                let annotation = NoAnn {
                    sess,
                    hir_map: hir_map.map(|m| m.clone()),
                };
                f(&annotation)
            }

            PpmIdentified | PpmExpandedIdentified => {
                let annotation = IdentifiedAnnotation {
                    sess,
                    hir_map: hir_map.map(|m| m.clone()),
                };
                f(&annotation)
            }
            PpmExpandedHygiene => {
                let annotation = HygieneAnnotation {
                    sess,
                };
                f(&annotation)
            }
            _ => panic!("Should use call_with_pp_support_hir"),
        }
    }
    fn call_with_pp_support_hir<'tcx, A, F>(
        &self,
        sess: &'tcx Session,
        cstore: &'tcx CStore,
        hir_map: &hir_map::Map<'tcx>,
        analysis: &ty::CrateAnalysis,
        resolutions: &Resolutions,
        output_filenames: &OutputFilenames,
        id: &str,
        f: F
    ) -> A
        where F: FnOnce(&dyn HirPrinterSupport, &hir::Crate) -> A
    {
        match *self {
            PpmNormal => {
                let annotation = NoAnn {
                    sess,
                    hir_map: Some(hir_map.clone()),
                };
                f(&annotation, hir_map.forest.krate())
            }

            PpmIdentified => {
                let annotation = IdentifiedAnnotation {
                    sess,
                    hir_map: Some(hir_map.clone()),
                };
                f(&annotation, hir_map.forest.krate())
            }
            PpmTyped => {
                let control = &driver::CompileController::basic();
                let codegen_backend = ::get_codegen_backend(sess);
                let mut arenas = AllArenas::new();
                abort_on_err(driver::phase_3_run_analysis_passes(&*codegen_backend,
                                                                 control,
                                                                 sess,
                                                                 cstore,
                                                                 hir_map.clone(),
                                                                 analysis.clone(),
                                                                 resolutions.clone(),
                                                                 &mut arenas,
                                                                 id,
                                                                 output_filenames,
                                                                 |tcx, _, _, _| {
                    let empty_tables = ty::TypeckTables::empty(None);
                    let annotation = TypedAnnotation {
                        tcx,
                        tables: Cell::new(&empty_tables)
                    };
                    tcx.dep_graph.with_ignore(|| {
                        f(&annotation, hir_map.forest.krate())
                    })
                }),
                             sess)
            }
            _ => panic!("Should use call_with_pp_support"),
        }
    }
}

trait PrinterSupport: pprust::PpAnn {
    /// Provides a uniform interface for re-extracting a reference to a
    /// `Session` from a value that now owns it.
    fn sess<'a>(&'a self) -> &'a Session;

    /// Produces the pretty-print annotation object.
    ///
    /// (Rust does not yet support upcasting from a trait object to
    /// an object for one of its super-traits.)
    fn pp_ann<'a>(&'a self) -> &'a dyn pprust::PpAnn;
}

trait HirPrinterSupport<'hir>: pprust_hir::PpAnn {
    /// Provides a uniform interface for re-extracting a reference to a
    /// `Session` from a value that now owns it.
    fn sess<'a>(&'a self) -> &'a Session;

    /// Provides a uniform interface for re-extracting a reference to an
    /// `hir_map::Map` from a value that now owns it.
    fn hir_map<'a>(&'a self) -> Option<&'a hir_map::Map<'hir>>;

    /// Produces the pretty-print annotation object.
    ///
    /// (Rust does not yet support upcasting from a trait object to
    /// an object for one of its super-traits.)
    fn pp_ann<'a>(&'a self) -> &'a dyn pprust_hir::PpAnn;

    /// Computes an user-readable representation of a path, if possible.
    fn node_path(&self, id: ast::NodeId) -> Option<String> {
        self.hir_map().and_then(|map| map.def_path_from_id(id)).map(|path| {
            path.data
                .into_iter()
                .map(|elem| elem.data.to_string())
                .collect::<Vec<_>>()
                .join("::")
        })
    }
}

struct NoAnn<'hir> {
    sess: &'hir Session,
    hir_map: Option<hir_map::Map<'hir>>,
}

impl<'hir> PrinterSupport for NoAnn<'hir> {
    fn sess<'a>(&'a self) -> &'a Session {
        self.sess
    }

    fn pp_ann<'a>(&'a self) -> &'a dyn pprust::PpAnn {
        self
    }
}

impl<'hir> HirPrinterSupport<'hir> for NoAnn<'hir> {
    fn sess<'a>(&'a self) -> &'a Session {
        self.sess
    }

    fn hir_map<'a>(&'a self) -> Option<&'a hir_map::Map<'hir>> {
        self.hir_map.as_ref()
    }

    fn pp_ann<'a>(&'a self) -> &'a dyn pprust_hir::PpAnn {
        self
    }
}

impl<'hir> pprust::PpAnn for NoAnn<'hir> {}
impl<'hir> pprust_hir::PpAnn for NoAnn<'hir> {
    fn nested(&self, state: &mut pprust_hir::State, nested: pprust_hir::Nested)
              -> io::Result<()> {
        if let Some(ref map) = self.hir_map {
            pprust_hir::PpAnn::nested(map, state, nested)
        } else {
            Ok(())
        }
    }
}

struct IdentifiedAnnotation<'hir> {
    sess: &'hir Session,
    hir_map: Option<hir_map::Map<'hir>>,
}

impl<'hir> PrinterSupport for IdentifiedAnnotation<'hir> {
    fn sess<'a>(&'a self) -> &'a Session {
        self.sess
    }

    fn pp_ann<'a>(&'a self) -> &'a dyn pprust::PpAnn {
        self
    }
}

impl<'hir> pprust::PpAnn for IdentifiedAnnotation<'hir> {
    fn pre(&self, s: &mut pprust::State, node: pprust::AnnNode) -> io::Result<()> {
        match node {
            pprust::AnnNode::Expr(_) => s.popen(),
            _ => Ok(()),
        }
    }
    fn post(&self, s: &mut pprust::State, node: pprust::AnnNode) -> io::Result<()> {
        match node {
            pprust::AnnNode::Ident(_) |
            pprust::AnnNode::Name(_) => Ok(()),

            pprust::AnnNode::Item(item) => {
                s.s.space()?;
                s.synth_comment(item.id.to_string())
            }
            pprust::AnnNode::SubItem(id) => {
                s.s.space()?;
                s.synth_comment(id.to_string())
            }
            pprust::AnnNode::Block(blk) => {
                s.s.space()?;
                s.synth_comment(format!("block {}", blk.id))
            }
            pprust::AnnNode::Expr(expr) => {
                s.s.space()?;
                s.synth_comment(expr.id.to_string())?;
                s.pclose()
            }
            pprust::AnnNode::Pat(pat) => {
                s.s.space()?;
                s.synth_comment(format!("pat {}", pat.id))
            }
        }
    }
}

impl<'hir> HirPrinterSupport<'hir> for IdentifiedAnnotation<'hir> {
    fn sess<'a>(&'a self) -> &'a Session {
        self.sess
    }

    fn hir_map<'a>(&'a self) -> Option<&'a hir_map::Map<'hir>> {
        self.hir_map.as_ref()
    }

    fn pp_ann<'a>(&'a self) -> &'a dyn pprust_hir::PpAnn {
        self
    }
}

impl<'hir> pprust_hir::PpAnn for IdentifiedAnnotation<'hir> {
    fn nested(&self, state: &mut pprust_hir::State, nested: pprust_hir::Nested)
              -> io::Result<()> {
        if let Some(ref map) = self.hir_map {
            pprust_hir::PpAnn::nested(map, state, nested)
        } else {
            Ok(())
        }
    }
    fn pre(&self, s: &mut pprust_hir::State, node: pprust_hir::AnnNode) -> io::Result<()> {
        match node {
            pprust_hir::AnnNode::Expr(_) => s.popen(),
            _ => Ok(()),
        }
    }
    fn post(&self, s: &mut pprust_hir::State, node: pprust_hir::AnnNode) -> io::Result<()> {
        match node {
            pprust_hir::AnnNode::Name(_) => Ok(()),
            pprust_hir::AnnNode::Item(item) => {
                s.s.space()?;
                s.synth_comment(format!("node_id: {} hir local_id: {}",
                                        item.id, item.hir_id.local_id.as_u32()))
            }
            pprust_hir::AnnNode::SubItem(id) => {
                s.s.space()?;
                s.synth_comment(id.to_string())
            }
            pprust_hir::AnnNode::Block(blk) => {
                s.s.space()?;
                s.synth_comment(format!("block node_id: {} hir local_id: {}",
                                        blk.id, blk.hir_id.local_id.as_u32()))
            }
            pprust_hir::AnnNode::Expr(expr) => {
                s.s.space()?;
                s.synth_comment(format!("node_id: {} hir local_id: {}",
                                        expr.id, expr.hir_id.local_id.as_u32()))?;
                s.pclose()
            }
            pprust_hir::AnnNode::Pat(pat) => {
                s.s.space()?;
                s.synth_comment(format!("pat node_id: {} hir local_id: {}",
                                        pat.id, pat.hir_id.local_id.as_u32()))
            }
        }
    }
}

struct HygieneAnnotation<'a> {
    sess: &'a Session
}

impl<'a> PrinterSupport for HygieneAnnotation<'a> {
    fn sess(&self) -> &Session {
        self.sess
    }

    fn pp_ann(&self) -> &dyn pprust::PpAnn {
        self
    }
}

impl<'a> pprust::PpAnn for HygieneAnnotation<'a> {
    fn post(&self, s: &mut pprust::State, node: pprust::AnnNode) -> io::Result<()> {
        match node {
            pprust::AnnNode::Ident(&ast::Ident { name, span }) => {
                s.s.space()?;
                // FIXME #16420: this doesn't display the connections
                // between syntax contexts
                s.synth_comment(format!("{}{:?}", name.as_u32(), span.ctxt()))
            }
            pprust::AnnNode::Name(&name) => {
                s.s.space()?;
                s.synth_comment(name.as_u32().to_string())
            }
            _ => Ok(()),
        }
    }
}


struct TypedAnnotation<'a, 'tcx: 'a> {
    tcx: TyCtxt<'a, 'tcx, 'tcx>,
    tables: Cell<&'a ty::TypeckTables<'tcx>>,
}

impl<'b, 'tcx> HirPrinterSupport<'tcx> for TypedAnnotation<'b, 'tcx> {
    fn sess<'a>(&'a self) -> &'a Session {
        &self.tcx.sess
    }

    fn hir_map<'a>(&'a self) -> Option<&'a hir_map::Map<'tcx>> {
        Some(&self.tcx.hir())
    }

    fn pp_ann<'a>(&'a self) -> &'a dyn pprust_hir::PpAnn {
        self
    }

    fn node_path(&self, id: ast::NodeId) -> Option<String> {
        Some(self.tcx.node_path_str(id))
    }
}

impl<'a, 'tcx> pprust_hir::PpAnn for TypedAnnotation<'a, 'tcx> {
    fn nested(&self, state: &mut pprust_hir::State, nested: pprust_hir::Nested)
              -> io::Result<()> {
        let old_tables = self.tables.get();
        if let pprust_hir::Nested::Body(id) = nested {
            self.tables.set(self.tcx.body_tables(id));
        }
        pprust_hir::PpAnn::nested(self.tcx.hir(), state, nested)?;
        self.tables.set(old_tables);
        Ok(())
    }
    fn pre(&self, s: &mut pprust_hir::State, node: pprust_hir::AnnNode) -> io::Result<()> {
        match node {
            pprust_hir::AnnNode::Expr(_) => s.popen(),
            _ => Ok(()),
        }
    }
    fn post(&self, s: &mut pprust_hir::State, node: pprust_hir::AnnNode) -> io::Result<()> {
        match node {
            pprust_hir::AnnNode::Expr(expr) => {
                s.s.space()?;
                s.s.word("as")?;
                s.s.space()?;
                s.s.word(self.tables.get().expr_ty(expr).to_string())?;
                s.pclose()
            }
            _ => Ok(()),
        }
    }
}

fn gather_flowgraph_variants(sess: &Session) -> Vec<borrowck_dot::Variant> {
    let print_loans = sess.opts.debugging_opts.flowgraph_print_loans;
    let print_moves = sess.opts.debugging_opts.flowgraph_print_moves;
    let print_assigns = sess.opts.debugging_opts.flowgraph_print_assigns;
    let print_all = sess.opts.debugging_opts.flowgraph_print_all;
    let mut variants = Vec::new();
    if print_all || print_loans {
        variants.push(borrowck_dot::Loans);
    }
    if print_all || print_moves {
        variants.push(borrowck_dot::Moves);
    }
    if print_all || print_assigns {
        variants.push(borrowck_dot::Assigns);
    }
    variants
}

#[derive(Clone, Debug)]
pub enum UserIdentifiedItem {
    ItemViaNode(ast::NodeId),
    ItemViaPath(Vec<String>),
}

impl FromStr for UserIdentifiedItem {
    type Err = ();
    fn from_str(s: &str) -> Result<UserIdentifiedItem, ()> {
        Ok(s.parse()
            .map(ast::NodeId::from_u32)
            .map(ItemViaNode)
            .unwrap_or_else(|_| ItemViaPath(s.split("::").map(|s| s.to_string()).collect())))
    }
}

enum NodesMatchingUII<'a, 'hir: 'a> {
    NodesMatchingDirect(option::IntoIter<ast::NodeId>),
    NodesMatchingSuffix(hir_map::NodesMatchingSuffix<'a, 'hir>),
}

impl<'a, 'hir> Iterator for NodesMatchingUII<'a, 'hir> {
    type Item = ast::NodeId;

    fn next(&mut self) -> Option<ast::NodeId> {
        match self {
            &mut NodesMatchingDirect(ref mut iter) => iter.next(),
            &mut NodesMatchingSuffix(ref mut iter) => iter.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            &NodesMatchingDirect(ref iter) => iter.size_hint(),
            &NodesMatchingSuffix(ref iter) => iter.size_hint(),
        }
    }
}

impl UserIdentifiedItem {
    fn reconstructed_input(&self) -> String {
        match *self {
            ItemViaNode(node_id) => node_id.to_string(),
            ItemViaPath(ref parts) => parts.join("::"),
        }
    }

    fn all_matching_node_ids<'a, 'hir>(&'a self,
                                       map: &'a hir_map::Map<'hir>)
                                       -> NodesMatchingUII<'a, 'hir> {
        match *self {
            ItemViaNode(node_id) => NodesMatchingDirect(Some(node_id).into_iter()),
            ItemViaPath(ref parts) => NodesMatchingSuffix(map.nodes_matching_suffix(&parts)),
        }
    }

    fn to_one_node_id(self, user_option: &str, sess: &Session, map: &hir_map::Map) -> ast::NodeId {
        let fail_because = |is_wrong_because| -> ast::NodeId {
            let message = format!("{} needs NodeId (int) or unique path suffix (b::c::d); got \
                                   {}, which {}",
                                  user_option,
                                  self.reconstructed_input(),
                                  is_wrong_because);
            sess.fatal(&message)
        };

        let mut saw_node = ast::DUMMY_NODE_ID;
        let mut seen = 0;
        for node in self.all_matching_node_ids(map) {
            saw_node = node;
            seen += 1;
            if seen > 1 {
                fail_because("does not resolve uniquely");
            }
        }
        if seen == 0 {
            fail_because("does not resolve to any item");
        }

        assert!(seen == 1);
        return saw_node;
    }
}

// Note: Also used by librustdoc, see PR #43348. Consider moving this struct elsewhere.
//
// FIXME: Currently the `everybody_loops` transformation is not applied to:
//  * `const fn`, due to issue #43636 that `loop` is not supported for const evaluation. We are
//    waiting for miri to fix that.
//  * `impl Trait`, due to issue #43869 that functions returning impl Trait cannot be diverging.
//    Solving this may require `!` to implement every trait, which relies on the an even more
//    ambitious form of the closed RFC #1637. See also [#34511].
//
// [#34511]: https://github.com/rust-lang/rust/issues/34511#issuecomment-322340401
pub struct ReplaceBodyWithLoop<'a> {
    within_static_or_const: bool,
    nested_blocks: Option<Vec<ast::Block>>,
    sess: &'a Session,
}

impl<'a> ReplaceBodyWithLoop<'a> {
    pub fn new(sess: &'a Session) -> ReplaceBodyWithLoop<'a> {
        ReplaceBodyWithLoop {
            within_static_or_const: false,
            nested_blocks: None,
            sess
        }
    }

    fn run<R, F: FnOnce(&mut Self) -> R>(&mut self, is_const: bool, action: F) -> R {
        let old_const = mem::replace(&mut self.within_static_or_const, is_const);
        let old_blocks = self.nested_blocks.take();
        let ret = action(self);
        self.within_static_or_const = old_const;
        self.nested_blocks = old_blocks;
        ret
    }

    fn should_ignore_fn(ret_ty: &ast::FnDecl) -> bool {
        if let ast::FunctionRetTy::Ty(ref ty) = ret_ty.output {
            fn involves_impl_trait(ty: &ast::Ty) -> bool {
                match ty.node {
                    ast::TyKind::ImplTrait(..) => true,
                    ast::TyKind::Slice(ref subty) |
                    ast::TyKind::Array(ref subty, _) |
                    ast::TyKind::Ptr(ast::MutTy { ty: ref subty, .. }) |
                    ast::TyKind::Rptr(_, ast::MutTy { ty: ref subty, .. }) |
                    ast::TyKind::Paren(ref subty) => involves_impl_trait(subty),
                    ast::TyKind::Tup(ref tys) => any_involves_impl_trait(tys.iter()),
                    ast::TyKind::Path(_, ref path) => path.segments.iter().any(|seg| {
                        match seg.args.as_ref().map(|generic_arg| &**generic_arg) {
                            None => false,
                            Some(&ast::GenericArgs::AngleBracketed(ref data)) => {
                                let types = data.args.iter().filter_map(|arg| match arg {
                                    ast::GenericArg::Type(ty) => Some(ty),
                                    _ => None,
                                });
                                any_involves_impl_trait(types.into_iter()) ||
                                any_involves_impl_trait(data.bindings.iter().map(|b| &b.ty))
                            },
                            Some(&ast::GenericArgs::Parenthesized(ref data)) => {
                                any_involves_impl_trait(data.inputs.iter()) ||
                                any_involves_impl_trait(data.output.iter())
                            }
                        }
                    }),
                    _ => false,
                }
            }

            fn any_involves_impl_trait<'a, I: Iterator<Item = &'a P<ast::Ty>>>(mut it: I) -> bool {
                it.any(|subty| involves_impl_trait(subty))
            }

            involves_impl_trait(ty)
        } else {
            false
        }
    }
}

impl<'a> fold::Folder for ReplaceBodyWithLoop<'a> {
    fn fold_item_kind(&mut self, i: ast::ItemKind) -> ast::ItemKind {
        let is_const = match i {
            ast::ItemKind::Static(..) | ast::ItemKind::Const(..) => true,
            ast::ItemKind::Fn(ref decl, ref header, _, _) =>
                header.constness.node == ast::Constness::Const || Self::should_ignore_fn(decl),
            _ => false,
        };
        self.run(is_const, |s| fold::noop_fold_item_kind(i, s))
    }

    fn fold_trait_item(&mut self, i: ast::TraitItem) -> SmallVec<[ast::TraitItem; 1]> {
        let is_const = match i.node {
            ast::TraitItemKind::Const(..) => true,
            ast::TraitItemKind::Method(ast::MethodSig { ref decl, ref header, .. }, _) =>
                header.constness.node == ast::Constness::Const || Self::should_ignore_fn(decl),
            _ => false,
        };
        self.run(is_const, |s| fold::noop_fold_trait_item(i, s))
    }

    fn fold_impl_item(&mut self, i: ast::ImplItem) -> SmallVec<[ast::ImplItem; 1]> {
        let is_const = match i.node {
            ast::ImplItemKind::Const(..) => true,
            ast::ImplItemKind::Method(ast::MethodSig { ref decl, ref header, .. }, _) =>
                header.constness.node == ast::Constness::Const || Self::should_ignore_fn(decl),
            _ => false,
        };
        self.run(is_const, |s| fold::noop_fold_impl_item(i, s))
    }

    fn fold_anon_const(&mut self, c: ast::AnonConst) -> ast::AnonConst {
        self.run(true, |s| fold::noop_fold_anon_const(c, s))
    }

    fn fold_block(&mut self, b: P<ast::Block>) -> P<ast::Block> {
        fn stmt_to_block(rules: ast::BlockCheckMode,
                         s: Option<ast::Stmt>,
                         sess: &Session) -> ast::Block {
            ast::Block {
                stmts: s.into_iter().collect(),
                rules,
                id: sess.next_node_id(),
                span: syntax_pos::DUMMY_SP,
            }
        }

        fn block_to_stmt(b: ast::Block, sess: &Session) -> ast::Stmt {
            let expr = P(ast::Expr {
                id: sess.next_node_id(),
                node: ast::ExprKind::Block(P(b), None),
                span: syntax_pos::DUMMY_SP,
                attrs: ThinVec::new(),
            });

            ast::Stmt {
                id: sess.next_node_id(),
                node: ast::StmtKind::Expr(expr),
                span: syntax_pos::DUMMY_SP,
            }
        }

        let empty_block = stmt_to_block(BlockCheckMode::Default, None, self.sess);
        let loop_expr = P(ast::Expr {
            node: ast::ExprKind::Loop(P(empty_block), None),
            id: self.sess.next_node_id(),
            span: syntax_pos::DUMMY_SP,
                attrs: ThinVec::new(),
        });

        let loop_stmt = ast::Stmt {
            id: self.sess.next_node_id(),
            span: syntax_pos::DUMMY_SP,
            node: ast::StmtKind::Expr(loop_expr),
        };

        if self.within_static_or_const {
            fold::noop_fold_block(b, self)
        } else {
            b.map(|b| {
                let mut stmts = vec![];
                for s in b.stmts {
                    let old_blocks = self.nested_blocks.replace(vec![]);

                    stmts.extend(self.fold_stmt(s).into_iter().filter(|s| s.is_item()));

                    // we put a Some in there earlier with that replace(), so this is valid
                    let new_blocks = self.nested_blocks.take().unwrap();
                    self.nested_blocks = old_blocks;
                    stmts.extend(new_blocks.into_iter().map(|b| block_to_stmt(b, &self.sess)));
                }

                let mut new_block = ast::Block {
                    stmts,
                    ..b
                };

                if let Some(old_blocks) = self.nested_blocks.as_mut() {
                    //push our fresh block onto the cache and yield an empty block with `loop {}`
                    if !new_block.stmts.is_empty() {
                        old_blocks.push(new_block);
                    }

                    stmt_to_block(b.rules, Some(loop_stmt), self.sess)
                } else {
                    //push `loop {}` onto the end of our fresh block and yield that
                    new_block.stmts.push(loop_stmt);

                    new_block
                }
            })
        }
    }

    // in general the pretty printer processes unexpanded code, so
    // we override the default `fold_mac` method which panics.
    fn fold_mac(&mut self, mac: ast::Mac) -> ast::Mac {
        fold::noop_fold_mac(mac, self)
    }
}

fn print_flowgraph<'a, 'tcx, W: Write>(variants: Vec<borrowck_dot::Variant>,
                                       tcx: TyCtxt<'a, 'tcx, 'tcx>,
                                       code: blocks::Code<'tcx>,
                                       mode: PpFlowGraphMode,
                                       mut out: W)
                                       -> io::Result<()> {
    let body_id = match code {
        blocks::Code::Expr(expr) => {
            // Find the function this expression is from.
            let mut node_id = expr.id;
            loop {
                let node = tcx.hir().get(node_id);
                if let Some(n) = hir::map::blocks::FnLikeNode::from_node(node) {
                    break n.body();
                }
                let parent = tcx.hir().get_parent_node(node_id);
                assert_ne!(node_id, parent);
                node_id = parent;
            }
        }
        blocks::Code::FnLike(fn_like) => fn_like.body(),
    };
    let body = tcx.hir().body(body_id);
    let cfg = cfg::CFG::new(tcx, &body);
    let labelled_edges = mode != PpFlowGraphMode::UnlabelledEdges;
    let lcfg = LabelledCFG {
        tcx,
        cfg: &cfg,
        name: format!("node_{}", code.id()),
        labelled_edges,
    };

    match code {
        _ if variants.is_empty() => {
            let r = dot::render(&lcfg, &mut out);
            return expand_err_details(r);
        }
        blocks::Code::Expr(_) => {
            tcx.sess.err("--pretty flowgraph with -Z flowgraph-print annotations requires \
                          fn-like node id.");
            return Ok(());
        }
        blocks::Code::FnLike(fn_like) => {
            let (bccx, analysis_data) =
                borrowck::build_borrowck_dataflow_data_for_fn(tcx, fn_like.body(), &cfg);

            let lcfg = borrowck_dot::DataflowLabeller {
                inner: lcfg,
                variants,
                borrowck_ctxt: &bccx,
                analysis_data: &analysis_data,
            };
            let r = dot::render(&lcfg, &mut out);
            return expand_err_details(r);
        }
    }

    fn expand_err_details(r: io::Result<()>) -> io::Result<()> {
        r.map_err(|ioerr| {
            io::Error::new(io::ErrorKind::Other,
                           format!("graphviz::render failed: {}", ioerr))
        })
    }
}

pub fn fold_crate(sess: &Session, krate: ast::Crate, ppm: PpMode) -> ast::Crate {
    if let PpmSource(PpmEveryBodyLoops) = ppm {
        let mut fold = ReplaceBodyWithLoop::new(sess);
        fold.fold_crate(krate)
    } else {
        krate
    }
}

fn get_source(input: &Input, sess: &Session) -> (Vec<u8>, FileName) {
    let src_name = driver::source_name(input);
    let src = sess.source_map()
        .get_source_file(&src_name)
        .unwrap()
        .src
        .as_ref()
        .unwrap()
        .as_bytes()
        .to_vec();
    (src, src_name)
}

fn write_output(out: Vec<u8>, ofile: Option<&Path>) {
    match ofile {
        None => print!("{}", String::from_utf8(out).unwrap()),
        Some(p) => {
            match File::create(p) {
                Ok(mut w) => w.write_all(&out).unwrap(),
                Err(e) => panic!("print-print failed to open {} due to {}", p.display(), e),
            }
        }
    }
}

pub fn print_after_parsing(sess: &Session,
                           input: &Input,
                           krate: &ast::Crate,
                           ppm: PpMode,
                           ofile: Option<&Path>) {
    let (src, src_name) = get_source(input, sess);

    let mut rdr = &*src;
    let mut out = Vec::new();

    if let PpmSource(s) = ppm {
        // Silently ignores an identified node.
        let out: &mut dyn Write = &mut out;
        s.call_with_pp_support(sess, None, move |annotation| {
            debug!("pretty printing source code {:?}", s);
            let sess = annotation.sess();
            pprust::print_crate(sess.source_map(),
                                &sess.parse_sess,
                                krate,
                                src_name,
                                &mut rdr,
                                box out,
                                annotation.pp_ann(),
                                false)
        }).unwrap()
    } else {
        unreachable!();
    };

    write_output(out, ofile);
}

pub fn print_after_hir_lowering<'tcx, 'a: 'tcx>(sess: &'a Session,
                                                cstore: &'tcx CStore,
                                                hir_map: &hir_map::Map<'tcx>,
                                                analysis: &ty::CrateAnalysis,
                                                resolutions: &Resolutions,
                                                input: &Input,
                                                krate: &ast::Crate,
                                                crate_name: &str,
                                                ppm: PpMode,
                                                output_filenames: &OutputFilenames,
                                                opt_uii: Option<UserIdentifiedItem>,
                                                ofile: Option<&Path>) {
    if ppm.needs_analysis() {
        print_with_analysis(sess,
                            cstore,
                            hir_map,
                            analysis,
                            resolutions,
                            crate_name,
                            output_filenames,
                            ppm,
                            opt_uii,
                            ofile);
        return;
    }

    let (src, src_name) = get_source(input, sess);

    let mut rdr = &src[..];
    let mut out = Vec::new();

    match (ppm, opt_uii) {
            (PpmSource(s), _) => {
                // Silently ignores an identified node.
                let out: &mut dyn Write = &mut out;
                s.call_with_pp_support(sess, Some(hir_map), move |annotation| {
                    debug!("pretty printing source code {:?}", s);
                    let sess = annotation.sess();
                    pprust::print_crate(sess.source_map(),
                                        &sess.parse_sess,
                                        krate,
                                        src_name,
                                        &mut rdr,
                                        box out,
                                        annotation.pp_ann(),
                                        true)
                })
            }

            (PpmHir(s), None) => {
                let out: &mut dyn Write = &mut out;
                s.call_with_pp_support_hir(sess,
                                           cstore,
                                           hir_map,
                                           analysis,
                                           resolutions,
                                           output_filenames,
                                           crate_name,
                                           move |annotation, krate| {
                    debug!("pretty printing source code {:?}", s);
                    let sess = annotation.sess();
                    pprust_hir::print_crate(sess.source_map(),
                                            &sess.parse_sess,
                                            krate,
                                            src_name,
                                            &mut rdr,
                                            box out,
                                            annotation.pp_ann(),
                                            true)
                })
            }

            (PpmHirTree(s), None) => {
                let out: &mut dyn Write = &mut out;
                s.call_with_pp_support_hir(sess,
                                           cstore,
                                           hir_map,
                                           analysis,
                                           resolutions,
                                           output_filenames,
                                           crate_name,
                                           move |_annotation, krate| {
                    debug!("pretty printing source code {:?}", s);
                    write!(out, "{:#?}", krate)
                })
            }

            (PpmHir(s), Some(uii)) => {
                let out: &mut dyn Write = &mut out;
                s.call_with_pp_support_hir(sess,
                                           cstore,
                                           hir_map,
                                           analysis,
                                           resolutions,
                                           output_filenames,
                                           crate_name,
                                           move |annotation, _| {
                    debug!("pretty printing source code {:?}", s);
                    let sess = annotation.sess();
                    let hir_map = annotation.hir_map().expect("-Z unpretty missing HIR map");
                    let mut pp_state = pprust_hir::State::new_from_input(sess.source_map(),
                                                                         &sess.parse_sess,
                                                                         src_name,
                                                                         &mut rdr,
                                                                         box out,
                                                                         annotation.pp_ann(),
                                                                         true);
                    for node_id in uii.all_matching_node_ids(hir_map) {
                        let node = hir_map.get(node_id);
                        pp_state.print_node(node)?;
                        pp_state.s.space()?;
                        let path = annotation.node_path(node_id)
                            .expect("-Z unpretty missing node paths");
                        pp_state.synth_comment(path)?;
                        pp_state.s.hardbreak()?;
                    }
                    pp_state.s.eof()
                })
            }

            (PpmHirTree(s), Some(uii)) => {
                let out: &mut dyn Write = &mut out;
                s.call_with_pp_support_hir(sess,
                                           cstore,
                                           hir_map,
                                           analysis,
                                           resolutions,
                                           output_filenames,
                                           crate_name,
                                           move |_annotation, _krate| {
                    debug!("pretty printing source code {:?}", s);
                    for node_id in uii.all_matching_node_ids(hir_map) {
                        let node = hir_map.get(node_id);
                        write!(out, "{:#?}", node)?;
                    }
                    Ok(())
                })
            }

            _ => unreachable!(),
        }
        .unwrap();

    write_output(out, ofile);
}

// In an ideal world, this would be a public function called by the driver after
// analsysis is performed. However, we want to call `phase_3_run_analysis_passes`
// with a different callback than the standard driver, so that isn't easy.
// Instead, we call that function ourselves.
fn print_with_analysis<'tcx, 'a: 'tcx>(sess: &'a Session,
                                       cstore: &'a CStore,
                                       hir_map: &hir_map::Map<'tcx>,
                                       analysis: &ty::CrateAnalysis,
                                       resolutions: &Resolutions,
                                       crate_name: &str,
                                       output_filenames: &OutputFilenames,
                                       ppm: PpMode,
                                       uii: Option<UserIdentifiedItem>,
                                       ofile: Option<&Path>) {
    let nodeid = if let Some(uii) = uii {
        debug!("pretty printing for {:?}", uii);
        Some(uii.to_one_node_id("-Z unpretty", sess, &hir_map))
    } else {
        debug!("pretty printing for whole crate");
        None
    };

    let mut out = Vec::new();

    let control = &driver::CompileController::basic();
    let codegen_backend = ::get_codegen_backend(sess);
    let mut arenas = AllArenas::new();
    abort_on_err(driver::phase_3_run_analysis_passes(&*codegen_backend,
                                                     control,
                                                     sess,
                                                     cstore,
                                                     hir_map.clone(),
                                                     analysis.clone(),
                                                     resolutions.clone(),
                                                     &mut arenas,
                                                     crate_name,
                                                     output_filenames,
                                                     |tcx, _, _, _| {
        match ppm {
            PpmMir | PpmMirCFG => {
                if let Some(nodeid) = nodeid {
                    let def_id = tcx.hir().local_def_id(nodeid);
                    match ppm {
                        PpmMir => write_mir_pretty(tcx, Some(def_id), &mut out),
                        PpmMirCFG => write_mir_graphviz(tcx, Some(def_id), &mut out),
                        _ => unreachable!(),
                    }?;
                } else {
                    match ppm {
                        PpmMir => write_mir_pretty(tcx, None, &mut out),
                        PpmMirCFG => write_mir_graphviz(tcx, None, &mut out),
                        _ => unreachable!(),
                    }?;
                }
                Ok(())
            }
            PpmFlowGraph(mode) => {
                let nodeid =
                    nodeid.expect("`pretty flowgraph=..` needs NodeId (int) or unique path \
                                   suffix (b::c::d)");
                let node = tcx.hir().find(nodeid).unwrap_or_else(|| {
                    tcx.sess.fatal(&format!("--pretty flowgraph couldn't find id: {}", nodeid))
                });

                match blocks::Code::from_node(&tcx.hir(), nodeid) {
                    Some(code) => {
                        let variants = gather_flowgraph_variants(tcx.sess);

                        let out: &mut dyn Write = &mut out;

                        print_flowgraph(variants, tcx, code, mode, out)
                    }
                    None => {
                        let message = format!("--pretty=flowgraph needs block, fn, or method; \
                                               got {:?}",
                                              node);

                        tcx.sess.span_fatal(tcx.hir().span(nodeid), &message)
                    }
                }
            }
            _ => unreachable!(),
        }
    }),
                 sess)
        .unwrap();

    write_output(out, ofile);
}
