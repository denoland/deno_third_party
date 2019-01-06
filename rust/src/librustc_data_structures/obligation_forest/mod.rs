//! The `ObligationForest` is a utility data structure used in trait
//! matching to track the set of outstanding obligations (those not
//! yet resolved to success or error). It also tracks the "backtrace"
//! of each pending obligation (why we are trying to figure this out
//! in the first place). See README.md for a general overview of how
//! to use this class.

use fx::{FxHashMap, FxHashSet};

use std::cell::Cell;
use std::collections::hash_map::Entry;
use std::fmt::Debug;
use std::hash;
use std::marker::PhantomData;

mod node_index;
use self::node_index::NodeIndex;

mod graphviz;

#[cfg(test)]
mod test;

pub trait ForestObligation : Clone + Debug {
    type Predicate : Clone + hash::Hash + Eq + Debug;

    fn as_predicate(&self) -> &Self::Predicate;
}

pub trait ObligationProcessor {
    type Obligation : ForestObligation;
    type Error : Debug;

    fn process_obligation(&mut self,
                          obligation: &mut Self::Obligation)
                          -> ProcessResult<Self::Obligation, Self::Error>;

    /// As we do the cycle check, we invoke this callback when we
    /// encounter an actual cycle. `cycle` is an iterator that starts
    /// at the start of the cycle in the stack and walks **toward the
    /// top**.
    ///
    /// In other words, if we had O1 which required O2 which required
    /// O3 which required O1, we would give an iterator yielding O1,
    /// O2, O3 (O1 is not yielded twice).
    fn process_backedge<'c, I>(&mut self,
                               cycle: I,
                               _marker: PhantomData<&'c Self::Obligation>)
        where I: Clone + Iterator<Item=&'c Self::Obligation>;
}

/// The result type used by `process_obligation`.
#[derive(Debug)]
pub enum ProcessResult<O, E> {
    Unchanged,
    Changed(Vec<O>),
    Error(E),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct ObligationTreeId(usize);

type ObligationTreeIdGenerator =
    ::std::iter::Map<::std::ops::RangeFrom<usize>, fn(usize) -> ObligationTreeId>;

pub struct ObligationForest<O: ForestObligation> {
    /// The list of obligations. In between calls to
    /// `process_obligations`, this list only contains nodes in the
    /// `Pending` or `Success` state (with a non-zero number of
    /// incomplete children). During processing, some of those nodes
    /// may be changed to the error state, or we may find that they
    /// are completed (That is, `num_incomplete_children` drops to 0).
    /// At the end of processing, those nodes will be removed by a
    /// call to `compress`.
    ///
    /// At all times we maintain the invariant that every node appears
    /// at a higher index than its parent. This is needed by the
    /// backtrace iterator (which uses `split_at`).
    nodes: Vec<Node<O>>,

    /// A cache of predicates that have been successfully completed.
    done_cache: FxHashSet<O::Predicate>,

    /// An cache of the nodes in `nodes`, indexed by predicate.
    waiting_cache: FxHashMap<O::Predicate, NodeIndex>,

    scratch: Option<Vec<usize>>,

    obligation_tree_id_generator: ObligationTreeIdGenerator,

    /// Per tree error cache.  This is used to deduplicate errors,
    /// which is necessary to avoid trait resolution overflow in
    /// some cases.
    ///
    /// See [this][details] for details.
    ///
    /// [details]: https://github.com/rust-lang/rust/pull/53255#issuecomment-421184780
    error_cache: FxHashMap<ObligationTreeId, FxHashSet<O::Predicate>>,
}

#[derive(Debug)]
struct Node<O> {
    obligation: O,
    state: Cell<NodeState>,

    /// The parent of a node - the original obligation of
    /// which it is a subobligation. Except for error reporting,
    /// it is just like any member of `dependents`.
    parent: Option<NodeIndex>,

    /// Obligations that depend on this obligation for their
    /// completion. They must all be in a non-pending state.
    dependents: Vec<NodeIndex>,

    /// Identifier of the obligation tree to which this node belongs.
    obligation_tree_id: ObligationTreeId,
}

/// The state of one node in some tree within the forest. This
/// represents the current state of processing for the obligation (of
/// type `O`) associated with this node.
///
/// Outside of ObligationForest methods, nodes should be either Pending
/// or Waiting.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum NodeState {
    /// Obligations for which selection had not yet returned a
    /// non-ambiguous result.
    Pending,

