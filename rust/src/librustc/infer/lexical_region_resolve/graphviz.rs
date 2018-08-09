// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! This module provides linkage between libgraphviz traits and
//! `rustc::middle::typeck::infer::region_constraints`, generating a
//! rendering of the graph represented by the list of `Constraint`
//! instances (which make up the edges of the graph), as well as the
//! origin for each constraint (which are attached to the labels on
//! each edge).

/// For clarity, rename the graphviz crate locally to dot.
use graphviz as dot;

use hir::def_id::DefIndex;
use ty;
use middle::free_region::RegionRelations;
use middle::region;
use super::Constraint;
use infer::SubregionOrigin;
use infer::region_constraints::RegionConstraintData;
use util::nodemap::{FxHashMap, FxHashSet};

use std::borrow::Cow;
use std::collections::hash_map::Entry::Vacant;
use std::collections::btree_map::BTreeMap;
use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

fn print_help_message() {
    println!("\
-Z print-region-graph by default prints a region constraint graph for every \n\
function body, to the path `/tmp/constraints.nodeXXX.dot`, where the XXX is \n\
replaced with the node id of the function under analysis.                   \n\
                                                                            \n\
To select one particular function body, set `RUST_REGION_GRAPH_NODE=XXX`,   \n\
where XXX is the node id desired.                                           \n\
                                                                            \n\
To generate output to some path other than the default                      \n\
`/tmp/constraints.nodeXXX.dot`, set `RUST_REGION_GRAPH=/path/desired.dot`;  \n\
occurrences of the character `%` in the requested path will be replaced with\n\
the node id of the function under analysis.                                 \n\
                                                                            \n\
(Since you requested help via RUST_REGION_GRAPH=help, no region constraint  \n\
graphs will be printed.                                                     \n\
");
}

pub fn maybe_print_constraints_for<'a, 'gcx, 'tcx>(
    region_data: &RegionConstraintData<'tcx>,
    region_rels: &RegionRelations<'a, 'gcx, 'tcx>)
{
    let tcx = region_rels.tcx;
    let context = region_rels.context;

    if !tcx.sess.opts.debugging_opts.print_region_graph {
        return;
    }

    let requested_node = env::var("RUST_REGION_GRAPH_NODE")
        .ok().and_then(|s| s.parse().map(DefIndex::from_raw_u32).ok());

    if requested_node.is_some() && requested_node != Some(context.index) {
        return;
    }

    let requested_output = env::var("RUST_REGION_GRAPH");
    debug!("requested_output: {:?} requested_node: {:?}",
           requested_output,
           requested_node);

    let output_path = {
        let output_template = match requested_output {
            Ok(ref s) if s == "help" => {
                static PRINTED_YET: AtomicBool = AtomicBool::new(false);
                if !PRINTED_YET.load(Ordering::SeqCst) {
                    print_help_message();
                    PRINTED_YET.store(true, Ordering::SeqCst);
                }
                return;
            }

            Ok(other_path) => other_path,
            Err(_) => "/tmp/constraints.node%.dot".to_string(),
        };

        if output_template.is_empty() {
            panic!("empty string provided as RUST_REGION_GRAPH");
        }

        if output_template.contains('%') {
            let mut new_str = String::new();
            for c in output_template.chars() {
                if c == '%' {
                    new_str.push_str(&context.index.as_raw_u32().to_string());
                } else {
                    new_str.push(c);
                }
            }
            new_str
        } else {
            output_template
        }
    };

    match dump_region_data_to(region_rels, &region_data.constraints, &output_path) {
        Ok(()) => {}
        Err(e) => {
            let msg = format!("io error dumping region constraints: {}", e);
            tcx.sess.err(&msg)
        }
    }
}

struct ConstraintGraph<'a, 'gcx: 'a+'tcx, 'tcx: 'a> {
    graph_name: String,
    region_rels: &'a RegionRelations<'a, 'gcx, 'tcx>,
    map: &'a BTreeMap<Constraint<'tcx>, SubregionOrigin<'tcx>>,
    node_ids: FxHashMap<Node, usize>,
}

#[derive(Clone, Hash, PartialEq, Eq, Debug, Copy)]
enum Node {
    RegionVid(ty::RegionVid),
    Region(ty::RegionKind),
}

#[derive(Clone, PartialEq, Eq, Debug, Copy)]
enum Edge<'tcx> {
    Constraint(Constraint<'tcx>),
    EnclScope(region::Scope, region::Scope),
}

impl<'a, 'gcx, 'tcx> ConstraintGraph<'a, 'gcx, 'tcx> {
    fn new(name: String,
           region_rels: &'a RegionRelations<'a, 'gcx, 'tcx>,
           map: &'a ConstraintMap<'tcx>)
           -> ConstraintGraph<'a, 'gcx, 'tcx> {
        let mut i = 0;
        let mut node_ids = FxHashMap();
        {
            let mut add_node = |node| {
                if let Vacant(e) = node_ids.entry(node) {
                    e.insert(i);
                    i += 1;
                }
            };

            for (n1, n2) in map.keys().map(|c| constraint_to_nodes(c)) {
                add_node(n1);
                add_node(n2);
            }

            region_rels.region_scope_tree.each_encl_scope(|sub, sup| {
                add_node(Node::Region(ty::ReScope(sub)));
                add_node(Node::Region(ty::ReScope(sup)));
            });
        }

