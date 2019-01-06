use graphviz as dot;
use obligation_forest::{ForestObligation, ObligationForest};
use std::env::var_os;
use std::fs::File;
use std::path::Path;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

impl<O: ForestObligation> ObligationForest<O> {
    /// Create a graphviz representation of the obligation forest.  Given a directory this will
    /// create files with name of the format `<counter>_<description>.gv`.  The counter is
    /// global and is maintained internally.
    ///
    /// Calling this will do nothing unless the environment variable
    /// `DUMP_OBLIGATION_FOREST_GRAPHVIZ` is defined.
    ///
    /// A few post-processing that you might want to do make the forest easier to visualize:
    ///
    ///  * `sed 's,std::[a-z]*::,,g'` — Deletes the `std::<package>::` prefix of paths.
    ///  * `sed 's,"Binder(TraitPredicate(<\(.*\)>)) (\([^)]*\))","\1 (\2)",'` — Transforms
    ///    `Binder(TraitPredicate(<predicate>))` into just `<predicate>`.
    #[allow(dead_code)]
    pub fn dump_graphviz<P: AsRef<Path>>(&self, dir: P, description: &str) {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        if var_os("DUMP_OBLIGATION_FOREST_GRAPHVIZ").is_none() {
            return;
        }

        let counter = COUNTER.fetch_add(1, Ordering::AcqRel);

        let file_path = dir.as_ref().join(format!("{:010}_{}.gv", counter, description));

        let mut gv_file = File::create(file_path).unwrap();

        dot::render(&self, &mut gv_file).unwrap();
    }
}

impl<'a, O: ForestObligation + 'a> dot::Labeller<'a> for &'a ObligationForest<O> {
    type Node = usize;
    type Edge = (usize, usize);

    fn graph_id(&self) -> dot::Id {
        dot::Id::new("trait_obligation_forest").unwrap()
    }

    fn node_id(&self, index: &Self::Node) -> dot::Id {
        dot::Id::new(format!("obligation_{}", index)).unwrap()
    }

    fn node_label(&self, index: &Self::Node) -> dot::LabelText {
        let node = &self.nodes[*index];
        let label = format!("{:?} ({:?})", node.obligation.as_predicate(), node.state.get());

        dot::LabelText::LabelStr(label.into())
    }

    fn edge_label(&self, (_index_source, _index_target): &Self::Edge) -> dot::LabelText {
        dot::LabelText::LabelStr("".into())
    }
}

impl<'a, O: ForestObligation + 'a> dot::GraphWalk<'a> for &'a ObligationForest<O> {
    type Node = usize;
    type Edge = (usize, usize);

    fn nodes(&self) -> dot::Nodes<Self::Node> {
        (0..self.nodes.len()).collect()
    }

    fn edges(&self) -> dot::Edges<Self::Edge> {
        (0..self.nodes.len())
            .flat_map(|i| {
                let node = &self.nodes[i];

                node.parent.iter().map(|p| p.get())
                    .chain(node.dependents.iter().map(|p| p.get()))
                    .map(move |p| (p, i))
            })
            .collect()
    }

    fn source(&self, (s, _): &Self::Edge) -> Self::Node {
        *s
    }

    fn target(&self, (_, t): &Self::Edge) -> Self::Node {
        *t
    }
}
