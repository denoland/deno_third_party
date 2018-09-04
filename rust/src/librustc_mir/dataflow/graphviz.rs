// Copyright 2012-2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Hook into libgraphviz for rendering dataflow graphs for MIR.

use syntax::ast::NodeId;
use rustc::mir::{BasicBlock, Mir};
use rustc_data_structures::bitslice::bits_to_string;
use rustc_data_structures::indexed_vec::Idx;

use dot;
use dot::IntoCow;

use std::fs;
use std::io;
use std::marker::PhantomData;
use std::path::Path;

use super::{BitDenotation, DataflowState};
use super::DataflowBuilder;
use super::DebugFormatted;

pub trait MirWithFlowState<'tcx> {
    type BD: BitDenotation;
    fn node_id(&self) -> NodeId;
    fn mir(&self) -> &Mir<'tcx>;
    fn flow_state(&self) -> &DataflowState<Self::BD>;
}

impl<'a, 'tcx: 'a, BD> MirWithFlowState<'tcx> for DataflowBuilder<'a, 'tcx, BD>
    where 'tcx: 'a, BD: BitDenotation
{
    type BD = BD;
    fn node_id(&self) -> NodeId { self.node_id }
    fn mir(&self) -> &Mir<'tcx> { self.flow_state.mir() }
    fn flow_state(&self) -> &DataflowState<Self::BD> { &self.flow_state.flow_state }
}

struct Graph<'a, 'tcx, MWF:'a, P> where
    MWF: MirWithFlowState<'tcx>
{
    mbcx: &'a MWF,
    phantom: PhantomData<&'tcx ()>,
    render_idx: P,
}

pub(crate) fn print_borrowck_graph_to<'a, 'tcx, BD, P>(
    mbcx: &DataflowBuilder<'a, 'tcx, BD>,
    path: &Path,
    render_idx: P)
    -> io::Result<()>
    where BD: BitDenotation,
          P: Fn(&BD, BD::Idx) -> DebugFormatted
{
    let g = Graph { mbcx, phantom: PhantomData, render_idx };
    let mut v = Vec::new();
    dot::render(&g, &mut v)?;
    debug!("print_borrowck_graph_to path: {} node_id: {}",
           path.display(), mbcx.node_id);
    fs::write(path, v)
}

pub type Node = BasicBlock;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Edge { source: BasicBlock, index: usize }

fn outgoing(mir: &Mir, bb: BasicBlock) -> Vec<Edge> {
    mir[bb].terminator().successors().enumerate()
        .map(|(index, _)| Edge { source: bb, index: index}).collect()
}