    /// This obligation was selected successfully, but may or
    /// may not have subobligations.
    Success,

    /// This obligation was selected successfully, but it has
    /// a pending subobligation.
    Waiting,

    /// This obligation, along with its subobligations, are complete,
    /// and will be removed in the next collection.
    Done,

    /// This obligation was resolved to an error. Error nodes are
    /// removed from the vector by the compression step.
    Error,

    /// This is a temporary state used in DFS loops to detect cycles,
    /// it should not exist outside of these DFSes.
    OnDfsStack,
}

#[derive(Debug)]
pub struct Outcome<O, E> {
    /// Obligations that were completely evaluated, including all
    /// (transitive) subobligations. Only computed if requested.
    pub completed: Option<Vec<O>>,

    /// Backtrace of obligations that were found to be in error.
    pub errors: Vec<Error<O, E>>,

    /// If true, then we saw no successful obligations, which means
    /// there is no point in further iteration. This is based on the
    /// assumption that when trait matching returns `Error` or
    /// `Unchanged`, those results do not affect environmental
    /// inference state. (Note that if we invoke `process_obligations`
    /// with no pending obligations, stalled will be true.)
    pub stalled: bool,
}

/// Should `process_obligations` compute the `Outcome::completed` field of its
/// result?
#[derive(PartialEq)]
pub enum DoCompleted {
    No,
    Yes,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Error<O, E> {
    pub error: E,
    pub backtrace: Vec<O>,
}

impl<O: ForestObligation> ObligationForest<O> {
    pub fn new() -> ObligationForest<O> {
        ObligationForest {
            nodes: vec![],
            done_cache: Default::default(),
            waiting_cache: Default::default(),
            scratch: Some(vec![]),
            obligation_tree_id_generator: (0..).map(|i| ObligationTreeId(i)),
            error_cache: Default::default(),
        }
    }

    /// Return the total number of nodes in the forest that have not
    /// yet been fully resolved.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Registers an obligation
    ///
    /// This CAN be done in a snapshot
    pub fn register_obligation(&mut self, obligation: O) {
        // Ignore errors here - there is no guarantee of success.
        let _ = self.register_obligation_at(obligation, None);
    }

    // returns Err(()) if we already know this obligation failed.
    fn register_obligation_at(&mut self, obligation: O, parent: Option<NodeIndex>)
                              -> Result<(), ()>
    {
        if self.done_cache.contains(obligation.as_predicate()) {
            return Ok(());
        }

        match self.waiting_cache.entry(obligation.as_predicate().clone()) {
            Entry::Occupied(o) => {
                debug!("register_obligation_at({:?}, {:?}) - duplicate of {:?}!",
                       obligation, parent, o.get());
                let node = &mut self.nodes[o.get().get()];
                if let Some(parent) = parent {
                    // If the node is already in `waiting_cache`, it's already
                    // been marked with a parent. (It's possible that parent
                    // has been cleared by `apply_rewrites`, though.) So just
                    // dump `parent` into `node.dependents`... unless it's
                    // already in `node.dependents` or `node.parent`.
                    if !node.dependents.contains(&parent) && Some(parent) != node.parent {
                        node.dependents.push(parent);
                    }
                }
                if let NodeState::Error = node.state.get() {
                    Err(())
                } else {
                    Ok(())
                }
            }
            Entry::Vacant(v) => {
                debug!("register_obligation_at({:?}, {:?}) - ok, new index is {}",
                       obligation, parent, self.nodes.len());

                let obligation_tree_id = match parent {
                    Some(p) => {
                        let parent_node = &self.nodes[p.get()];
                        parent_node.obligation_tree_id
                    }
                    None => self.obligation_tree_id_generator.next().unwrap()
                };

                let already_failed =
                    parent.is_some()
                        && self.error_cache
                            .get(&obligation_tree_id)
                            .map(|errors| errors.contains(obligation.as_predicate()))
                            .unwrap_or(false);

                if already_failed {
                    Err(())
                } else {
                    v.insert(NodeIndex::new(self.nodes.len()));
                    self.nodes.push(Node::new(parent, obligation, obligation_tree_id));
                    Ok(())
                }
            }
        }
    }

