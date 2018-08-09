// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A classic liveness analysis based on dataflow over the AST.  Computes,
//! for each local variable in a function, whether that variable is live
//! at a given point.  Program execution points are identified by their
//! id.
//!
//! # Basic idea
//!
//! The basic model is that each local variable is assigned an index.  We
//! represent sets of local variables using a vector indexed by this
//! index.  The value in the vector is either 0, indicating the variable
//! is dead, or the id of an expression that uses the variable.
//!
//! We conceptually walk over the AST in reverse execution order.  If we
//! find a use of a variable, we add it to the set of live variables.  If
//! we find an assignment to a variable, we remove it from the set of live
//! variables.  When we have to merge two flows, we take the union of
//! those two flows---if the variable is live on both paths, we simply
//! pick one id.  In the event of loops, we continue doing this until a
//! fixed point is reached.
//!
//! ## Checking initialization
//!
//! At the function entry point, all variables must be dead.  If this is
//! not the case, we can report an error using the id found in the set of
//! live variables, which identifies a use of the variable which is not
//! dominated by an assignment.
//!
//! ## Checking moves
//!
//! After each explicit move, the variable must be dead.
//!
//! ## Computing last uses
//!
//! Any use of the variable where the variable is dead afterwards is a
//! last use.
//!
//! # Implementation details
//!
//! The actual implementation contains two (nested) walks over the AST.
//! The outer walk has the job of building up the ir_maps instance for the
//! enclosing function.  On the way down the tree, it identifies those AST
//! nodes and variable IDs that will be needed for the liveness analysis
//! and assigns them contiguous IDs.  The liveness id for an AST node is
//! called a `live_node` (it's a newtype'd u32) and the id for a variable
//! is called a `variable` (another newtype'd u32).
//!
//! On the way back up the tree, as we are about to exit from a function
//! declaration we allocate a `liveness` instance.  Now that we know
//! precisely how many nodes and variables we need, we can allocate all
//! the various arrays that we will need to precisely the right size.  We then
//! perform the actual propagation on the `liveness` instance.
//!
//! This propagation is encoded in the various `propagate_through_*()`
//! methods.  It effectively does a reverse walk of the AST; whenever we
//! reach a loop node, we iterate until a fixed point is reached.
//!
//! ## The `Users` struct
//!
//! At each live node `N`, we track three pieces of information for each
//! variable `V` (these are encapsulated in the `Users` struct):
//!
//! - `reader`: the `LiveNode` ID of some node which will read the value
//!    that `V` holds on entry to `N`.  Formally: a node `M` such
//!    that there exists a path `P` from `N` to `M` where `P` does not
//!    write `V`.  If the `reader` is `invalid_node()`, then the current
//!    value will never be read (the variable is dead, essentially).
//!
//! - `writer`: the `LiveNode` ID of some node which will write the
//!    variable `V` and which is reachable from `N`.  Formally: a node `M`
//!    such that there exists a path `P` from `N` to `M` and `M` writes
//!    `V`.  If the `writer` is `invalid_node()`, then there is no writer
//!    of `V` that follows `N`.
//!
//! - `used`: a boolean value indicating whether `V` is *used*.  We
//!   distinguish a *read* from a *use* in that a *use* is some read that
//!   is not just used to generate a new value.  For example, `x += 1` is
//!   a read but not a use.  This is used to generate better warnings.
//!
//! ## Special Variables
//!
//! We generate various special variables for various, well, special purposes.
//! These are described in the `specials` struct:
//!
//! - `exit_ln`: a live node that is generated to represent every 'exit' from
//!   the function, whether it be by explicit return, panic, or other means.
//!
//! - `fallthrough_ln`: a live node that represents a fallthrough
//!
//! - `clean_exit_var`: a synthetic variable that is only 'read' from the
//!   fallthrough node.  It is only live if the function could converge
//!   via means other than an explicit `return` expression. That is, it is
//!   only dead if the end of the function's block can never be reached.
//!   It is the responsibility of typeck to ensure that there are no
//!   `return` expressions in a function declared as diverging.
use self::LoopKind::*;
use self::LiveNodeKind::*;
use self::VarKind::*;

use hir::def::*;
use ty::{self, TyCtxt};
use lint;
use errors::Applicability;
use util::nodemap::{NodeMap, HirIdMap, HirIdSet};

use std::collections::VecDeque;
use std::{fmt, u32};
use std::io::prelude::*;
use std::io;
use std::rc::Rc;
use syntax::ast::{self, NodeId};
use syntax::ptr::P;
use syntax::symbol::keywords;
use syntax_pos::Span;

use hir::{Expr, HirId};
use hir;
use hir::intravisit::{self, Visitor, FnKind, NestedVisitorMap};

/// For use with `propagate_through_loop`.
enum LoopKind<'a> {
    /// An endless `loop` loop.
    LoopLoop,
    /// A `while` loop, with the given expression as condition.
    WhileLoop(&'a Expr),
}

#[derive(Copy, Clone, PartialEq)]
struct Variable(u32);

#[derive(Copy, Clone, PartialEq)]
struct LiveNode(u32);

impl Variable {
    fn get(&self) -> usize { self.0 as usize }
}

impl LiveNode {
    fn get(&self) -> usize { self.0 as usize }
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum LiveNodeKind {
    FreeVarNode(Span),
    ExprNode(Span),
    VarDefNode(Span),
    ExitNode
}

fn live_node_kind_to_string(lnk: LiveNodeKind, tcx: TyCtxt) -> String {
    let cm = tcx.sess.codemap();
    match lnk {
        FreeVarNode(s) => {
            format!("Free var node [{}]", cm.span_to_string(s))
        }
        ExprNode(s) => {
            format!("Expr node [{}]", cm.span_to_string(s))
        }
        VarDefNode(s) => {
            format!("Var def node [{}]", cm.span_to_string(s))
        }
        ExitNode => "Exit node".to_string(),
    }
}

impl<'a, 'tcx> Visitor<'tcx> for IrMaps<'a, 'tcx> {
    fn nested_visit_map<'this>(&'this mut self) -> NestedVisitorMap<'this, 'tcx> {
        NestedVisitorMap::OnlyBodies(&self.tcx.hir)
    }

    fn visit_fn(&mut self, fk: FnKind<'tcx>, fd: &'tcx hir::FnDecl,
                b: hir::BodyId, s: Span, id: NodeId) {
        visit_fn(self, fk, fd, b, s, id);
    }

    fn visit_local(&mut self, l: &'tcx hir::Local) { visit_local(self, l); }
    fn visit_expr(&mut self, ex: &'tcx Expr) { visit_expr(self, ex); }
    fn visit_arm(&mut self, a: &'tcx hir::Arm) { visit_arm(self, a); }
}

pub fn check_crate<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>) {
    tcx.hir.krate().visit_all_item_likes(&mut IrMaps::new(tcx).as_deep_visitor());
    tcx.sess.abort_if_errors();
}

impl fmt::Debug for LiveNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ln({})", self.get())
    }
}

