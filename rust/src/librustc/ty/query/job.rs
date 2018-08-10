// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(warnings)]

use std::mem;
use rustc_data_structures::sync::{Lock, LockGuard, Lrc, Weak};
use rustc_data_structures::OnDrop;
use syntax_pos::Span;
use ty::tls;
use ty::query::Query;
use ty::query::plumbing::CycleError;
use ty::context::TyCtxt;
use errors::Diagnostic;
use std::process;
use std::fmt;
use std::collections::HashSet;
#[cfg(parallel_queries)]
use {
    rayon_core,
    parking_lot::{Mutex, Condvar},
    std::sync::atomic::Ordering,
    std::thread,
    std::iter,
    std::iter::FromIterator,
    syntax_pos::DUMMY_SP,
    rustc_data_structures::stable_hasher::{StableHasherResult, StableHasher, HashStable},
};

/// Indicates the state of a query for a given key in a query map
pub(super) enum QueryResult<'tcx> {
    /// An already executing query. The query job can be used to await for its completion
    Started(Lrc<QueryJob<'tcx>>),

    /// The query panicked. Queries trying to wait on this will raise a fatal error / silently panic
    Poisoned,
}

/// A span and a query key
#[derive(Clone, Debug)]
pub struct QueryInfo<'tcx> {
    /// The span for a reason this query was required
    pub span: Span,
    pub query: Query<'tcx>,
}

/// A object representing an active query job.
pub struct QueryJob<'tcx> {
    pub info: QueryInfo<'tcx>,

    /// The parent query job which created this job and is implicitly waiting on it.
    pub parent: Option<Lrc<QueryJob<'tcx>>>,

    /// Diagnostic messages which are emitted while the query executes
    pub diagnostics: Lock<Vec<Diagnostic>>,

    /// The latch which is used to wait on this job
    #[cfg(parallel_queries)]
    latch: QueryLatch<'tcx>,
}

impl<'tcx> QueryJob<'tcx> {
    /// Creates a new query job
    pub fn new(info: QueryInfo<'tcx>, parent: Option<Lrc<QueryJob<'tcx>>>) -> Self {
        QueryJob {
            diagnostics: Lock::new(Vec::new()),
            info,
            parent,
            #[cfg(parallel_queries)]
            latch: QueryLatch::new(),
        }
    }

    /// Awaits for the query job to complete.
    ///
    /// For single threaded rustc there's no concurrent jobs running, so if we are waiting for any
    /// query that means that there is a query cycle, thus this always running a cycle error.
    pub(super) fn await<'lcx>(
        &self,
        tcx: TyCtxt<'_, 'tcx, 'lcx>,
        span: Span,
    ) -> Result<(), CycleError<'tcx>> {
        #[cfg(not(parallel_queries))]
        {
            self.find_cycle_in_stack(tcx, span)
        }

        #[cfg(parallel_queries)]
        {
            tls::with_related_context(tcx, move |icx| {
                let mut waiter = Lrc::new(QueryWaiter {
                    query: icx.query.clone(),
                    span,
                    cycle: Lock::new(None),
                    condvar: Condvar::new(),
                });
                self.latch.await(&waiter);

                match Lrc::get_mut(&mut waiter).unwrap().cycle.get_mut().take() {
                    None => Ok(()),
                    Some(cycle) => Err(cycle)
                }
            })
        }
    }

    #[cfg(not(parallel_queries))]
    fn find_cycle_in_stack<'lcx>(
        &self,
        tcx: TyCtxt<'_, 'tcx, 'lcx>,
        span: Span,
    ) -> Result<(), CycleError<'tcx>> {
        // Get the current executing query (waiter) and find the waitee amongst its parents
        let mut current_job = tls::with_related_context(tcx, |icx| icx.query.clone());
        let mut cycle = Vec::new();

        while let Some(job) = current_job {
            cycle.insert(0, job.info.clone());

            if &*job as *const _ == self as *const _ {
                // This is the end of the cycle
                // The span entry we included was for the usage
                // of the cycle itself, and not part of the cycle
                // Replace it with the span which caused the cycle to form
                cycle[0].span = span;
                // Find out why the cycle itself was used
                let usage = job.parent.as_ref().map(|parent| {
                    (job.info.span, parent.info.query.clone())
                });
                return Err(CycleError { usage, cycle });
            }

            current_job = job.parent.clone();
        }

        panic!("did not find a cycle")
    }

    /// Signals to waiters that the query is complete.
    ///
    /// This does nothing for single threaded rustc,
    /// as there are no concurrent jobs which could be waiting on us
    pub fn signal_complete(&self) {
        #[cfg(parallel_queries)]
        self.latch.set();
    }

    fn as_ptr(&self) -> *const QueryJob<'tcx> {
        self as *const _
    }
}