    /// Convert all remaining obligations to the given error.
    ///
    /// This cannot be done during a snapshot.
    pub fn to_errors<E: Clone>(&mut self, error: E) -> Vec<Error<O, E>> {
        let mut errors = vec![];
        for index in 0..self.nodes.len() {
            if let NodeState::Pending = self.nodes[index].state.get() {
                let backtrace = self.error_at(index);
                errors.push(Error {
                    error: error.clone(),
                    backtrace,
                });
            }
        }
        let successful_obligations = self.compress(DoCompleted::Yes);
        assert!(successful_obligations.unwrap().is_empty());
        errors
    }

    /// Returns the set of obligations that are in a pending state.
    pub fn map_pending_obligations<P, F>(&self, f: F) -> Vec<P>
        where F: Fn(&O) -> P
    {
        self.nodes
            .iter()
            .filter(|n| n.state.get() == NodeState::Pending)
            .map(|n| f(&n.obligation))
            .collect()
    }

    fn insert_into_error_cache(&mut self, node_index: usize) {
        let node = &self.nodes[node_index];

        self.error_cache
            .entry(node.obligation_tree_id)
            .or_default()
            .insert(node.obligation.as_predicate().clone());
    }

    /// Perform a pass through the obligation list. This must
    /// be called in a loop until `outcome.stalled` is false.
    ///
    /// This CANNOT be unrolled (presently, at least).
    pub fn process_obligations<P>(&mut self, processor: &mut P, do_completed: DoCompleted)
                                  -> Outcome<O, P::Error>
        where P: ObligationProcessor<Obligation=O>
    {
        debug!("process_obligations(len={})", self.nodes.len());

        let mut errors = vec![];
        let mut stalled = true;

        for index in 0..self.nodes.len() {
            debug!("process_obligations: node {} == {:?}", index, self.nodes[index]);

            let result = match self.nodes[index] {
                Node { ref state, ref mut obligation, .. } if state.get() == NodeState::Pending =>
                    processor.process_obligation(obligation),
                _ => continue
            };

            debug!("process_obligations: node {} got result {:?}", index, result);

            match result {
                ProcessResult::Unchanged => {
                    // No change in state.
                }
                ProcessResult::Changed(children) => {
                    // We are not (yet) stalled.
                    stalled = false;
                    self.nodes[index].state.set(NodeState::Success);

                    for child in children {
                        let st = self.register_obligation_at(
                            child,
                            Some(NodeIndex::new(index))
                        );
                        if let Err(()) = st {
                            // error already reported - propagate it
                            // to our node.
                            self.error_at(index);
                        }
                    }
                }
                ProcessResult::Error(err) => {
                    stalled = false;
                    let backtrace = self.error_at(index);
                    errors.push(Error {
                        error: err,
                        backtrace,
                    });
                }
            }
        }

        if stalled {
            // There's no need to perform marking, cycle processing and compression when nothing
            // changed.
            return Outcome {
                completed: if do_completed == DoCompleted::Yes { Some(vec![]) } else { None },
                errors,
                stalled,
            };
        }

        self.mark_as_waiting();
        self.process_cycles(processor);

        // Now we have to compress the result
        let completed = self.compress(do_completed);

        debug!("process_obligations: complete");

        Outcome {
            completed,
            errors,
            stalled,
        }
    }

    /// Mark all NodeState::Success nodes as NodeState::Done and
    /// report all cycles between them. This should be called
    /// after `mark_as_waiting` marks all nodes with pending
    /// subobligations as NodeState::Waiting.
    fn process_cycles<P>(&mut self, processor: &mut P)
        where P: ObligationProcessor<Obligation=O>
    {
        let mut stack = self.scratch.take().unwrap();
        debug_assert!(stack.is_empty());

        debug!("process_cycles()");

        for index in 0..self.nodes.len() {
            // For rustc-benchmarks/inflate-0.1.0 this state test is extremely
            // hot and the state is almost always `Pending` or `Waiting`. It's
            // a win to handle the no-op cases immediately to avoid the cost of
            // the function call.
            let state = self.nodes[index].state.get();
            match state {
                NodeState::Waiting | NodeState::Pending | NodeState::Done | NodeState::Error => {},
                _ => self.find_cycles_from_node(&mut stack, processor, index),
            }
        }

        debug!("process_cycles: complete");

        debug_assert!(stack.is_empty());
        self.scratch = Some(stack);
    }