impl<'a, 'tcx, MWF, P> dot::Labeller<'a> for Graph<'a, 'tcx, MWF, P>
    where MWF: MirWithFlowState<'tcx>,
          P: Fn(&MWF::BD, <MWF::BD as BitDenotation>::Idx) -> DebugFormatted,
{
    type Node = Node;
    type Edge = Edge;
    fn graph_id(&self) -> dot::Id {
        dot::Id::new(format!("graph_for_node_{}",
                             self.mbcx.node_id()))
            .unwrap()
    }

    fn node_id(&self, n: &Node) -> dot::Id {
        dot::Id::new(format!("bb_{}", n.index()))
            .unwrap()
    }

    fn node_label(&self, n: &Node) -> dot::LabelText {
        // Node label is something like this:
        // +---------+----------------------------------+------------------+------------------+
        // | ENTRY   | MIR                              | GEN              | KILL             |
        // +---------+----------------------------------+------------------+------------------+
        // |         |  0: StorageLive(_7)              | bb3[2]: reserved | bb2[0]: reserved |
        // |         |  1: StorageLive(_8)              | bb3[2]: active   | bb2[0]: active   |
        // |         |  2: _8 = &mut _1                 |                  | bb4[2]: reserved |
        // |         |                                  |                  | bb4[2]: active   |
        // |         |                                  |                  | bb9[0]: reserved |
        // |         |                                  |                  | bb9[0]: active   |
        // |         |                                  |                  | bb10[0]: reserved|
        // |         |                                  |                  | bb10[0]: active  |
        // |         |                                  |                  | bb11[0]: reserved|
        // |         |                                  |                  | bb11[0]: active  |
        // +---------+----------------------------------+------------------+------------------+
        // | [00-00] | _7 = const Foo::twiddle(move _8) | [0c-00]          | [f3-0f]          |
        // +---------+----------------------------------+------------------+------------------+
        let mut v = Vec::new();
        self.node_label_internal(n, &mut v, *n, self.mbcx.mir()).unwrap();
        dot::LabelText::html(String::from_utf8(v).unwrap())
    }


    fn node_shape(&self, _n: &Node) -> Option<dot::LabelText> {
        Some(dot::LabelText::label("none"))
    }

    fn edge_label(&'a self, e: &Edge) -> dot::LabelText<'a> {
        let term = self.mbcx.mir()[e.source].terminator();
        let label = &term.kind.fmt_successor_labels()[e.index];
        dot::LabelText::label(label.clone())
    }
}

impl<'a, 'tcx, MWF, P> Graph<'a, 'tcx, MWF, P>
where MWF: MirWithFlowState<'tcx>,
      P: Fn(&MWF::BD, <MWF::BD as BitDenotation>::Idx) -> DebugFormatted,
{
    /// Generate the node label
    fn node_label_internal<W: io::Write>(&self,
                                         n: &Node,
                                         w: &mut W,
                                         block: BasicBlock,
                                         mir: &Mir) -> io::Result<()> {
        // Header rows
        const HDRS: [&'static str; 4] = ["ENTRY", "MIR", "BLOCK GENS", "BLOCK KILLS"];
        const HDR_FMT: &'static str = "bgcolor=\"grey\"";
        write!(w, "<table><tr><td rowspan=\"{}\">", HDRS.len())?;
        write!(w, "{:?}", block.index())?;
        write!(w, "</td></tr><tr>")?;
        for hdr in &HDRS {
            write!(w, "<td {}>{}</td>", HDR_FMT, hdr)?;
        }
        write!(w, "</tr>")?;

        // Data row
        self.node_label_verbose_row(n, w, block, mir)?;
        self.node_label_final_row(n, w, block, mir)?;
        write!(w, "</table>")?;

        Ok(())
    }

    /// Build the verbose row: full MIR data, and detailed gen/kill/entry sets
    fn node_label_verbose_row<W: io::Write>(&self,
                                            n: &Node,
                                            w: &mut W,
                                            block: BasicBlock,
                                            mir: &Mir)
                                            -> io::Result<()> {
        let i = n.index();

        macro_rules! dump_set_for {
            ($set:ident) => {
                write!(w, "<td>")?;

                let flow = self.mbcx.flow_state();
                let entry_interp = flow.interpret_set(&flow.operator,
                                                      flow.sets.$set(i),
                                                      &self.render_idx);
                for e in &entry_interp {
                    write!(w, "{:?}<br/>", e)?;
                }
                write!(w, "</td>")?;
            }
        }

        write!(w, "<tr>")?;
        // Entry
        dump_set_for!(on_entry_set_for);

        // MIR statements
        write!(w, "<td>")?;
        {
            let data = &mir[block];
            for (i, statement) in data.statements.iter().enumerate() {
                write!(w, "{}<br align=\"left\"/>",
                       dot::escape_html(&format!("{:3}: {:?}", i, statement)))?;
            }
        }
        write!(w, "</td>")?;

        // Gen
        dump_set_for!(gen_set_for);

        // Kill
        dump_set_for!(kill_set_for);

        write!(w, "</tr>")?;

        Ok(())
    }

    /// Build the summary row: terminator, gen/kill/entry bit sets
    fn node_label_final_row<W: io::Write>(&self,
                                          n: &Node,
                                          w: &mut W,
                                          block: BasicBlock,
                                          mir: &Mir)
                                          -> io::Result<()> {
        let i = n.index();

        macro_rules! dump_set_for {
            ($set:ident) => {
                let flow = self.mbcx.flow_state();
                let bits_per_block = flow.sets.bits_per_block();
                let set = flow.sets.$set(i);
                write!(w, "<td>{:?}</td>",
                       dot::escape_html(&bits_to_string(set.words(), bits_per_block)))?;
            }
        }

        write!(w, "<tr>")?;
        // Entry
        dump_set_for!(on_entry_set_for);

        // Terminator
        write!(w, "<td>")?;
        {
            let data = &mir[block];
            let mut terminator_head = String::new();
            data.terminator().kind.fmt_head(&mut terminator_head).unwrap();
            write!(w, "{}", dot::escape_html(&terminator_head))?;
        }
        write!(w, "</td>")?;

        // Gen
        dump_set_for!(gen_set_for);

        // Kill
        dump_set_for!(kill_set_for);

        write!(w, "</tr>")?;

        Ok(())
    }
}

impl<'a, 'tcx, MWF, P> dot::GraphWalk<'a> for Graph<'a, 'tcx, MWF, P>
    where MWF: MirWithFlowState<'tcx>
{
    type Node = Node;
    type Edge = Edge;
    fn nodes(&self) -> dot::Nodes<Node> {
        self.mbcx.mir()
            .basic_blocks()
            .indices()
            .collect::<Vec<_>>()
            .into_cow()
    }

    fn edges(&self) -> dot::Edges<Edge> {
        let mir = self.mbcx.mir();
        // base initial capacity on assumption every block has at
        // least one outgoing edge (Which should be true for all
        // blocks but one, the exit-block).
        let mut edges = Vec::with_capacity(mir.basic_blocks().len());
        for bb in mir.basic_blocks().indices() {
            let outgoing = outgoing(mir, bb);
            edges.extend(outgoing.into_iter());
        }
        edges.into_cow()
    }

    fn source(&self, edge: &Edge) -> Node {
        edge.source
    }

    fn target(&self, edge: &Edge) -> Node {
        let mir = self.mbcx.mir();
        *mir[edge.source].terminator().successors().nth(edge.index).unwrap()
    }
}