#[cfg(parallel_queries)]
struct QueryWaiter<'tcx> {
    query: Option<Lrc<QueryJob<'tcx>>>,
    condvar: Condvar,
    span: Span,
    cycle: Lock<Option<CycleError<'tcx>>>,
}

#[cfg(parallel_queries)]
impl<'tcx> QueryWaiter<'tcx> {
    fn notify(&self, registry: &rayon_core::Registry) {
        rayon_core::mark_unblocked(registry);
        self.condvar.notify_one();
    }
}

#[cfg(parallel_queries)]
struct QueryLatchInfo<'tcx> {
    complete: bool,
    waiters: Vec<Lrc<QueryWaiter<'tcx>>>,
}

#[cfg(parallel_queries)]
struct QueryLatch<'tcx> {
    info: Mutex<QueryLatchInfo<'tcx>>,
}

#[cfg(parallel_queries)]
impl<'tcx> QueryLatch<'tcx> {
    fn new() -> Self {
        QueryLatch {
            info: Mutex::new(QueryLatchInfo {
                complete: false,
                waiters: Vec::new(),
            }),
        }
    }

    /// Awaits the caller on this latch by blocking the current thread.
    fn await(&self, waiter: &Lrc<QueryWaiter<'tcx>>) {
        let mut info = self.info.lock();
        if !info.complete {
            // We push the waiter on to the `waiters` list. It can be accessed inside
            // the `wait` call below, by 1) the `set` method or 2) by deadlock detection.
            // Both of these will remove it from the `waiters` list before resuming
            // this thread.
            info.waiters.push(waiter.clone());

            // If this detects a deadlock and the deadlock handler wants to resume this thread
            // we have to be in the `wait` call. This is ensured by the deadlock handler
            // getting the self.info lock.
            rayon_core::mark_blocked();
            waiter.condvar.wait(&mut info);
        }
    }

    /// Sets the latch and resumes all waiters on it
    fn set(&self) {
        let mut info = self.info.lock();
        debug_assert!(!info.complete);
        info.complete = true;
        let registry = rayon_core::Registry::current();
        for waiter in info.waiters.drain(..) {
            waiter.notify(&registry);
        }
    }

    /// Remove a single waiter from the list of waiters.
    /// This is used to break query cycles.
    fn extract_waiter(
        &self,
        waiter: usize,
    ) -> Lrc<QueryWaiter<'tcx>> {
        let mut info = self.info.lock();
        debug_assert!(!info.complete);
        // Remove the waiter from the list of waiters
        info.waiters.remove(waiter)
    }
}

/// A resumable waiter of a query. The usize is the index into waiters in the query's latch
#[cfg(parallel_queries)]
type Waiter<'tcx> = (Lrc<QueryJob<'tcx>>, usize);