impl fmt::Debug for Variable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "v({})", self.get())
    }
}

// ______________________________________________________________________
// Creating ir_maps
//
// This is the first pass and the one that drives the main
// computation.  It walks up and down the IR once.  On the way down,
// we count for each function the number of variables as well as
// liveness nodes.  A liveness node is basically an expression or
// capture clause that does something of interest: either it has
// interesting control flow or it uses/defines a local variable.
//
// On the way back up, at each function node we create liveness sets
// (we now know precisely how big to make our various vectors and so
// forth) and then do the data-flow propagation to compute the set
// of live variables at each program point.
//
// Finally, we run back over the IR one last time and, using the
// computed liveness, check various safety conditions.  For example,
// there must be no live nodes at the definition site for a variable
// unless it has an initializer.  Similarly, each non-mutable local
// variable must not be assigned if there is some successor
// assignment.  And so forth.

impl LiveNode {
    fn is_valid(&self) -> bool {
        self.0 != u32::MAX
    }
}

fn invalid_node() -> LiveNode { LiveNode(u32::MAX) }

struct CaptureInfo {
    ln: LiveNode,
    var_hid: HirId
}

#[derive(Copy, Clone, Debug)]
struct LocalInfo {
    id: HirId,
    name: ast::Name,
    is_shorthand: bool,
}

#[derive(Copy, Clone, Debug)]
enum VarKind {
    Arg(HirId, ast::Name),
    Local(LocalInfo),
    CleanExit
}

struct IrMaps<'a, 'tcx: 'a> {
    tcx: TyCtxt<'a, 'tcx, 'tcx>,

    num_live_nodes: usize,
    num_vars: usize,
    live_node_map: HirIdMap<LiveNode>,
    variable_map: HirIdMap<Variable>,
    capture_info_map: NodeMap<Rc<Vec<CaptureInfo>>>,
    var_kinds: Vec<VarKind>,
    lnks: Vec<LiveNodeKind>,
}

impl<'a, 'tcx> IrMaps<'a, 'tcx> {
    fn new(tcx: TyCtxt<'a, 'tcx, 'tcx>) -> IrMaps<'a, 'tcx> {
        IrMaps {
            tcx,
            num_live_nodes: 0,
            num_vars: 0,
            live_node_map: HirIdMap(),
            variable_map: HirIdMap(),
            capture_info_map: NodeMap(),
            var_kinds: Vec::new(),
            lnks: Vec::new(),
        }
    }

    fn add_live_node(&mut self, lnk: LiveNodeKind) -> LiveNode {
        let ln = LiveNode(self.num_live_nodes as u32);
        self.lnks.push(lnk);
        self.num_live_nodes += 1;

        debug!("{:?} is of kind {}", ln,
               live_node_kind_to_string(lnk, self.tcx));

        ln
    }

    fn add_live_node_for_node(&mut self, hir_id: HirId, lnk: LiveNodeKind) {
        let ln = self.add_live_node(lnk);
        self.live_node_map.insert(hir_id, ln);

        debug!("{:?} is node {:?}", ln, hir_id);
    }

    fn add_variable(&mut self, vk: VarKind) -> Variable {
        let v = Variable(self.num_vars as u32);
        self.var_kinds.push(vk);
        self.num_vars += 1;

        match vk {
            Local(LocalInfo { id: node_id, .. }) | Arg(node_id, _) => {
                self.variable_map.insert(node_id, v);
            },
            CleanExit => {}
        }

        debug!("{:?} is {:?}", v, vk);

        v
    }

    fn variable(&self, hir_id: HirId, span: Span) -> Variable {
        match self.variable_map.get(&hir_id) {
            Some(&var) => var,
            None => {
                span_bug!(span, "no variable registered for id {:?}", hir_id);
            }
        }
    }

    fn variable_name(&self, var: Variable) -> String {
        match self.var_kinds[var.get()] {
            Local(LocalInfo { name, .. }) | Arg(_, name) => {
                name.to_string()
            },
            CleanExit => "<clean-exit>".to_string()
        }
    }

    fn variable_is_shorthand(&self, var: Variable) -> bool {
        match self.var_kinds[var.get()] {
            Local(LocalInfo { is_shorthand, .. }) => is_shorthand,
            Arg(..) | CleanExit => false
        }
    }

    fn set_captures(&mut self, node_id: NodeId, cs: Vec<CaptureInfo>) {
        self.capture_info_map.insert(node_id, Rc::new(cs));
    }

    fn lnk(&self, ln: LiveNode) -> LiveNodeKind {
        self.lnks[ln.get()]
    }
}

fn visit_fn<'a, 'tcx: 'a>(ir: &mut IrMaps<'a, 'tcx>,
                          fk: FnKind<'tcx>,
                          decl: &'tcx hir::FnDecl,
                          body_id: hir::BodyId,
                          sp: Span,
                          id: ast::NodeId) {
    debug!("visit_fn");

    // swap in a new set of IR maps for this function body:
    let mut fn_maps = IrMaps::new(ir.tcx);

    // Don't run unused pass for #[derive()]
    if let FnKind::Method(..) = fk {
        let parent = ir.tcx.hir.get_parent(id);
        if let Some(hir::map::Node::NodeItem(i)) = ir.tcx.hir.find(parent) {
            if i.attrs.iter().any(|a| a.check_name("automatically_derived")) {
                return;
            }
        }
    }

    debug!("creating fn_maps: {:?}", &fn_maps as *const IrMaps);

    let body = ir.tcx.hir.body(body_id);

    for arg in &body.arguments {
        arg.pat.each_binding(|_bm, hir_id, _x, path1| {
            debug!("adding argument {:?}", hir_id);
            let name = path1.node;
            fn_maps.add_variable(Arg(hir_id, name));
        })
    };

    // gather up the various local variables, significant expressions,
    // and so forth:
    intravisit::walk_fn(&mut fn_maps, fk, decl, body_id, sp, id);

    // compute liveness
    let mut lsets = Liveness::new(&mut fn_maps, body_id);
    let entry_ln = lsets.compute(&body.value);

    // check for various error conditions
    lsets.visit_body(body);
    lsets.warn_about_unused_args(body, entry_ln);
}