        ConstraintGraph {
            map,
            node_ids,
            region_rels,
            graph_name: name,
        }
    }
}

impl<'a, 'gcx, 'tcx> dot::Labeller<'a> for ConstraintGraph<'a, 'gcx, 'tcx> {
    type Node = Node;
    type Edge = Edge<'tcx>;
    fn graph_id(&self) -> dot::Id {
        dot::Id::new(&*self.graph_name).unwrap()
    }
    fn node_id(&self, n: &Node) -> dot::Id {
        let node_id = match self.node_ids.get(n) {
            Some(node_id) => node_id,
            None => bug!("no node_id found for node: {:?}", n),
        };
        let name = || format!("node_{}", node_id);
        match dot::Id::new(name()) {
            Ok(id) => id,
            Err(_) => {
                bug!("failed to create graphviz node identified by {}", name());
            }
        }
    }
    fn node_label(&self, n: &Node) -> dot::LabelText {
        match *n {
            Node::RegionVid(n_vid) => dot::LabelText::label(format!("{:?}", n_vid)),
            Node::Region(n_rgn) => dot::LabelText::label(format!("{:?}", n_rgn)),
        }
    }
    fn edge_label(&self, e: &Edge) -> dot::LabelText {
        match *e {
            Edge::Constraint(ref c) =>
                dot::LabelText::label(format!("{:?}", self.map.get(c).unwrap())),
            Edge::EnclScope(..) => dot::LabelText::label(format!("(enclosed)")),
        }
    }
}

fn constraint_to_nodes(c: &Constraint) -> (Node, Node) {
    match *c {
        Constraint::VarSubVar(rv_1, rv_2) =>
            (Node::RegionVid(rv_1), Node::RegionVid(rv_2)),
        Constraint::RegSubVar(r_1, rv_2) =>
            (Node::Region(*r_1), Node::RegionVid(rv_2)),
        Constraint::VarSubReg(rv_1, r_2) =>
            (Node::RegionVid(rv_1), Node::Region(*r_2)),
        Constraint::RegSubReg(r_1, r_2) =>
            (Node::Region(*r_1), Node::Region(*r_2)),
    }
}

fn edge_to_nodes(e: &Edge) -> (Node, Node) {
    match *e {
        Edge::Constraint(ref c) => constraint_to_nodes(c),
        Edge::EnclScope(sub, sup) => {
            (Node::Region(ty::ReScope(sub)),
             Node::Region(ty::ReScope(sup)))
        }
    }
}

impl<'a, 'gcx, 'tcx> dot::GraphWalk<'a> for ConstraintGraph<'a, 'gcx, 'tcx> {
    type Node = Node;
    type Edge = Edge<'tcx>;
    fn nodes(&self) -> dot::Nodes<Node> {
        let mut set = FxHashSet();
        for node in self.node_ids.keys() {
            set.insert(*node);
        }
        debug!("constraint graph has {} nodes", set.len());
        set.into_iter().collect()
    }
    fn edges(&self) -> dot::Edges<Edge<'tcx>> {
        debug!("constraint graph has {} edges", self.map.len());
        let mut v: Vec<_> = self.map.keys().map(|e| Edge::Constraint(*e)).collect();
        self.region_rels.region_scope_tree.each_encl_scope(|sub, sup| {
            v.push(Edge::EnclScope(sub, sup))
        });
        debug!("region graph has {} edges", v.len());
        Cow::Owned(v)
    }
    fn source(&self, edge: &Edge<'tcx>) -> Node {
        let (n1, _) = edge_to_nodes(edge);
        debug!("edge {:?} has source {:?}", edge, n1);
        n1
    }
    fn target(&self, edge: &Edge<'tcx>) -> Node {
        let (_, n2) = edge_to_nodes(edge);
        debug!("edge {:?} has target {:?}", edge, n2);
        n2
    }
}

pub type ConstraintMap<'tcx> = BTreeMap<Constraint<'tcx>, SubregionOrigin<'tcx>>;

fn dump_region_data_to<'a, 'gcx, 'tcx>(region_rels: &RegionRelations<'a, 'gcx, 'tcx>,
                                       map: &ConstraintMap<'tcx>,
                                       path: &str)
                                       -> io::Result<()> {
    debug!("dump_region_data map (len: {}) path: {}",
           map.len(),
           path);
    let g = ConstraintGraph::new(format!("region_data"), region_rels, map);
    debug!("dump_region_data calling render");
    let mut v = Vec::new();
    dot::render(&g, &mut v).unwrap();
    File::create(path).and_then(|mut f| f.write_all(&v))
}