/// Visits all the non-resumable and resumable waiters of a query.
/// Only waiters in a query are visited.
/// `visit` is called for every waiter and is passed a query waiting on `query_ref`
/// and a span indicating the reason the query waited on `query_ref`.
/// If `visit` returns Some, this function returns.
/// For visits of non-resumable waiters it returns the return value of `visit`.
/// For visits of resumable waiters it returns Some(Some(Waiter)) which has the
/// required information to resume the waiter.
/// If all `visit` calls returns None, this function also returns None.
#[cfg(parallel_queries)]
fn visit_waiters<'tcx, F>(query: Lrc<QueryJob<'tcx>>, mut visit: F) -> Option<Option<Waiter<'tcx>>>
where
    F: FnMut(Span, Lrc<QueryJob<'tcx>>) -> Option<Option<Waiter<'tcx>>>
{
    // Visit the parent query which is a non-resumable waiter since it's on the same stack
    if let Some(ref parent) = query.parent {
        if let Some(cycle) = visit(query.info.span, parent.clone()) {
            return Some(cycle);
        }
    }

    // Visit the explict waiters which use condvars and are resumable
    for (i, waiter) in query.latch.info.lock().waiters.iter().enumerate() {
        if let Some(ref waiter_query) = waiter.query {
            if visit(waiter.span, waiter_query.clone()).is_some() {
                // Return a value which indicates that this waiter can be resumed
                return Some(Some((query.clone(), i)));
            }
        }
    }
    None
}

/// Look for query cycles by doing a depth first search starting at `query`.
/// `span` is the reason for the `query` to execute. This is initially DUMMY_SP.
/// If a cycle is detected, this initial value is replaced with the span causing
/// the cycle.
#[cfg(parallel_queries)]
fn cycle_check<'tcx>(query: Lrc<QueryJob<'tcx>>,
                     span: Span,
                     stack: &mut Vec<(Span, Lrc<QueryJob<'tcx>>)>,
                     visited: &mut HashSet<*const QueryJob<'tcx>>
) -> Option<Option<Waiter<'tcx>>> {
    if visited.contains(&query.as_ptr()) {
        return if let Some(p) = stack.iter().position(|q| q.1.as_ptr() == query.as_ptr()) {
            // We detected a query cycle, fix up the initial span and return Some

            // Remove previous stack entries
            stack.splice(0..p, iter::empty());
            // Replace the span for the first query with the cycle cause
            stack[0].0 = span;
            Some(None)
        } else {
            None
        }
    }

    // Mark this query is visited and add it to the stack
    visited.insert(query.as_ptr());
    stack.push((span, query.clone()));

    // Visit all the waiters
    let r = visit_waiters(query, |span, successor| {
        cycle_check(successor, span, stack, visited)
    });

    // Remove the entry in our stack if we didn't find a cycle
    if r.is_none() {
        stack.pop();
    }

    r
}

/// Finds out if there's a path to the compiler root (aka. code which isn't in a query)
/// from `query` without going through any of the queries in `visited`.
/// This is achieved with a depth first search.
#[cfg(parallel_queries)]
fn connected_to_root<'tcx>(
    query: Lrc<QueryJob<'tcx>>,
    visited: &mut HashSet<*const QueryJob<'tcx>>
) -> bool {
    // We already visited this or we're deliberately ignoring it
    if visited.contains(&query.as_ptr()) {
        return false;
    }

    // This query is connected to the root (it has no query parent), return true
    if query.parent.is_none() {
        return true;
    }

    visited.insert(query.as_ptr());

    let mut connected = false;

    visit_waiters(query, |_, successor| {
        if connected_to_root(successor, visited) {
            Some(None)
        } else {
            None
        }
    }).is_some()
}

/// Looks for query cycles starting from the last query in `jobs`.
/// If a cycle is found, all queries in the cycle is removed from `jobs` and
/// the function return true.
/// If a cycle was not found, the starting query is removed from `jobs` and
/// the function returns false.
#[cfg(parallel_queries)]
fn remove_cycle<'tcx>(
    jobs: &mut Vec<Lrc<QueryJob<'tcx>>>,
    wakelist: &mut Vec<Lrc<QueryWaiter<'tcx>>>,
    tcx: TyCtxt<'_, 'tcx, '_>
) -> bool {
    let mut visited = HashSet::new();
    let mut stack = Vec::new();
    // Look for a cycle starting with the last query in `jobs`
    if let Some(waiter) = cycle_check(jobs.pop().unwrap(),
                                      DUMMY_SP,
                                      &mut stack,
                                      &mut visited) {
        // Reverse the stack so earlier entries require later entries
        stack.reverse();

        // Extract the spans and queries into separate arrays
        let mut spans: Vec<_> = stack.iter().map(|e| e.0).collect();
        let queries = stack.into_iter().map(|e| e.1);

        // Shift the spans so that queries are matched with the span for their waitee
        let last = spans.pop().unwrap();
        spans.insert(0, last);

        // Zip them back together
        let mut stack: Vec<_> = spans.into_iter().zip(queries).collect();

        // Remove the queries in our cycle from the list of jobs to look at
        for r in &stack {
            if let Some(pos) = jobs.iter().position(|j| j.as_ptr() == r.1.as_ptr()) {
                jobs.remove(pos);
            }
        }

        // Find the queries in the cycle which are
        // connected to queries outside the cycle
        let entry_points: Vec<Lrc<QueryJob<'tcx>>> = stack.iter().filter_map(|query| {
            // Mark all the other queries in the cycle as already visited
            let mut visited = HashSet::from_iter(stack.iter().filter_map(|q| {
                if q.1.as_ptr() != query.1.as_ptr() {
                    Some(q.1.as_ptr())
                } else {
                    None
                }
            }));

            if connected_to_root(query.1.clone(), &mut visited) {
                Some(query.1.clone())
            } else {
                None
            }
        }).collect();

        // Deterministically pick an entry point
        // FIXME: Sort this instead
        let mut hcx = tcx.create_stable_hashing_context();
        let entry_point = entry_points.iter().min_by_key(|q| {
            let mut stable_hasher = StableHasher::<u64>::new();
            q.info.query.hash_stable(&mut hcx, &mut stable_hasher);
            stable_hasher.finish()
        }).unwrap().as_ptr();

        // Shift the stack until our entry point is first
        while stack[0].1.as_ptr() != entry_point {
            let last = stack.pop().unwrap();
            stack.insert(0, last);
        }

        // Create the cycle error
        let mut error = CycleError {
            usage: None,
            cycle: stack.iter().map(|&(s, ref q)| QueryInfo {
                span: s,
                query: q.info.query.clone(),
            } ).collect(),
        };

        // We unwrap `waiter` here since there must always be one
        // edge which is resumeable / waited using a query latch
        let (waitee_query, waiter_idx) = waiter.unwrap();

        // Extract the waiter we want to resume
        let waiter = waitee_query.latch.extract_waiter(waiter_idx);

        // Set the cycle error so it will be picked up when resumed
        *waiter.cycle.lock() = Some(error);

        // Put the waiter on the list of things to resume
        wakelist.push(waiter);

        true
    } else {
        false
    }
}