fn add_from_pat<'a, 'tcx>(ir: &mut IrMaps<'a, 'tcx>, pat: &P<hir::Pat>) {
    // For struct patterns, take note of which fields used shorthand
    // (`x` rather than `x: x`).
    let mut shorthand_field_ids = HirIdSet();
    let mut pats = VecDeque::new();
    pats.push_back(pat);
    while let Some(pat) = pats.pop_front() {
        use hir::PatKind::*;
        match pat.node {
            Binding(_, _, _, ref inner_pat) => {
                pats.extend(inner_pat.iter());
            }
            Struct(_, ref fields, _) => {
                for field in fields {
                    if field.node.is_shorthand {
                        shorthand_field_ids.insert(field.node.pat.hir_id);
                    }
                }
            }
            Ref(ref inner_pat, _) |
            Box(ref inner_pat) => {
                pats.push_back(inner_pat);
            }
            TupleStruct(_, ref inner_pats, _) |
            Tuple(ref inner_pats, _) => {
                pats.extend(inner_pats.iter());
            }
            Slice(ref pre_pats, ref inner_pat, ref post_pats) => {
                pats.extend(pre_pats.iter());
                pats.extend(inner_pat.iter());
                pats.extend(post_pats.iter());
            }
            _ => {}
        }
    }

    pat.each_binding(|_bm, hir_id, _sp, path1| {
        let name = path1.node;
        ir.add_live_node_for_node(hir_id, VarDefNode(path1.span));
        ir.add_variable(Local(LocalInfo {
            id: hir_id,
            name,
            is_shorthand: shorthand_field_ids.contains(&hir_id)
        }));
    });
}

fn visit_local<'a, 'tcx>(ir: &mut IrMaps<'a, 'tcx>, local: &'tcx hir::Local) {
    add_from_pat(ir, &local.pat);
    intravisit::walk_local(ir, local);
}

fn visit_arm<'a, 'tcx>(ir: &mut IrMaps<'a, 'tcx>, arm: &'tcx hir::Arm) {
    for pat in &arm.pats {
        add_from_pat(ir, pat);
    }
    intravisit::walk_arm(ir, arm);
}

fn visit_expr<'a, 'tcx>(ir: &mut IrMaps<'a, 'tcx>, expr: &'tcx Expr) {
    match expr.node {
      // live nodes required for uses or definitions of variables:
      hir::ExprPath(hir::QPath::Resolved(_, ref path)) => {
        debug!("expr {}: path that leads to {:?}", expr.id, path.def);
        if let Def::Local(..) = path.def {
            ir.add_live_node_for_node(expr.hir_id, ExprNode(expr.span));
        }
        intravisit::walk_expr(ir, expr);
      }
      hir::ExprClosure(..) => {
        // Interesting control flow (for loops can contain labeled
        // breaks or continues)
        ir.add_live_node_for_node(expr.hir_id, ExprNode(expr.span));

        // Make a live_node for each captured variable, with the span
        // being the location that the variable is used.  This results
        // in better error messages than just pointing at the closure
        // construction site.
        let mut call_caps = Vec::new();
        ir.tcx.with_freevars(expr.id, |freevars| {
            for fv in freevars {
                if let Def::Local(rv) = fv.def {
                    let fv_ln = ir.add_live_node(FreeVarNode(fv.span));
                    let var_hid = ir.tcx.hir.node_to_hir_id(rv);
                    call_caps.push(CaptureInfo { ln: fv_ln, var_hid });
                }
            }
        });
        ir.set_captures(expr.id, call_caps);

        intravisit::walk_expr(ir, expr);
      }

      // live nodes required for interesting control flow:
      hir::ExprIf(..) | hir::ExprMatch(..) | hir::ExprWhile(..) | hir::ExprLoop(..) => {
        ir.add_live_node_for_node(expr.hir_id, ExprNode(expr.span));
        intravisit::walk_expr(ir, expr);
      }
      hir::ExprBinary(op, ..) if op.node.is_lazy() => {
        ir.add_live_node_for_node(expr.hir_id, ExprNode(expr.span));
        intravisit::walk_expr(ir, expr);
      }

      // otherwise, live nodes are not required:
      hir::ExprIndex(..) | hir::ExprField(..) |
      hir::ExprArray(..) | hir::ExprCall(..) | hir::ExprMethodCall(..) |
      hir::ExprTup(..) | hir::ExprBinary(..) | hir::ExprAddrOf(..) |
      hir::ExprCast(..) | hir::ExprUnary(..) | hir::ExprBreak(..) |
      hir::ExprAgain(_) | hir::ExprLit(_) | hir::ExprRet(..) |
      hir::ExprBlock(..) | hir::ExprAssign(..) | hir::ExprAssignOp(..) |
      hir::ExprStruct(..) | hir::ExprRepeat(..) |
      hir::ExprInlineAsm(..) | hir::ExprBox(..) | hir::ExprYield(..) |
      hir::ExprType(..) | hir::ExprPath(hir::QPath::TypeRelative(..)) => {
          intravisit::walk_expr(ir, expr);
      }
    }
}

// ______________________________________________________________________
// Computing liveness sets
//
// Actually we compute just a bit more than just liveness, but we use
// the same basic propagation framework in all cases.

#[derive(Clone, Copy)]
struct Users {
    reader: LiveNode,
    writer: LiveNode,
    used: bool
}

fn invalid_users() -> Users {
    Users {
        reader: invalid_node(),
        writer: invalid_node(),
        used: false
    }
}

#[derive(Copy, Clone)]
struct Specials {
    exit_ln: LiveNode,
    fallthrough_ln: LiveNode,
    clean_exit_var: Variable
}

const ACC_READ: u32 = 1;
const ACC_WRITE: u32 = 2;
const ACC_USE: u32 = 4;

struct Liveness<'a, 'tcx: 'a> {
    ir: &'a mut IrMaps<'a, 'tcx>,
    tables: &'a ty::TypeckTables<'tcx>,
    s: Specials,
    successors: Vec<LiveNode>,
    users: Vec<Users>,

    // mappings from loop node ID to LiveNode
    // ("break" label should map to loop node ID,
    // it probably doesn't now)
    break_ln: NodeMap<LiveNode>,
    cont_ln: NodeMap<LiveNode>,
}

impl<'a, 'tcx> Liveness<'a, 'tcx> {
    fn new(ir: &'a mut IrMaps<'a, 'tcx>, body: hir::BodyId) -> Liveness<'a, 'tcx> {
        // Special nodes and variables:
        // - exit_ln represents the end of the fn, either by return or panic
        // - implicit_ret_var is a pseudo-variable that represents
        //   an implicit return
        let specials = Specials {
            exit_ln: ir.add_live_node(ExitNode),
            fallthrough_ln: ir.add_live_node(ExitNode),
            clean_exit_var: ir.add_variable(CleanExit)
        };

        let tables = ir.tcx.body_tables(body);

        let num_live_nodes = ir.num_live_nodes;
        let num_vars = ir.num_vars;

        Liveness {
            ir,
            tables,
            s: specials,
            successors: vec![invalid_node(); num_live_nodes],
            users: vec![invalid_users(); num_live_nodes * num_vars],
            break_ln: NodeMap(),
            cont_ln: NodeMap(),
        }
    }