    fn find_cycles_from_node<P>(&self, stack: &mut Vec<usize>,
                                processor: &mut P, index: usize)
        where P: ObligationProcessor<Obligation=O>
    {
        let node = &self.nodes[index];
        let state = node.state.get();
        match state {
            NodeState::OnDfsStack => {
                let index =
                    stack.iter().rposition(|n| *n == index).unwrap();
                processor.process_backedge(stack[index..].iter().map(GetObligation(&self.nodes)),
                                           PhantomData);
            }
            NodeState::Success => {
                node.state.set(NodeState::OnDfsStack);
                stack.push(index);
                for dependent in node.parent.iter().chain(node.dependents.iter()) {
                    self.find_cycles_from_node(stack, processor, dependent.get());
                }
                stack.pop();
                node.state.set(NodeState::Done);
            },
            NodeState::Waiting | NodeState::Pending => {
                // this node is still reachable from some pending node. We
                // will get to it when they are all processed.
            }
            NodeState::Done | NodeState::Error => {
                // already processed that node
            }
        };
    }

    /// Returns a vector of obligations for `p` and all of its
    /// ancestors, putting them into the error state in the process.
    fn error_at(&mut self, p: usize) -> Vec<O> {
        let mut error_stack = self.scratch.take().unwrap();
        let mut trace = vec![];

        let mut n = p;
        loop {
            self.nodes[n].state.set(NodeState::Error);
            trace.push(self.nodes[n].obligation.clone());
            error_stack.extend(self.nodes[n].dependents.iter().map(|x| x.get()));

            // loop to the parent
            match self.nodes[n].parent {
                Some(q) => n = q.get(),
                None => break
            }
        }

        while let Some(i) = error_stack.pop() {
            match self.nodes[i].state.get() {
                NodeState::Error => continue,
                _ => self.nodes[i].state.set(NodeState::Error),
            }

            let node = &self.nodes[i];

            error_stack.extend(
                node.parent.iter().chain(node.dependents.iter()).map(|x| x.get())
            );
        }

        self.scratch = Some(error_stack);
        trace
    }

    #[inline]
    fn mark_neighbors_as_waiting_from(&self, node: &Node<O>) {
        for dependent in node.parent.iter().chain(node.dependents.iter()) {
            self.mark_as_waiting_from(&self.nodes[dependent.get()]);
        }
    }

    /// Marks all nodes that depend on a pending node as NodeState::Waiting.
    fn mark_as_waiting(&self) {
        for node in &self.nodes {
            if node.state.get() == NodeState::Waiting {
                node.state.set(NodeState::Success);
            }
        }

        for node in &self.nodes {
            if node.state.get() == NodeState::Pending {
                self.mark_neighbors_as_waiting_from(node);
            }
        }
    }

    fn mark_as_waiting_from(&self, node: &Node<O>) {
        match node.state.get() {
            NodeState::Waiting | NodeState::Error | NodeState::OnDfsStack => return,
            NodeState::Success => node.state.set(NodeState::Waiting),
            NodeState::Pending | NodeState::Done => {},
        }

        self.mark_neighbors_as_waiting_from(node);
    }