/// Creates a new thread and forwards information in thread locals to it.
/// The new thread runs the deadlock handler.
/// Must only be called when a deadlock is about to happen.
#[cfg(parallel_queries)]
pub unsafe fn handle_deadlock() {
    use syntax;
    use syntax_pos;

    let registry = rayon_core::Registry::current();

    let gcx_ptr = tls::GCX_PTR.with(|gcx_ptr| {
        gcx_ptr as *const _
    });
    let gcx_ptr = &*gcx_ptr;

    let syntax_globals = syntax::GLOBALS.with(|syntax_globals| {
        syntax_globals as *const _
    });
    let syntax_globals = &*syntax_globals;

    let syntax_pos_globals = syntax_pos::GLOBALS.with(|syntax_pos_globals| {
        syntax_pos_globals as *const _
    });
    let syntax_pos_globals = &*syntax_pos_globals;
    thread::spawn(move || {
        tls::GCX_PTR.set(gcx_ptr, || {
            syntax_pos::GLOBALS.set(syntax_pos_globals, || {
                syntax_pos::GLOBALS.set(syntax_pos_globals, || {
                    tls::with_thread_locals(|| {
                        tls::with_global(|tcx| deadlock(tcx, &registry))
                    })
                })
            })
        })
    });
}

/// Detects query cycles by using depth first search over all active query jobs.
/// If a query cycle is found it will break the cycle by finding an edge which
/// uses a query latch and then resuming that waiter.
/// There may be multiple cycles involved in a deadlock, so this searches
/// all active queries for cycles before finally resuming all the waiters at once.
#[cfg(parallel_queries)]
fn deadlock(tcx: TyCtxt<'_, '_, '_>, registry: &rayon_core::Registry) {
    let on_panic = OnDrop(|| {
        eprintln!("deadlock handler panicked, aborting process");
        process::abort();
    });

    let mut wakelist = Vec::new();
    let mut jobs: Vec<_> = tcx.queries.collect_active_jobs();

    let mut found_cycle = false;

    while jobs.len() > 0 {
        if remove_cycle(&mut jobs, &mut wakelist, tcx) {
            found_cycle = true;
        }
    }

    // Check that a cycle was found. It is possible for a deadlock to occur without
    // a query cycle if a query which can be waited on uses Rayon to do multithreading
    // internally. Such a query (X) may be executing on 2 threads (A and B) and A may
    // wait using Rayon on B. Rayon may then switch to executing another query (Y)
    // which in turn will wait on X causing a deadlock. We have a false dependency from
    // X to Y due to Rayon waiting and a true dependency from Y to X. The algorithm here
    // only considers the true dependency and won't detect a cycle.
    assert!(found_cycle);

    // FIXME: Ensure this won't cause a deadlock before we return
    for waiter in wakelist.into_iter() {
        waiter.notify(registry);
    }

    on_panic.disable();
}