    fn live_node(&self, hir_id: HirId, span: Span) -> LiveNode {
        match self.ir.live_node_map.get(&hir_id) {
          Some(&ln) => ln,
          None => {
            // This must be a mismatch between the ir_map construction
            // above and the propagation code below; the two sets of
            // code have to agree about which AST nodes are worth
            // creating liveness nodes for.
            span_bug!(
                span,
                "no live node registered for node {:?}",
                hir_id);
          }
        }
    }

    fn variable(&self, hir_id: HirId, span: Span) -> Variable {
        self.ir.variable(hir_id, span)
    }

    fn pat_bindings<F>(&mut self, pat: &hir::Pat, mut f: F) where
        F: FnMut(&mut Liveness<'a, 'tcx>, LiveNode, Variable, Span, HirId),
    {
        pat.each_binding(|_bm, hir_id, sp, n| {
            let ln = self.live_node(hir_id, sp);
            let var = self.variable(hir_id, n.span);
            f(self, ln, var, n.span, hir_id);
        })
    }

    fn arm_pats_bindings<F>(&mut self, pat: Option<&hir::Pat>, f: F) where
        F: FnMut(&mut Liveness<'a, 'tcx>, LiveNode, Variable, Span, HirId),
    {
        if let Some(pat) = pat {
            self.pat_bindings(pat, f);
        }
    }

    fn define_bindings_in_pat(&mut self, pat: &hir::Pat, succ: LiveNode)
                              -> LiveNode {
        self.define_bindings_in_arm_pats(Some(pat), succ)
    }

    fn define_bindings_in_arm_pats(&mut self, pat: Option<&hir::Pat>, succ: LiveNode)
                                   -> LiveNode {
        let mut succ = succ;
        self.arm_pats_bindings(pat, |this, ln, var, _sp, _id| {
            this.init_from_succ(ln, succ);
            this.define(ln, var);
            succ = ln;
        });
        succ
    }

    fn idx(&self, ln: LiveNode, var: Variable) -> usize {
        ln.get() * self.ir.num_vars + var.get()
    }

    fn live_on_entry(&self, ln: LiveNode, var: Variable)
                      -> Option<LiveNodeKind> {
        assert!(ln.is_valid());
        let reader = self.users[self.idx(ln, var)].reader;
        if reader.is_valid() {Some(self.ir.lnk(reader))} else {None}
    }

    /*
    Is this variable live on entry to any of its successor nodes?
    */
    fn live_on_exit(&self, ln: LiveNode, var: Variable)
                    -> Option<LiveNodeKind> {
        let successor = self.successors[ln.get()];
        self.live_on_entry(successor, var)
    }

    fn used_on_entry(&self, ln: LiveNode, var: Variable) -> bool {
        assert!(ln.is_valid());
        self.users[self.idx(ln, var)].used
    }

    fn assigned_on_entry(&self, ln: LiveNode, var: Variable)
                         -> Option<LiveNodeKind> {
        assert!(ln.is_valid());
        let writer = self.users[self.idx(ln, var)].writer;
        if writer.is_valid() {Some(self.ir.lnk(writer))} else {None}
    }

    fn assigned_on_exit(&self, ln: LiveNode, var: Variable)
                        -> Option<LiveNodeKind> {
        let successor = self.successors[ln.get()];
        self.assigned_on_entry(successor, var)
    }

    fn indices2<F>(&mut self, ln: LiveNode, succ_ln: LiveNode, mut op: F) where
        F: FnMut(&mut Liveness<'a, 'tcx>, usize, usize),
    {
        let node_base_idx = self.idx(ln, Variable(0));
        let succ_base_idx = self.idx(succ_ln, Variable(0));
        for var_idx in 0..self.ir.num_vars {
            op(self, node_base_idx + var_idx, succ_base_idx + var_idx);
        }
    }

    fn write_vars<F>(&self,
                     wr: &mut dyn Write,
                     ln: LiveNode,
                     mut test: F)
                     -> io::Result<()> where
        F: FnMut(usize) -> LiveNode,
    {
        let node_base_idx = self.idx(ln, Variable(0));
        for var_idx in 0..self.ir.num_vars {
            let idx = node_base_idx + var_idx;
            if test(idx).is_valid() {
                write!(wr, " {:?}", Variable(var_idx as u32))?;
            }
        }
        Ok(())
    }


    #[allow(unused_must_use)]
    fn ln_str(&self, ln: LiveNode) -> String {
        let mut wr = Vec::new();
        {
            let wr = &mut wr as &mut dyn Write;
            write!(wr, "[ln({:?}) of kind {:?} reads", ln.get(), self.ir.lnk(ln));
            self.write_vars(wr, ln, |idx| self.users[idx].reader);
            write!(wr, "  writes");
            self.write_vars(wr, ln, |idx| self.users[idx].writer);
            write!(wr, "  precedes {:?}]", self.successors[ln.get()]);
        }
        String::from_utf8(wr).unwrap()
    }

    fn init_empty(&mut self, ln: LiveNode, succ_ln: LiveNode) {
        self.successors[ln.get()] = succ_ln;

        // It is not necessary to initialize the
        // values to empty because this is the value
        // they have when they are created, and the sets
        // only grow during iterations.
        //
        // self.indices(ln) { |idx|
        //     self.users[idx] = invalid_users();
        // }
    }

    fn init_from_succ(&mut self, ln: LiveNode, succ_ln: LiveNode) {
        // more efficient version of init_empty() / merge_from_succ()
        self.successors[ln.get()] = succ_ln;

        self.indices2(ln, succ_ln, |this, idx, succ_idx| {
            this.users[idx] = this.users[succ_idx]
        });
        debug!("init_from_succ(ln={}, succ={})",
               self.ln_str(ln), self.ln_str(succ_ln));
    }

    fn merge_from_succ(&mut self,
                       ln: LiveNode,
                       succ_ln: LiveNode,
                       first_merge: bool)
                       -> bool {
        if ln == succ_ln { return false; }

        let mut changed = false;
        self.indices2(ln, succ_ln, |this, idx, succ_idx| {
            changed |= copy_if_invalid(this.users[succ_idx].reader,
                                       &mut this.users[idx].reader);
            changed |= copy_if_invalid(this.users[succ_idx].writer,
                                       &mut this.users[idx].writer);
            if this.users[succ_idx].used && !this.users[idx].used {
                this.users[idx].used = true;
                changed = true;
            }
        });

        debug!("merge_from_succ(ln={:?}, succ={}, first_merge={}, changed={})",
               ln, self.ln_str(succ_ln), first_merge, changed);
        return changed;

        fn copy_if_invalid(src: LiveNode, dst: &mut LiveNode) -> bool {
            if src.is_valid() && !dst.is_valid() {
                *dst = src;
                true
            } else {
                false
            }
        }
    }