    /// Compresses the vector, removing all popped nodes. This adjusts
    /// the indices and hence invalidates any outstanding
    /// indices. Cannot be used during a transaction.
    ///
    /// Beforehand, all nodes must be marked as `Done` and no cycles
    /// on these nodes may be present. This is done by e.g., `process_cycles`.
    #[inline(never)]
    fn compress(&mut self, do_completed: DoCompleted) -> Option<Vec<O>> {
        let nodes_len = self.nodes.len();
        let mut node_rewrites: Vec<_> = self.scratch.take().unwrap();
        node_rewrites.extend(0..nodes_len);
        let mut dead_nodes = 0;

        // Now move all popped nodes to the end. Try to keep the order.
        //
        // LOOP INVARIANT:
        //     self.nodes[0..i - dead_nodes] are the first remaining nodes
        //     self.nodes[i - dead_nodes..i] are all dead
        //     self.nodes[i..] are unchanged
        for i in 0..self.nodes.len() {
            match self.nodes[i].state.get() {
                NodeState::Pending | NodeState::Waiting => {
                    if dead_nodes > 0 {
                        self.nodes.swap(i, i - dead_nodes);
                        node_rewrites[i] -= dead_nodes;
                    }
                }
                NodeState::Done => {
                    // Avoid cloning the key (predicate) in case it exists in the waiting cache
                    if let Some((predicate, _)) = self.waiting_cache
                        .remove_entry(self.nodes[i].obligation.as_predicate())
                    {
                        self.done_cache.insert(predicate);
                    } else {
                        self.done_cache.insert(self.nodes[i].obligation.as_predicate().clone());
                    }
                    node_rewrites[i] = nodes_len;
                    dead_nodes += 1;
                }
                NodeState::Error => {
                    // We *intentionally* remove the node from the cache at this point. Otherwise
                    // tests must come up with a different type on every type error they
                    // check against.
                    self.waiting_cache.remove(self.nodes[i].obligation.as_predicate());
                    node_rewrites[i] = nodes_len;
                    dead_nodes += 1;
                    self.insert_into_error_cache(i);
                }
                NodeState::OnDfsStack | NodeState::Success => unreachable!()
            }
        }

        // No compression needed.
        if dead_nodes == 0 {
            node_rewrites.truncate(0);
            self.scratch = Some(node_rewrites);
            return if do_completed == DoCompleted::Yes { Some(vec![]) } else { None };
        }

        // Pop off all the nodes we killed and extract the success
        // stories.
        let successful = if do_completed == DoCompleted::Yes {
            Some((0..dead_nodes)
                .map(|_| self.nodes.pop().unwrap())
                .flat_map(|node| {
                    match node.state.get() {
                        NodeState::Error => None,
                        NodeState::Done => Some(node.obligation),
                        _ => unreachable!()
                    }
                })
                .collect())
        } else {
            self.nodes.truncate(self.nodes.len() - dead_nodes);
            None
        };
        self.apply_rewrites(&node_rewrites);

        node_rewrites.truncate(0);
        self.scratch = Some(node_rewrites);

        successful
    }

    fn apply_rewrites(&mut self, node_rewrites: &[usize]) {
        let nodes_len = node_rewrites.len();

        for node in &mut self.nodes {
            if let Some(index) = node.parent {
                let new_index = node_rewrites[index.get()];
                if new_index >= nodes_len {
                    // parent dead due to error
                    node.parent = None;
                } else {
                    node.parent = Some(NodeIndex::new(new_index));
                }
            }

            let mut i = 0;
            while i < node.dependents.len() {
                let new_index = node_rewrites[node.dependents[i].get()];
                if new_index >= nodes_len {
                    node.dependents.swap_remove(i);
                } else {
                    node.dependents[i] = NodeIndex::new(new_index);
                    i += 1;
                }
            }
        }

        let mut kill_list = vec![];
        for (predicate, index) in &mut self.waiting_cache {
            let new_index = node_rewrites[index.get()];
            if new_index >= nodes_len {
                kill_list.push(predicate.clone());
            } else {
                *index = NodeIndex::new(new_index);
            }
        }

        for predicate in kill_list { self.waiting_cache.remove(&predicate); }
    }
}

impl<O> Node<O> {
    fn new(
        parent: Option<NodeIndex>,
        obligation: O,
        obligation_tree_id: ObligationTreeId
    ) -> Node<O> {
        Node {
            obligation,
            state: Cell::new(NodeState::Pending),
            parent,
            dependents: vec![],
            obligation_tree_id,
        }
    }
}

// I need a Clone closure
#[derive(Clone)]
struct GetObligation<'a, O: 'a>(&'a [Node<O>]);

impl<'a, 'b, O> FnOnce<(&'b usize,)> for GetObligation<'a, O> {
    type Output = &'a O;
    extern "rust-call" fn call_once(self, args: (&'b usize,)) -> &'a O {
        &self.0[*args.0].obligation
    }
}

impl<'a, 'b, O> FnMut<(&'b usize,)> for GetObligation<'a, O> {
    extern "rust-call" fn call_mut(&mut self, args: (&'b usize,)) -> &'a O {
        &self.0[*args.0].obligation
    }
}