    // Indicates that a local variable was *defined*; we know that no
    // uses of the variable can precede the definition (resolve checks
    // this) so we just clear out all the data.
    fn define(&mut self, writer: LiveNode, var: Variable) {
        let idx = self.idx(writer, var);
        self.users[idx].reader = invalid_node();
        self.users[idx].writer = invalid_node();

        debug!("{:?} defines {:?} (idx={}): {}", writer, var,
               idx, self.ln_str(writer));
    }

    // Either read, write, or both depending on the acc bitset
    fn acc(&mut self, ln: LiveNode, var: Variable, acc: u32) {
        debug!("{:?} accesses[{:x}] {:?}: {}",
               ln, acc, var, self.ln_str(ln));

        let idx = self.idx(ln, var);
        let user = &mut self.users[idx];

        if (acc & ACC_WRITE) != 0 {
            user.reader = invalid_node();
            user.writer = ln;
        }

        // Important: if we both read/write, must do read second
        // or else the write will override.
        if (acc & ACC_READ) != 0 {
            user.reader = ln;
        }

        if (acc & ACC_USE) != 0 {
            user.used = true;
        }
    }

    // _______________________________________________________________________

    fn compute(&mut self, body: &hir::Expr) -> LiveNode {
        // if there is a `break` or `again` at the top level, then it's
        // effectively a return---this only occurs in `for` loops,
        // where the body is really a closure.

        debug!("compute: using id for body, {}", self.ir.tcx.hir.node_to_pretty_string(body.id));

        let exit_ln = self.s.exit_ln;

        self.break_ln.insert(body.id, exit_ln);
        self.cont_ln.insert(body.id, exit_ln);

        // the fallthrough exit is only for those cases where we do not
        // explicitly return:
        let s = self.s;
        self.init_from_succ(s.fallthrough_ln, s.exit_ln);
        self.acc(s.fallthrough_ln, s.clean_exit_var, ACC_READ);

        let entry_ln = self.propagate_through_expr(body, s.fallthrough_ln);

        // hack to skip the loop unless debug! is enabled:
        debug!("^^ liveness computation results for body {} (entry={:?})",
               {
                   for ln_idx in 0..self.ir.num_live_nodes {
                       debug!("{:?}", self.ln_str(LiveNode(ln_idx as u32)));
                   }
                   body.id
               },
               entry_ln);

        entry_ln
    }

    fn propagate_through_block(&mut self, blk: &hir::Block, succ: LiveNode)
                               -> LiveNode {
        if blk.targeted_by_break {
            self.break_ln.insert(blk.id, succ);
        }
        let succ = self.propagate_through_opt_expr(blk.expr.as_ref().map(|e| &**e), succ);
        blk.stmts.iter().rev().fold(succ, |succ, stmt| {
            self.propagate_through_stmt(stmt, succ)
        })
    }

    fn propagate_through_stmt(&mut self, stmt: &hir::Stmt, succ: LiveNode)
                              -> LiveNode {
        match stmt.node {
            hir::StmtDecl(ref decl, _) => {
                self.propagate_through_decl(&decl, succ)
            }

            hir::StmtExpr(ref expr, _) | hir::StmtSemi(ref expr, _) => {
                self.propagate_through_expr(&expr, succ)
            }
        }
    }

    fn propagate_through_decl(&mut self, decl: &hir::Decl, succ: LiveNode)
                              -> LiveNode {
        match decl.node {
            hir::DeclLocal(ref local) => {
                self.propagate_through_local(&local, succ)
            }
            hir::DeclItem(_) => succ,
        }
    }

    fn propagate_through_local(&mut self, local: &hir::Local, succ: LiveNode)
                               -> LiveNode {
        // Note: we mark the variable as defined regardless of whether
        // there is an initializer.  Initially I had thought to only mark
        // the live variable as defined if it was initialized, and then we
        // could check for uninit variables just by scanning what is live
        // at the start of the function. But that doesn't work so well for
        // immutable variables defined in a loop:
        //     loop { let x; x = 5; }
        // because the "assignment" loops back around and generates an error.
        //
        // So now we just check that variables defined w/o an
        // initializer are not live at the point of their
        // initialization, which is mildly more complex than checking
        // once at the func header but otherwise equivalent.

        let succ = self.propagate_through_opt_expr(local.init.as_ref().map(|e| &**e), succ);
        self.define_bindings_in_pat(&local.pat, succ)
    }

    fn propagate_through_exprs(&mut self, exprs: &[Expr], succ: LiveNode)
                               -> LiveNode {
        exprs.iter().rev().fold(succ, |succ, expr| {
            self.propagate_through_expr(&expr, succ)
        })
    }

    fn propagate_through_opt_expr(&mut self,
                                  opt_expr: Option<&Expr>,
                                  succ: LiveNode)
                                  -> LiveNode {
        opt_expr.map_or(succ, |expr| self.propagate_through_expr(expr, succ))
    }

    fn propagate_through_expr(&mut self, expr: &Expr, succ: LiveNode)
                              -> LiveNode {
        debug!("propagate_through_expr: {}", self.ir.tcx.hir.node_to_pretty_string(expr.id));

        match expr.node {
          // Interesting cases with control flow or which gen/kill
          hir::ExprPath(hir::QPath::Resolved(_, ref path)) => {
              self.access_path(expr.hir_id, path, succ, ACC_READ | ACC_USE)
          }

          hir::ExprField(ref e, _) => {
              self.propagate_through_expr(&e, succ)
          }

          hir::ExprClosure(.., blk_id, _, _) => {
              debug!("{} is an ExprClosure", self.ir.tcx.hir.node_to_pretty_string(expr.id));

              // The next-node for a break is the successor of the entire
              // loop. The next-node for a continue is the top of this loop.
              let node = self.live_node(expr.hir_id, expr.span);

              let break_ln = succ;
              let cont_ln = node;
              self.break_ln.insert(blk_id.node_id, break_ln);
              self.cont_ln.insert(blk_id.node_id, cont_ln);

              // the construction of a closure itself is not important,
              // but we have to consider the closed over variables.
              let caps = match self.ir.capture_info_map.get(&expr.id) {
                  Some(caps) => caps.clone(),
                  None => {
                      span_bug!(expr.span, "no registered caps");
                  }
              };
              caps.iter().rev().fold(succ, |succ, cap| {
                  self.init_from_succ(cap.ln, succ);
                  let var = self.variable(cap.var_hid, expr.span);
                  self.acc(cap.ln, var, ACC_READ | ACC_USE);
                  cap.ln
              })
          }

          hir::ExprIf(ref cond, ref then, ref els) => {
            //
            //     (cond)
            //       |
            //       v
            //     (expr)
            //     /   \
            //    |     |
            //    v     v
            //  (then)(els)
            //    |     |
            //    v     v
            //   (  succ  )
            //
            let else_ln = self.propagate_through_opt_expr(els.as_ref().map(|e| &**e), succ);
            let then_ln = self.propagate_through_expr(&then, succ);
            let ln = self.live_node(expr.hir_id, expr.span);
            self.init_from_succ(ln, else_ln);
            self.merge_from_succ(ln, then_ln, false);
            self.propagate_through_expr(&cond, ln)
          }

          hir::ExprWhile(ref cond, ref blk, _) => {
            self.propagate_through_loop(expr, WhileLoop(&cond), &blk, succ)
          }

          // Note that labels have been resolved, so we don't need to look
          // at the label ident
          hir::ExprLoop(ref blk, _, _) => {
            self.propagate_through_loop(expr, LoopLoop, &blk, succ)
          }

          hir::ExprMatch(ref e, ref arms, _) => {
            //
            //      (e)
            //       |
            //       v
            //     (expr)
            //     / | \
            //    |  |  |
            //    v  v  v
            //   (..arms..)
            //    |  |  |
            //    v  v  v
            //   (  succ  )
            //
            //
            let ln = self.live_node(expr.hir_id, expr.span);
            self.init_empty(ln, succ);
            let mut first_merge = true;
            for arm in arms {
                let body_succ =
                    self.propagate_through_expr(&arm.body, succ);
                let guard_succ =
                    self.propagate_through_opt_expr(arm.guard.as_ref().map(|e| &**e), body_succ);
                // only consider the first pattern; any later patterns must have
                // the same bindings, and we also consider the first pattern to be
                // the "authoritative" set of ids
                let arm_succ =
                    self.define_bindings_in_arm_pats(arm.pats.first().map(|p| &**p),
                                                     guard_succ);
                self.merge_from_succ(ln, arm_succ, first_merge);
                first_merge = false;
            };
            self.propagate_through_expr(&e, ln)
          }

          hir::ExprRet(ref o_e) => {
            // ignore succ and subst exit_ln:
            let exit_ln = self.s.exit_ln;
            self.propagate_through_opt_expr(o_e.as_ref().map(|e| &**e), exit_ln)
          }

          hir::ExprBreak(label, ref opt_expr) => {
              // Find which label this break jumps to
              let target = match label.target_id {
                    Ok(node_id) => self.break_ln.get(&node_id),
                    Err(err) => span_bug!(expr.span, "loop scope error: {}", err),
              }.map(|x| *x);

              // Now that we know the label we're going to,
              // look it up in the break loop nodes table

              match target {
                  Some(b) => self.propagate_through_opt_expr(opt_expr.as_ref().map(|e| &**e), b),
                  None => span_bug!(expr.span, "break to unknown label")
              }
          }

          hir::ExprAgain(label) => {
              // Find which label this expr continues to
              let sc = match label.target_id {
                    Ok(node_id) => node_id,
                    Err(err) => span_bug!(expr.span, "loop scope error: {}", err),
              };

              // Now that we know the label we're going to,
              // look it up in the continue loop nodes table

              match self.cont_ln.get(&sc) {
                  Some(&b) => b,
                  None => span_bug!(expr.span, "continue to unknown label")
              }
          }

          hir::ExprAssign(ref l, ref r) => {
            // see comment on places in
            // propagate_through_place_components()
            let succ = self.write_place(&l, succ, ACC_WRITE);
            let succ = self.propagate_through_place_components(&l, succ);
            self.propagate_through_expr(&r, succ)
          }

          hir::ExprAssignOp(_, ref l, ref r) => {
            // an overloaded assign op is like a method call
            if self.tables.is_method_call(expr) {
                let succ = self.propagate_through_expr(&l, succ);
                self.propagate_through_expr(&r, succ)
            } else {
                // see comment on places in
                // propagate_through_place_components()
                let succ = self.write_place(&l, succ, ACC_WRITE|ACC_READ);
                let succ = self.propagate_through_expr(&r, succ);
                self.propagate_through_place_components(&l, succ)
            }
          }

          // Uninteresting cases: just propagate in rev exec order

          hir::ExprArray(ref exprs) => {
            self.propagate_through_exprs(exprs, succ)
          }

          hir::ExprStruct(_, ref fields, ref with_expr) => {
            let succ = self.propagate_through_opt_expr(with_expr.as_ref().map(|e| &**e), succ);
            fields.iter().rev().fold(succ, |succ, field| {
                self.propagate_through_expr(&field.expr, succ)
            })
          }

          hir::ExprCall(ref f, ref args) => {
            // FIXME(canndrew): This is_never should really be an is_uninhabited
            let succ = if self.tables.expr_ty(expr).is_never() {
                self.s.exit_ln
            } else {
                succ
            };
            let succ = self.propagate_through_exprs(args, succ);
            self.propagate_through_expr(&f, succ)
          }

          hir::ExprMethodCall(.., ref args) => {
            // FIXME(canndrew): This is_never should really be an is_uninhabited
            let succ = if self.tables.expr_ty(expr).is_never() {
                self.s.exit_ln
            } else {
                succ
            };
            self.propagate_through_exprs(args, succ)
          }

          hir::ExprTup(ref exprs) => {
            self.propagate_through_exprs(exprs, succ)
          }

          hir::ExprBinary(op, ref l, ref r) if op.node.is_lazy() => {
            let r_succ = self.propagate_through_expr(&r, succ);

            let ln = self.live_node(expr.hir_id, expr.span);
            self.init_from_succ(ln, succ);
            self.merge_from_succ(ln, r_succ, false);

            self.propagate_through_expr(&l, ln)
          }

          hir::ExprIndex(ref l, ref r) |
          hir::ExprBinary(_, ref l, ref r) => {
            let r_succ = self.propagate_through_expr(&r, succ);
            self.propagate_through_expr(&l, r_succ)
          }

          hir::ExprBox(ref e) |
          hir::ExprAddrOf(_, ref e) |
          hir::ExprCast(ref e, _) |
          hir::ExprType(ref e, _) |
          hir::ExprUnary(_, ref e) |
          hir::ExprYield(ref e) |
          hir::ExprRepeat(ref e, _) => {
            self.propagate_through_expr(&e, succ)
          }

          hir::ExprInlineAsm(ref ia, ref outputs, ref inputs) => {
            let succ = ia.outputs.iter().zip(outputs).rev().fold(succ, |succ, (o, output)| {
                // see comment on places
                // in propagate_through_place_components()
                if o.is_indirect {
                    self.propagate_through_expr(output, succ)
                } else {
                    let acc = if o.is_rw { ACC_WRITE|ACC_READ } else { ACC_WRITE };
                    let succ = self.write_place(output, succ, acc);
                    self.propagate_through_place_components(output, succ)
                }
            });

            // Inputs are executed first. Propagate last because of rev order
            self.propagate_through_exprs(inputs, succ)
          }

          hir::ExprLit(..) | hir::ExprPath(hir::QPath::TypeRelative(..)) => {
            succ
          }

          // Note that labels have been resolved, so we don't need to look
          // at the label ident
          hir::ExprBlock(ref blk, _) => {
            self.propagate_through_block(&blk, succ)
          }
        }
    }

    fn propagate_through_place_components(&mut self,
                                           expr: &Expr,
                                           succ: LiveNode)
                                           -> LiveNode {
        // # Places
        //
        // In general, the full flow graph structure for an
        // assignment/move/etc can be handled in one of two ways,
        // depending on whether what is being assigned is a "tracked
        // value" or not. A tracked value is basically a local
        // variable or argument.
        //
        // The two kinds of graphs are:
        //
        //    Tracked place          Untracked place
        // ----------------------++-----------------------
        //                       ||
        //         |             ||           |
        //         v             ||           v
        //     (rvalue)          ||       (rvalue)
        //         |             ||           |
        //         v             ||           v
        // (write of place)     ||   (place components)
        //         |             ||           |
        //         v             ||           v
        //      (succ)           ||        (succ)
        //                       ||
        // ----------------------++-----------------------
        //
        // I will cover the two cases in turn:
        //
        // # Tracked places
        //
        // A tracked place is a local variable/argument `x`.  In
        // these cases, the link_node where the write occurs is linked
        // to node id of `x`.  The `write_place()` routine generates
        // the contents of this node.  There are no subcomponents to
        // consider.
        //
        // # Non-tracked places
        //
        // These are places like `x[5]` or `x.f`.  In that case, we
        // basically ignore the value which is written to but generate
        // reads for the components---`x` in these two examples.  The
        // components reads are generated by
        // `propagate_through_place_components()` (this fn).
        //
        // # Illegal places
        //
        // It is still possible to observe assignments to non-places;
        // these errors are detected in the later pass borrowck.  We
        // just ignore such cases and treat them as reads.

        match expr.node {
            hir::ExprPath(_) => succ,
            hir::ExprField(ref e, _) => self.propagate_through_expr(&e, succ),
            _ => self.propagate_through_expr(expr, succ)
        }
    }

    // see comment on propagate_through_place()
    fn write_place(&mut self, expr: &Expr, succ: LiveNode, acc: u32)
                    -> LiveNode {
        match expr.node {
          hir::ExprPath(hir::QPath::Resolved(_, ref path)) => {
              self.access_path(expr.hir_id, path, succ, acc)
          }

          // We do not track other places, so just propagate through
          // to their subcomponents.  Also, it may happen that
          // non-places occur here, because those are detected in the
          // later pass borrowck.
          _ => succ
        }
    }

    fn access_var(&mut self, hir_id: HirId, nid: NodeId, succ: LiveNode, acc: u32, span: Span)
                  -> LiveNode {
        let ln = self.live_node(hir_id, span);
        if acc != 0 {
            self.init_from_succ(ln, succ);
            let var_hid = self.ir.tcx.hir.node_to_hir_id(nid);
            let var = self.variable(var_hid, span);
            self.acc(ln, var, acc);
        }
        ln
    }

    fn access_path(&mut self, hir_id: HirId, path: &hir::Path, succ: LiveNode, acc: u32)
                   -> LiveNode {
        match path.def {
          Def::Local(nid) => {
            self.access_var(hir_id, nid, succ, acc, path.span)
          }
          _ => succ
        }
    }

    fn propagate_through_loop(&mut self,
                              expr: &Expr,
                              kind: LoopKind,
                              body: &hir::Block,
                              succ: LiveNode)
                              -> LiveNode {

        /*

        We model control flow like this:

              (cond) <--+
                |       |
                v       |
          +-- (expr)    |
          |     |       |
          |     v       |
          |   (body) ---+
          |
          |
          v
        (succ)

        */


        // first iteration:
        let mut first_merge = true;
        let ln = self.live_node(expr.hir_id, expr.span);
        self.init_empty(ln, succ);
        match kind {
            LoopLoop => {}
            _ => {
                // If this is not a `loop` loop, then it's possible we bypass
                // the body altogether. Otherwise, the only way is via a `break`
                // in the loop body.
                self.merge_from_succ(ln, succ, first_merge);
                first_merge = false;
            }
        }
        debug!("propagate_through_loop: using id for loop body {} {}",
               expr.id, self.ir.tcx.hir.node_to_pretty_string(body.id));

        let break_ln = succ;
        let cont_ln = ln;
        self.break_ln.insert(expr.id, break_ln);
        self.cont_ln.insert(expr.id, cont_ln);

        let cond_ln = match kind {
            LoopLoop => ln,
            WhileLoop(ref cond) => self.propagate_through_expr(&cond, ln),
        };
        let body_ln = self.propagate_through_block(body, cond_ln);

        // repeat until fixed point is reached:
        while self.merge_from_succ(ln, body_ln, first_merge) {
            first_merge = false;

            let new_cond_ln = match kind {
                LoopLoop => ln,
                WhileLoop(ref cond) => {
                    self.propagate_through_expr(&cond, ln)
                }
            };
            assert!(cond_ln == new_cond_ln);
            assert!(body_ln == self.propagate_through_block(body, cond_ln));
        }

        cond_ln
    }
}

// _______________________________________________________________________
// Checking for error conditions

impl<'a, 'tcx> Visitor<'tcx> for Liveness<'a, 'tcx> {
    fn nested_visit_map<'this>(&'this mut self) -> NestedVisitorMap<'this, 'tcx> {
        NestedVisitorMap::None
    }

    fn visit_local(&mut self, l: &'tcx hir::Local) {
        check_local(self, l);
    }
    fn visit_expr(&mut self, ex: &'tcx Expr) {
        check_expr(self, ex);
    }
    fn visit_arm(&mut self, a: &'tcx hir::Arm) {
        check_arm(self, a);
    }
}

fn check_local<'a, 'tcx>(this: &mut Liveness<'a, 'tcx>, local: &'tcx hir::Local) {
    match local.init {
        Some(_) => {
            this.warn_about_unused_or_dead_vars_in_pat(&local.pat);
        },
        None => {
            this.pat_bindings(&local.pat, |this, ln, var, sp, id| {
                let span = local.pat.simple_span().unwrap_or(sp);
                this.warn_about_unused(span, id, ln, var);
            })
        }
    }

    intravisit::walk_local(this, local);
}

fn check_arm<'a, 'tcx>(this: &mut Liveness<'a, 'tcx>, arm: &'tcx hir::Arm) {
    // only consider the first pattern; any later patterns must have
    // the same bindings, and we also consider the first pattern to be
    // the "authoritative" set of ids
    this.arm_pats_bindings(arm.pats.first().map(|p| &**p), |this, ln, var, sp, id| {
        this.warn_about_unused(sp, id, ln, var);
    });
    intravisit::walk_arm(this, arm);
}

fn check_expr<'a, 'tcx>(this: &mut Liveness<'a, 'tcx>, expr: &'tcx Expr) {
    match expr.node {
      hir::ExprAssign(ref l, _) => {
        this.check_place(&l);

        intravisit::walk_expr(this, expr);
      }

      hir::ExprAssignOp(_, ref l, _) => {
        if !this.tables.is_method_call(expr) {
            this.check_place(&l);
        }

        intravisit::walk_expr(this, expr);
      }

      hir::ExprInlineAsm(ref ia, ref outputs, ref inputs) => {
        for input in inputs {
          this.visit_expr(input);
        }

        // Output operands must be places
        for (o, output) in ia.outputs.iter().zip(outputs) {
          if !o.is_indirect {
            this.check_place(output);
          }
          this.visit_expr(output);
        }

        intravisit::walk_expr(this, expr);
      }

      // no correctness conditions related to liveness
      hir::ExprCall(..) | hir::ExprMethodCall(..) | hir::ExprIf(..) |
      hir::ExprMatch(..) | hir::ExprWhile(..) | hir::ExprLoop(..) |
      hir::ExprIndex(..) | hir::ExprField(..) |
      hir::ExprArray(..) | hir::ExprTup(..) | hir::ExprBinary(..) |
      hir::ExprCast(..) | hir::ExprUnary(..) | hir::ExprRet(..) |
      hir::ExprBreak(..) | hir::ExprAgain(..) | hir::ExprLit(_) |
      hir::ExprBlock(..) | hir::ExprAddrOf(..) |
      hir::ExprStruct(..) | hir::ExprRepeat(..) |
      hir::ExprClosure(..) | hir::ExprPath(_) | hir::ExprYield(..) |
      hir::ExprBox(..) | hir::ExprType(..) => {
        intravisit::walk_expr(this, expr);
      }
    }
}

impl<'a, 'tcx> Liveness<'a, 'tcx> {
    fn check_place(&mut self, expr: &'tcx Expr) {
        match expr.node {
            hir::ExprPath(hir::QPath::Resolved(_, ref path)) => {
                if let Def::Local(nid) = path.def {
                    // Assignment to an immutable variable or argument: only legal
                    // if there is no later assignment. If this local is actually
                    // mutable, then check for a reassignment to flag the mutability
                    // as being used.
                    let ln = self.live_node(expr.hir_id, expr.span);
                    let var_hid = self.ir.tcx.hir.node_to_hir_id(nid);
                    let var = self.variable(var_hid, expr.span);
                    self.warn_about_dead_assign(expr.span, expr.hir_id, ln, var);
                }
            }
            _ => {
                // For other kinds of places, no checks are required,
                // and any embedded expressions are actually rvalues
                intravisit::walk_expr(self, expr);
            }
        }
    }

    fn should_warn(&self, var: Variable) -> Option<String> {
        let name = self.ir.variable_name(var);
        if name.is_empty() || name.as_bytes()[0] == ('_' as u8) {
            None
        } else {
            Some(name)
        }
    }

    fn warn_about_unused_args(&self, body: &hir::Body, entry_ln: LiveNode) {
        for arg in &body.arguments {
            arg.pat.each_binding(|_bm, hir_id, _, path1| {
                let sp = path1.span;
                let var = self.variable(hir_id, sp);
                // Ignore unused self.
                let name = path1.node;
                if name != keywords::SelfValue.name() {
                    if !self.warn_about_unused(sp, hir_id, entry_ln, var) {
                        if self.live_on_entry(entry_ln, var).is_none() {
                            self.report_dead_assign(hir_id, sp, var, true);
                        }
                    }
                }
            })
        }
    }

    fn warn_about_unused_or_dead_vars_in_pat(&mut self, pat: &hir::Pat) {
        self.pat_bindings(pat, |this, ln, var, sp, id| {
            if !this.warn_about_unused(sp, id, ln, var) {
                this.warn_about_dead_assign(sp, id, ln, var);
            }
        })
    }

    fn warn_about_unused(&self,
                         sp: Span,
                         hir_id: HirId,
                         ln: LiveNode,
                         var: Variable)
                         -> bool {
        if !self.used_on_entry(ln, var) {
            let r = self.should_warn(var);
            if let Some(name) = r {

                // annoying: for parameters in funcs like `fn(x: i32)
                // {ret}`, there is only one node, so asking about
                // assigned_on_exit() is not meaningful.
                let is_assigned = if ln == self.s.exit_ln {
                    false
                } else {
                    self.assigned_on_exit(ln, var).is_some()
                };

                let suggest_underscore_msg = format!("consider using `_{}` instead",
                                                     name);

                if is_assigned {
                    self.ir.tcx
                        .lint_hir_note(lint::builtin::UNUSED_VARIABLES, hir_id, sp,
                                       &format!("variable `{}` is assigned to, but never used",
                                                name),
                                       &suggest_underscore_msg);
                } else if name != "self" {
                    let msg = format!("unused variable: `{}`", name);
                    let mut err = self.ir.tcx
                        .struct_span_lint_hir(lint::builtin::UNUSED_VARIABLES, hir_id, sp, &msg);
                    if self.ir.variable_is_shorthand(var) {
                        err.span_suggestion_with_applicability(sp, "try ignoring the field",
                                                               format!("{}: _", name),
                                                               Applicability::MachineApplicable);
                    } else {
                        err.span_suggestion_short_with_applicability(
                            sp, &suggest_underscore_msg,
                            format!("_{}", name),
                            Applicability::MachineApplicable,
                        );
                    }
                    err.emit()
                }
            }
            true
        } else {
            false
        }
    }

    fn warn_about_dead_assign(&self,
                              sp: Span,
                              hir_id: HirId,
                              ln: LiveNode,
                              var: Variable) {
        if self.live_on_exit(ln, var).is_none() {
            self.report_dead_assign(hir_id, sp, var, false);
        }
    }

    fn report_dead_assign(&self, hir_id: HirId, sp: Span, var: Variable, is_argument: bool) {
        if let Some(name) = self.should_warn(var) {
            if is_argument {
                self.ir.tcx.lint_hir(lint::builtin::UNUSED_ASSIGNMENTS, hir_id, sp,
                    &format!("value passed to `{}` is never read", name));
            } else {
                self.ir.tcx.lint_hir(lint::builtin::UNUSED_ASSIGNMENTS, hir_id, sp,
                    &format!("value assigned to `{}` is never read", name));
            }
        }
    }
}
