// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Code related to match expressions. These are sufficiently complex
//! to warrant their own module and submodules. :) This main module
//! includes the high-level algorithm, the submodules contain the
//! details.

use build::{BlockAnd, BlockAndExtension, Builder};
use build::{GuardFrame, GuardFrameLocal, LocalsForNode};
use build::ForGuard::{self, OutsideGuard, RefWithinGuard, ValWithinGuard};
use rustc_data_structures::fx::FxHashMap;
use rustc_data_structures::bitvec::BitVector;
use rustc::ty::{self, Ty};
use rustc::mir::*;
use rustc::hir;
use hair::*;
use syntax::ast::{Name, NodeId};
use syntax_pos::Span;

// helper functions, broken out by category:
mod simplify;
mod test;
mod util;

/// ArmHasGuard is isomorphic to a boolean flag. It indicates whether
/// a match arm has a guard expression attached to it.
#[derive(Copy, Clone, Debug)]
pub(crate) struct ArmHasGuard(pub bool);

impl<'a, 'gcx, 'tcx> Builder<'a, 'gcx, 'tcx> {
    pub fn match_expr(&mut self,
                      destination: &Place<'tcx>,
                      span: Span,
                      mut block: BasicBlock,
                      discriminant: ExprRef<'tcx>,
                      arms: Vec<Arm<'tcx>>)
                      -> BlockAnd<()> {
        let tcx = self.hir.tcx();
        let discriminant_place = unpack!(block = self.as_place(block, discriminant));

        // Matching on a `discriminant_place` with an uninhabited type doesn't
        // generate any memory reads by itself, and so if the place "expression"
        // contains unsafe operations like raw pointer dereferences or union
        // field projections, we wouldn't know to require an `unsafe` block
        // around a `match` equivalent to `std::intrinsics::unreachable()`.
        // See issue #47412 for this hole being discovered in the wild.
        //
        // HACK(eddyb) Work around the above issue by adding a dummy inspection
        // of `discriminant_place`, specifically by applying `Rvalue::Discriminant`
        // (which will work regardless of type) and storing the result in a temp.
        //
        // NOTE: Under NLL, the above issue should no longer occur because it
        // injects a borrow of the matched input, which should have the same effect
        // as eddyb's hack. Once NLL is the default, we can remove the hack.

        let dummy_source_info = self.source_info(span);
        let dummy_access = Rvalue::Discriminant(discriminant_place.clone());
        let dummy_ty = dummy_access.ty(&self.local_decls, tcx);
        let dummy_temp = self.temp(dummy_ty, dummy_source_info.span);
        self.cfg.push_assign(block, dummy_source_info, &dummy_temp, dummy_access);

        let source_info = self.source_info(span);
        let borrowed_input_temp = if tcx.generate_borrow_of_any_match_input() {
            // The region is unknown at this point; we rely on NLL
            // inference to find an appropriate one. Therefore you can
            // only use this when NLL is turned on.
            assert!(tcx.use_mir_borrowck());
            let borrowed_input =
                Rvalue::Ref(tcx.types.re_empty, BorrowKind::Shared, discriminant_place.clone());
            let borrowed_input_ty = borrowed_input.ty(&self.local_decls, tcx);
            let borrowed_input_temp = self.temp(borrowed_input_ty, span);
            self.cfg.push_assign(block, source_info, &borrowed_input_temp, borrowed_input);
            Some(borrowed_input_temp)
        } else {
            None
        };

        let mut arm_blocks = ArmBlocks {
            blocks: arms.iter()
                        .map(|_| self.cfg.start_new_block())
                        .collect(),
        };

        // Get the arm bodies and their scopes, while declaring bindings.
        let arm_bodies: Vec<_> = arms.iter().map(|arm| {
            // BUG: use arm lint level
            let body = self.hir.mirror(arm.body.clone());
            let scope = self.declare_bindings(None, body.span,
                                              LintLevel::Inherited,
                                              &arm.patterns[0],
                                              ArmHasGuard(arm.guard.is_some()));
            (body, scope.unwrap_or(self.source_scope))
        }).collect();

        // create binding start block for link them by false edges
        let candidate_count = arms.iter().fold(0, |ac, c| ac + c.patterns.len());
        let pre_binding_blocks: Vec<_> = (0..candidate_count + 1)
            .map(|_| self.cfg.start_new_block()).collect();

        // assemble a list of candidates: there is one candidate per
        // pattern, which means there may be more than one candidate
        // *per arm*. These candidates are kept sorted such that the
        // highest priority candidate comes first in the list.
        // (i.e. same order as in source)

        let candidates: Vec<_> =
            arms.iter()
                .enumerate()
                .flat_map(|(arm_index, arm)| {
                    arm.patterns.iter()
                                .map(move |pat| (arm_index, pat, arm.guard.clone()))
                })
                .zip(pre_binding_blocks.iter().zip(pre_binding_blocks.iter().skip(1)))
                .map(|((arm_index, pattern, guard),
                       (pre_binding_block, next_candidate_pre_binding_block))| {

                    if let (true, Some(borrow_temp)) = (tcx.emit_read_for_match(),
                                                        borrowed_input_temp.clone()) {
                        // inject a fake read of the borrowed input at
                        // the start of each arm's pattern testing
                        // code.
                        //
                        // This should ensure that you cannot change
                        // the variant for an enum while you are in
                        // the midst of matching on it.

                        self.cfg.push(*pre_binding_block, Statement {
                            source_info,
                            kind: StatementKind::ReadForMatch(borrow_temp.clone()),
                        });
                    }

                    // One might ask: why not build up the match pair such that it
                    // matches via `borrowed_input_temp.deref()` instead of
                    // using the `discriminant_place` directly, as it is doing here?
                    //
                    // The basic answer is that if you do that, then you end up with
                    // accceses to a shared borrow of the input and that conflicts with
                    // any arms that look like e.g.
                    //
                    // match Some(&4) {
                    //     ref mut foo => {
                    //         ... /* mutate `foo` in arm body */ ...
                    //     }
                    // }
                    //
                    // (Perhaps we could further revise the MIR
                    //  construction here so that it only does a
                    //  shared borrow at the outset and delays doing
                    //  the mutable borrow until after the pattern is
                    //  matched *and* the guard (if any) for the arm
                    //  has been run.)

                    Candidate {
                        span: pattern.span,
                        match_pairs: vec![MatchPair::new(discriminant_place.clone(), pattern)],
                        bindings: vec![],
                        guard,
                        arm_index,
                        pre_binding_block: *pre_binding_block,
                        next_candidate_pre_binding_block: *next_candidate_pre_binding_block,
                    }
                })
                .collect();

        let outer_source_info = self.source_info(span);
        self.cfg.terminate(*pre_binding_blocks.last().unwrap(),
                           outer_source_info, TerminatorKind::Unreachable);

        // this will generate code to test discriminant_place and
        // branch to the appropriate arm block
        let otherwise = self.match_candidates(span, &mut arm_blocks, candidates, block);

        if !otherwise.is_empty() {
            // All matches are exhaustive. However, because some matches
            // only have exponentially-large exhaustive decision trees, we
            // sometimes generate an inexhaustive decision tree.
            //
            // In that case, the inexhaustive tips of the decision tree
            // can't be reached - terminate them with an `unreachable`.
            let source_info = self.source_info(span);

            let mut otherwise = otherwise;
            otherwise.sort();
            otherwise.dedup(); // variant switches can introduce duplicate target blocks
            for block in otherwise {
                self.cfg.terminate(block, source_info, TerminatorKind::Unreachable);
            }
        }

        // all the arm blocks will rejoin here
        let end_block = self.cfg.start_new_block();

        let outer_source_info = self.source_info(span);
        for (arm_index, (body, source_scope)) in arm_bodies.into_iter().enumerate() {
            let mut arm_block = arm_blocks.blocks[arm_index];
            // Re-enter the source scope we created the bindings in.
            self.source_scope = source_scope;
            unpack!(arm_block = self.into(destination, arm_block, body));
            self.cfg.terminate(arm_block, outer_source_info,
                               TerminatorKind::Goto { target: end_block });
        }
        self.source_scope = outer_source_info.scope;

        end_block.unit()
    }

    pub fn user_assert_ty(&mut self, block: BasicBlock, hir_id: hir::HirId,
                          var: NodeId, span: Span) {
        if self.hir.tcx().sess.opts.debugging_opts.disable_nll_user_type_assert { return; }

        let local_id = self.var_local_id(var, OutsideGuard);
        let source_info = self.source_info(span);

        debug!("user_assert_ty: local_id={:?}", hir_id.local_id);
        if let Some(c_ty) = self.hir.tables.user_provided_tys().get(hir_id) {
            debug!("user_assert_ty: c_ty={:?}", c_ty);
            self.cfg.push(block, Statement {
                source_info,
                kind: StatementKind::UserAssertTy(*c_ty, local_id),
            });
        }
    }

    pub fn expr_into_pattern(&mut self,
                             mut block: BasicBlock,
                             ty: Option<hir::HirId>,
                             irrefutable_pat: Pattern<'tcx>,
                             initializer: ExprRef<'tcx>)
                             -> BlockAnd<()> {
        // optimize the case of `let x = ...`
        match *irrefutable_pat.kind {
            PatternKind::Binding { mode: BindingMode::ByValue,
                                   var,
                                   subpattern: None, .. } => {
                let place = self.storage_live_binding(block, var, irrefutable_pat.span,
                                                      OutsideGuard);

                if let Some(ty) = ty {
                    self.user_assert_ty(block, ty, var, irrefutable_pat.span);
                }

                unpack!(block = self.into(&place, block, initializer));
                self.schedule_drop_for_binding(var, irrefutable_pat.span, OutsideGuard);
                block.unit()
            }
            _ => {
                let place = unpack!(block = self.as_place(block, initializer));
                self.place_into_pattern(block, irrefutable_pat, &place)
            }
        }
    }

    pub fn place_into_pattern(&mut self,
                               mut block: BasicBlock,
                               irrefutable_pat: Pattern<'tcx>,
                               initializer: &Place<'tcx>)
                               -> BlockAnd<()> {
        // create a dummy candidate
        let mut candidate = Candidate {
            span: irrefutable_pat.span,
            match_pairs: vec![MatchPair::new(initializer.clone(), &irrefutable_pat)],
            bindings: vec![],
            guard: None,

            // since we don't call `match_candidates`, next fields is unused
            arm_index: 0,
            pre_binding_block: block,
            next_candidate_pre_binding_block: block
        };

        // Simplify the candidate. Since the pattern is irrefutable, this should
        // always convert all match-pairs into bindings.
        unpack!(block = self.simplify_candidate(block, &mut candidate));

        if !candidate.match_pairs.is_empty() {
            span_bug!(candidate.match_pairs[0].pattern.span,
                      "match pairs {:?} remaining after simplifying \
                       irrefutable pattern",
                      candidate.match_pairs);
        }

        // now apply the bindings, which will also declare the variables
        self.bind_matched_candidate_for_arm_body(block, &candidate.bindings);

        block.unit()
    }

    /// Declares the bindings of the given pattern and returns the visibility scope
    /// for the bindings in this patterns, if such a scope had to be created.
    /// NOTE: Declaring the bindings should always be done in their drop scope.
    pub fn declare_bindings(&mut self,
                            mut visibility_scope: Option<SourceScope>,
                            scope_span: Span,
                            lint_level: LintLevel,
                            pattern: &Pattern<'tcx>,
                            has_guard: ArmHasGuard)
                            -> Option<SourceScope> {
        assert!(!(visibility_scope.is_some() && lint_level.is_explicit()),
                "can't have both a visibility and a lint scope at the same time");
        let mut scope = self.source_scope;
        self.visit_bindings(pattern, &mut |this, mutability, name, mode, var, span, ty| {
            if visibility_scope.is_none() {
                visibility_scope = Some(this.new_source_scope(scope_span,
                                                           LintLevel::Inherited,
                                                           None));
                // If we have lints, create a new source scope
                // that marks the lints for the locals. See the comment
                // on the `source_info` field for why this is needed.
                if lint_level.is_explicit() {
                    scope =
                        this.new_source_scope(scope_span, lint_level, None);
                }
            }
            let source_info = SourceInfo {
                span,
                scope,
            };
            let visibility_scope = visibility_scope.unwrap();
            this.declare_binding(source_info, visibility_scope, mutability, name, mode, var,
                                 ty, has_guard);
        });
        visibility_scope
    }

    pub fn storage_live_binding(&mut self,
                                block: BasicBlock,
                                var: NodeId,
                                span: Span,
                                for_guard: ForGuard)
                            -> Place<'tcx>
    {
        let local_id = self.var_local_id(var, for_guard);
        let source_info = self.source_info(span);
        self.cfg.push(block, Statement {
            source_info,
            kind: StatementKind::StorageLive(local_id)
        });
        Place::Local(local_id)
    }

    pub fn schedule_drop_for_binding(&mut self,
                                     var: NodeId,
                                     span: Span,
                                     for_guard: ForGuard) {
        let local_id = self.var_local_id(var, for_guard);
        let var_ty = self.local_decls[local_id].ty;
        let hir_id = self.hir.tcx().hir.node_to_hir_id(var);
        let region_scope = self.hir.region_scope_tree.var_scope(hir_id.local_id);
        self.schedule_drop(span, region_scope, &Place::Local(local_id), var_ty);
    }

    pub fn visit_bindings<F>(&mut self, pattern: &Pattern<'tcx>, f: &mut F)
        where F: FnMut(&mut Self, Mutability, Name, BindingMode, NodeId, Span, Ty<'tcx>)
    {
        match *pattern.kind {
            PatternKind::Binding { mutability, name, mode, var, ty, ref subpattern, .. } => {
                f(self, mutability, name, mode, var, pattern.span, ty);
                if let Some(subpattern) = subpattern.as_ref() {
                    self.visit_bindings(subpattern, f);
                }
            }
            PatternKind::Array { ref prefix, ref slice, ref suffix } |
            PatternKind::Slice { ref prefix, ref slice, ref suffix } => {
                for subpattern in prefix.iter().chain(slice).chain(suffix) {
                    self.visit_bindings(subpattern, f);
                }
            }
            PatternKind::Constant { .. } | PatternKind::Range { .. } | PatternKind::Wild => {
            }
            PatternKind::Deref { ref subpattern } => {
                self.visit_bindings(subpattern, f);
            }
            PatternKind::Leaf { ref subpatterns } |
            PatternKind::Variant { ref subpatterns, .. } => {
                for subpattern in subpatterns {
                    self.visit_bindings(&subpattern.pattern, f);
                }
            }
        }
    }
}


/// List of blocks for each arm (and potentially other metadata in the
/// future).
struct ArmBlocks {
    blocks: Vec<BasicBlock>,
}

#[derive(Clone, Debug)]
pub struct Candidate<'pat, 'tcx:'pat> {
    // span of the original pattern that gave rise to this candidate
    span: Span,

    // all of these must be satisfied...
    match_pairs: Vec<MatchPair<'pat, 'tcx>>,

    // ...these bindings established...
    bindings: Vec<Binding<'tcx>>,

    // ...and the guard must be evaluated...
    guard: Option<ExprRef<'tcx>>,

    // ...and then we branch to arm with this index.
    arm_index: usize,

    // ...and the blocks for add false edges between candidates
    pre_binding_block: BasicBlock,
    next_candidate_pre_binding_block: BasicBlock,
}

#[derive(Clone, Debug)]
struct Binding<'tcx> {
    span: Span,
    source: Place<'tcx>,
    name: Name,
    var_id: NodeId,
    var_ty: Ty<'tcx>,
    mutability: Mutability,
    binding_mode: BindingMode<'tcx>,
}

#[derive(Clone, Debug)]
pub struct MatchPair<'pat, 'tcx:'pat> {
    // this place...
    place: Place<'tcx>,

    // ... must match this pattern.
    pattern: &'pat Pattern<'tcx>,

    // HACK(eddyb) This is used to toggle whether a Slice pattern
    // has had its length checked. This is only necessary because
    // the "rest" part of the pattern right now has type &[T] and
    // as such, it requires an Rvalue::Slice to be generated.
    // See RFC 495 / issue #23121 for the eventual (proper) solution.
    slice_len_checked: bool
}

#[derive(Clone, Debug, PartialEq)]
enum TestKind<'tcx> {
    // test the branches of enum
    Switch {
        adt_def: &'tcx ty::AdtDef,
        variants: BitVector,
    },

    // test the branches of enum
    SwitchInt {
        switch_ty: Ty<'tcx>,
        options: Vec<u128>,
        indices: FxHashMap<&'tcx ty::Const<'tcx>, usize>,
    },

    // test for equality
    Eq {
        value: &'tcx ty::Const<'tcx>,
        ty: Ty<'tcx>,
    },

    // test whether the value falls within an inclusive or exclusive range
    Range {
        lo: Literal<'tcx>,
        hi: Literal<'tcx>,
        ty: Ty<'tcx>,
        end: hir::RangeEnd,
    },

    // test length of the slice is equal to len
    Len {
        len: u64,
        op: BinOp,
    },
}

#[derive(Debug)]
pub struct Test<'tcx> {
    span: Span,
    kind: TestKind<'tcx>,
}

///////////////////////////////////////////////////////////////////////////
// Main matching algorithm

impl<'a, 'gcx, 'tcx> Builder<'a, 'gcx, 'tcx> {
    /// The main match algorithm. It begins with a set of candidates
    /// `candidates` and has the job of generating code to determine
    /// which of these candidates, if any, is the correct one. The
    /// candidates are sorted such that the first item in the list
    /// has the highest priority. When a candidate is found to match
    /// the value, we will generate a branch to the appropriate
    /// block found in `arm_blocks`.
    ///
    /// The return value is a list of "otherwise" blocks. These are
    /// points in execution where we found that *NONE* of the
    /// candidates apply.  In principle, this means that the input
    /// list was not exhaustive, though at present we sometimes are
    /// not smart enough to recognize all exhaustive inputs.
    ///
    /// It might be surprising that the input can be inexhaustive.
    /// Indeed, initially, it is not, because all matches are
    /// exhaustive in Rust. But during processing we sometimes divide
    /// up the list of candidates and recurse with a non-exhaustive
    /// list. This is important to keep the size of the generated code
    /// under control. See `test_candidates` for more details.
    fn match_candidates<'pat>(&mut self,
                              span: Span,
                              arm_blocks: &mut ArmBlocks,
                              mut candidates: Vec<Candidate<'pat, 'tcx>>,
                              mut block: BasicBlock)
                              -> Vec<BasicBlock>
    {
        debug!("matched_candidate(span={:?}, block={:?}, candidates={:?})",
               span, block, candidates);

        // Start by simplifying candidates. Once this process is
        // complete, all the match pairs which remain require some
        // form of test, whether it be a switch or pattern comparison.
        for candidate in &mut candidates {
            unpack!(block = self.simplify_candidate(block, candidate));
        }

        // The candidates are sorted by priority. Check to see
        // whether the higher priority candidates (and hence at
        // the front of the vec) have satisfied all their match
        // pairs.
        let fully_matched =
            candidates.iter().take_while(|c| c.match_pairs.is_empty()).count();
        debug!("match_candidates: {:?} candidates fully matched", fully_matched);
        let mut unmatched_candidates = candidates.split_off(fully_matched);

        let fully_matched_with_guard =
            candidates.iter().take_while(|c| c.guard.is_some()).count();

        let unreachable_candidates = if fully_matched_with_guard + 1 < candidates.len() {
            candidates.split_off(fully_matched_with_guard + 1)
        } else {
            vec![]
        };

        for candidate in candidates {
            // If so, apply any bindings, test the guard (if any), and
            // branch to the arm.
            if let Some(b) = self.bind_and_guard_matched_candidate(block, arm_blocks, candidate) {
                block = b;
            } else {
                // if None is returned, then any remaining candidates
                // are unreachable (at least not through this path).
                // Link them with false edges.
                debug!("match_candidates: add false edges for unreachable {:?} and unmatched {:?}",
                       unreachable_candidates, unmatched_candidates);
                for candidate in unreachable_candidates {
                    let source_info = self.source_info(candidate.span);
                    let target = self.cfg.start_new_block();
                    if let Some(otherwise) = self.bind_and_guard_matched_candidate(target,
                                                                                   arm_blocks,
                                                                                   candidate) {
                        self.cfg.terminate(otherwise, source_info, TerminatorKind::Unreachable);
                    }
                }

                if unmatched_candidates.is_empty() {
                    return vec![]
                } else {
                    let target = self.cfg.start_new_block();
                    return self.match_candidates(span, arm_blocks, unmatched_candidates, target);
                }
            }
        }

        // If there are no candidates that still need testing, we're done.
        // Since all matches are exhaustive, execution should never reach this point.
        if unmatched_candidates.is_empty() {
            return vec![block];
        }

        // Test candidates where possible.
        let (otherwise, tested_candidates) =
            self.test_candidates(span, arm_blocks, &unmatched_candidates, block);

        // If the target candidates were exhaustive, then we are done.
        // But for borrowck continue build decision tree.

        // If all candidates were sorted into `target_candidates` somewhere, then
        // the initial set was inexhaustive.
        let untested_candidates = unmatched_candidates.split_off(tested_candidates);
        if untested_candidates.len() == 0 {
            return otherwise;
        }

        // Otherwise, let's process those remaining candidates.
        let join_block = self.join_otherwise_blocks(span, otherwise);
        self.match_candidates(span, arm_blocks, untested_candidates, join_block)
    }

    fn join_otherwise_blocks(&mut self,
                             span: Span,
                             mut otherwise: Vec<BasicBlock>)
                             -> BasicBlock
    {
        let source_info = self.source_info(span);
        otherwise.sort();
        otherwise.dedup(); // variant switches can introduce duplicate target blocks
        if otherwise.len() == 1 {
            otherwise[0]
        } else {
            let join_block = self.cfg.start_new_block();
            for block in otherwise {
                self.cfg.terminate(block, source_info,
                                   TerminatorKind::Goto { target: join_block });
            }
            join_block
        }
    }

    /// This is the most subtle part of the matching algorithm.  At
    /// this point, the input candidates have been fully simplified,
    /// and so we know that all remaining match-pairs require some
    /// sort of test. To decide what test to do, we take the highest
    /// priority candidate (last one in the list) and extract the
    /// first match-pair from the list. From this we decide what kind
    /// of test is needed using `test`, defined in the `test` module.
    ///
    /// *Note:* taking the first match pair is somewhat arbitrary, and
    /// we might do better here by choosing more carefully what to
    /// test.
    ///
    /// For example, consider the following possible match-pairs:
    ///
    /// 1. `x @ Some(P)` -- we will do a `Switch` to decide what variant `x` has
    /// 2. `x @ 22` -- we will do a `SwitchInt`
    /// 3. `x @ 3..5` -- we will do a range test
    /// 4. etc.
    ///
    /// Once we know what sort of test we are going to perform, this
    /// test may also help us with other candidates. So we walk over
    /// the candidates (from high to low priority) and check. This
    /// gives us, for each outcome of the test, a transformed list of
    /// candidates.  For example, if we are testing the current
    /// variant of `x.0`, and we have a candidate `{x.0 @ Some(v), x.1
    /// @ 22}`, then we would have a resulting candidate of `{(x.0 as
    /// Some).0 @ v, x.1 @ 22}`. Note that the first match-pair is now
    /// simpler (and, in fact, irrefutable).
    ///
    /// But there may also be candidates that the test just doesn't
    /// apply to. The classical example involves wildcards:
    ///
    /// ```
    /// # let (x, y, z) = (true, true, true);
    /// match (x, y, z) {
    ///     (true, _, true) => true,    // (0)
    ///     (_, true, _) => true,       // (1)
    ///     (false, false, _) => false, // (2)
    ///     (true, _, false) => false,  // (3)
    /// }
    /// ```
    ///
    /// In that case, after we test on `x`, there are 2 overlapping candidate
    /// sets:
    ///
    /// - If the outcome is that `x` is true, candidates 0, 1, and 3
    /// - If the outcome is that `x` is false, candidates 1 and 2
    ///
    /// Here, the traditional "decision tree" method would generate 2
    /// separate code-paths for the 2 separate cases.
    ///
    /// In some cases, this duplication can create an exponential amount of
    /// code. This is most easily seen by noticing that this method terminates
    /// with precisely the reachable arms being reachable - but that problem
    /// is trivially NP-complete:
    ///
    /// ```rust
    ///     match (var0, var1, var2, var3, ..) {
    ///         (true, _, _, false, true, ...) => false,
    ///         (_, true, true, false, _, ...) => false,
    ///         (false, _, false, false, _, ...) => false,
    ///         ...
    ///         _ => true
    ///     }
    /// ```
    ///
    /// Here the last arm is reachable only if there is an assignment to
    /// the variables that does not match any of the literals. Therefore,
    /// compilation would take an exponential amount of time in some cases.
    ///
    /// That kind of exponential worst-case might not occur in practice, but
    /// our simplistic treatment of constants and guards would make it occur
    /// in very common situations - for example #29740:
    ///
    /// ```rust
    /// match x {
    ///     "foo" if foo_guard => ...,
    ///     "bar" if bar_guard => ...,
    ///     "baz" if baz_guard => ...,
    ///     ...
    /// }
    /// ```
    ///
    /// Here we first test the match-pair `x @ "foo"`, which is an `Eq` test.
    ///
    /// It might seem that we would end up with 2 disjoint candidate
    /// sets, consisting of the first candidate or the other 3, but our
    /// algorithm doesn't reason about "foo" being distinct from the other
    /// constants; it considers the latter arms to potentially match after
    /// both outcomes, which obviously leads to an exponential amount
    /// of tests.
    ///
    /// To avoid these kinds of problems, our algorithm tries to ensure
    /// the amount of generated tests is linear. When we do a k-way test,
    /// we return an additional "unmatched" set alongside the obvious `k`
    /// sets. When we encounter a candidate that would be present in more
    /// than one of the sets, we put it and all candidates below it into the
    /// "unmatched" set. This ensures these `k+1` sets are disjoint.
    ///
    /// After we perform our test, we branch into the appropriate candidate
    /// set and recurse with `match_candidates`. These sub-matches are
    /// obviously inexhaustive - as we discarded our otherwise set - so
    /// we set their continuation to do `match_candidates` on the
    /// "unmatched" set (which is again inexhaustive).
    ///
    /// If you apply this to the above test, you basically wind up
    /// with an if-else-if chain, testing each candidate in turn,
    /// which is precisely what we want.
    ///
    /// In addition to avoiding exponential-time blowups, this algorithm
    /// also has nice property that each guard and arm is only generated
    /// once.
    fn test_candidates<'pat>(&mut self,
                             span: Span,
                             arm_blocks: &mut ArmBlocks,
                             candidates: &[Candidate<'pat, 'tcx>],
                             block: BasicBlock)
                             -> (Vec<BasicBlock>, usize)
    {
        // extract the match-pair from the highest priority candidate
        let match_pair = &candidates.first().unwrap().match_pairs[0];
        let mut test = self.test(match_pair);

        // most of the time, the test to perform is simply a function
        // of the main candidate; but for a test like SwitchInt, we
        // may want to add cases based on the candidates that are
        // available
        match test.kind {
            TestKind::SwitchInt { switch_ty, ref mut options, ref mut indices } => {
                for candidate in candidates.iter() {
                    if !self.add_cases_to_switch(&match_pair.place,
                                                 candidate,
                                                 switch_ty,
                                                 options,
                                                 indices) {
                        break;
                    }
                }
            }
            TestKind::Switch { adt_def: _, ref mut variants} => {
                for candidate in candidates.iter() {
                    if !self.add_variants_to_switch(&match_pair.place,
                                                    candidate,
                                                    variants) {
                        break;
                    }
                }
            }
            _ => { }
        }

        // perform the test, branching to one of N blocks. For each of
        // those N possible outcomes, create a (initially empty)
        // vector of candidates. Those are the candidates that still
        // apply if the test has that particular outcome.
        debug!("match_candidates: test={:?} match_pair={:?}", test, match_pair);
        let target_blocks = self.perform_test(block, &match_pair.place, &test);
        let mut target_candidates: Vec<_> = (0..target_blocks.len()).map(|_| vec![]).collect();

        // Sort the candidates into the appropriate vector in
        // `target_candidates`. Note that at some point we may
        // encounter a candidate where the test is not relevant; at
        // that point, we stop sorting.
        let tested_candidates =
            candidates.iter()
                      .take_while(|c| self.sort_candidate(&match_pair.place,
                                                          &test,
                                                          c,
                                                          &mut target_candidates))
                      .count();
        assert!(tested_candidates > 0); // at least the last candidate ought to be tested
        debug!("tested_candidates: {}", tested_candidates);
        debug!("untested_candidates: {}", candidates.len() - tested_candidates);

        // For each outcome of test, process the candidates that still
        // apply. Collect a list of blocks where control flow will
        // branch if one of the `target_candidate` sets is not
        // exhaustive.
        let otherwise: Vec<_> =
            target_blocks.into_iter()
                         .zip(target_candidates)
                         .flat_map(|(target_block, target_candidates)| {
                             self.match_candidates(span,
                                                   arm_blocks,
                                                   target_candidates,
                                                   target_block)
                         })
                         .collect();

        (otherwise, tested_candidates)
    }

    /// Initializes each of the bindings from the candidate by
    /// moving/copying/ref'ing the source as appropriate. Tests the
    /// guard, if any, and then branches to the arm. Returns the block
    /// for the case where the guard fails.
    ///
    /// Note: we check earlier that if there is a guard, there cannot
    /// be move bindings.  This isn't really important for the
    /// self-consistency of this fn, but the reason for it should be
    /// clear: after we've done the assignments, if there were move
    /// bindings, further tests would be a use-after-move (which would
    /// in turn be detected by the borrowck code that runs on the
    /// MIR).
    fn bind_and_guard_matched_candidate<'pat>(&mut self,
                                              mut block: BasicBlock,
                                              arm_blocks: &mut ArmBlocks,
                                              candidate: Candidate<'pat, 'tcx>)
                                              -> Option<BasicBlock> {
        debug!("bind_and_guard_matched_candidate(block={:?}, candidate={:?})",
               block, candidate);

        debug_assert!(candidate.match_pairs.is_empty());

        let arm_block = arm_blocks.blocks[candidate.arm_index];
        let candidate_source_info = self.source_info(candidate.span);

        self.cfg.terminate(block, candidate_source_info,
                               TerminatorKind::Goto { target: candidate.pre_binding_block });

        block = self.cfg.start_new_block();
        self.cfg.terminate(candidate.pre_binding_block, candidate_source_info,
                               TerminatorKind::FalseEdges {
                                   real_target: block,
                                   imaginary_targets:
                                       vec![candidate.next_candidate_pre_binding_block],
                               });


        // rust-lang/rust#27282: The `autoref` business deserves some
        // explanation here.
        //
        // The intent of the `autoref` flag is that when it is true,
        // then any pattern bindings of type T will map to a `&T`
        // within the context of the guard expression, but will
        // continue to map to a `T` in the context of the arm body. To
        // avoid surfacing this distinction in the user source code
        // (which would be a severe change to the language and require
        // far more revision to the compiler), when `autoref` is true,
        // then any occurrence of the identifier in the guard
        // expression will automatically get a deref op applied to it.
        //
        // So an input like:
        //
        // ```
        // let place = Foo::new();
        // match place { foo if inspect(foo)
        //     => feed(foo), ...  }
        // ```
        //
        // will be treated as if it were really something like:
        //
        // ```
        // let place = Foo::new();
        // match place { Foo { .. } if { let tmp1 = &place; inspect(*tmp1) }
        //     => { let tmp2 = place; feed(tmp2) }, ... }
        //
        // And an input like:
        //
        // ```
        // let place = Foo::new();
        // match place { ref mut foo if inspect(foo)
        //     => feed(foo), ...  }
        // ```
        //
        // will be treated as if it were really something like:
        //
        // ```
        // let place = Foo::new();
        // match place { Foo { .. } if { let tmp1 = & &mut place; inspect(*tmp1) }
        //     => { let tmp2 = &mut place; feed(tmp2) }, ... }
        // ```
        //
        // In short, any pattern binding will always look like *some*
        // kind of `&T` within the guard at least in terms of how the
        // MIR-borrowck views it, and this will ensure that guard
        // expressions cannot mutate their the match inputs via such
        // bindings. (It also ensures that guard expressions can at
        // most *copy* values from such bindings; non-Copy things
        // cannot be moved via pattern bindings in guard expressions.)
        //
        // ----
        //
        // Implementation notes (under assumption `autoref` is true).
        //
        // To encode the distinction above, we must inject the
        // temporaries `tmp1` and `tmp2`.
        //
        // There are two cases of interest: binding by-value, and binding by-ref.
        //
        // 1. Binding by-value: Things are simple.
        //
        //    * Establishing `tmp1` creates a reference into the
        //      matched place. This code is emitted by
        //      bind_matched_candidate_for_guard.
        //
        //    * `tmp2` is only initialized "lazily", after we have
        //      checked the guard. Thus, the code that can trigger
        //      moves out of the candidate can only fire after the
        //      guard evaluated to true. This initialization code is
        //      emitted by bind_matched_candidate_for_arm.
        //
        // 2. Binding by-reference: Things are tricky.
        //
        //    * Here, the guard expression wants a `&&` or `&&mut`
        //      into the original input. This means we need to borrow
        //      a reference that we do not immediately have at hand
        //      (because all we have is the places associated with the
        //      match input itself; it is up to us to create a place
        //      holding a `&` or `&mut` that we can then borrow).

        let autoref = self.hir.tcx().all_pat_vars_are_implicit_refs_within_guards();
        if let Some(guard) = candidate.guard {
            if autoref {
                self.bind_matched_candidate_for_guard(block, &candidate.bindings);
                let guard_frame = GuardFrame {
                    locals: candidate.bindings.iter()
                        .map(|b| GuardFrameLocal::new(b.var_id, b.binding_mode))
                        .collect(),
                };
                debug!("Entering guard building context: {:?}", guard_frame);
                self.guard_context.push(guard_frame);
            } else {
                self.bind_matched_candidate_for_arm_body(block, &candidate.bindings);
            }

            // the block to branch to if the guard fails; if there is no
            // guard, this block is simply unreachable
            let guard = self.hir.mirror(guard);
            let source_info = self.source_info(guard.span);
            let cond = unpack!(block = self.as_local_operand(block, guard));
            if autoref {
                let guard_frame = self.guard_context.pop().unwrap();
                debug!("Exiting guard building context with locals: {:?}", guard_frame);
            }

            let false_edge_block = self.cfg.start_new_block();

            // We want to ensure that the matched candidates are bound
            // after we have confirmed this candidate *and* any
            // associated guard; Binding them on `block` is too soon,
            // because that would be before we've checked the result
            // from the guard.
            //
            // But binding them on `arm_block` is *too late*, because
            // then all of the candidates for a single arm would be
            // bound in the same place, that would cause a case like:
            //
            // ```rust
            // match (30, 2) {
            //     (mut x, 1) | (2, mut x) if { true } => { ... }
            //     ...                                 // ^^^^^^^ (this is `arm_block`)
            // }
            // ```
            //
            // would yield a `arm_block` something like:
            //
            // ```
            // StorageLive(_4);        // _4 is `x`
            // _4 = &mut (_1.0: i32);  // this is handling `(mut x, 1)` case
            // _4 = &mut (_1.1: i32);  // this is handling `(2, mut x)` case
            // ```
            //
            // and that is clearly not correct.
            let post_guard_block = self.cfg.start_new_block();
            self.cfg.terminate(block, source_info,
                               TerminatorKind::if_(self.hir.tcx(), cond, post_guard_block,
                                                   false_edge_block));

            if autoref {
                self.bind_matched_candidate_for_arm_body(post_guard_block, &candidate.bindings);
            }

            self.cfg.terminate(post_guard_block, source_info,
                               TerminatorKind::Goto { target: arm_block });

            let otherwise = self.cfg.start_new_block();

            self.cfg.terminate(false_edge_block, source_info,
                               TerminatorKind::FalseEdges {
                                   real_target: otherwise,
                                   imaginary_targets:
                                       vec![candidate.next_candidate_pre_binding_block],
                               });
            Some(otherwise)
        } else {
            // (Here, it is not too early to bind the matched
            // candidate on `block`, because there is no guard result
            // that we have to inspect before we bind them.)
            self.bind_matched_candidate_for_arm_body(block, &candidate.bindings);
            self.cfg.terminate(block, candidate_source_info,
                               TerminatorKind::Goto { target: arm_block });
            None
        }
    }

    // Only called when all_pat_vars_are_implicit_refs_within_guards,
    // and thus all code/comments assume we are in that context.
    fn bind_matched_candidate_for_guard(&mut self,
                                        block: BasicBlock,
                                        bindings: &[Binding<'tcx>]) {
        debug!("bind_matched_candidate_for_guard(block={:?}, bindings={:?})",
               block, bindings);

        // Assign each of the bindings. Since we are binding for a
        // guard expression, this will never trigger moves out of the
        // candidate.
        let re_empty = self.hir.tcx().types.re_empty;
        for binding in bindings {
            let source_info = self.source_info(binding.span);

            // For each pattern ident P of type T, `ref_for_guard` is
            // a reference R: &T pointing to the location matched by
            // the pattern, and every occurrence of P within a guard
            // denotes *R.
            let ref_for_guard = self.storage_live_binding(
                block, binding.var_id, binding.span, RefWithinGuard);
            // Question: Why schedule drops if bindings are all
            // shared-&'s?  Answer: Because schedule_drop_for_binding
            // also emits StorageDead's for those locals.
            self.schedule_drop_for_binding(binding.var_id, binding.span, RefWithinGuard);
            match binding.binding_mode {
                BindingMode::ByValue => {
                    let rvalue = Rvalue::Ref(re_empty, BorrowKind::Shared, binding.source.clone());
                    self.cfg.push_assign(block, source_info, &ref_for_guard, rvalue);
                }
                BindingMode::ByRef(region, borrow_kind) => {
                    // Tricky business: For `ref id` and `ref mut id`
                    // patterns, we want `id` within the guard to
                    // correspond to a temp of type `& &T` or `& &mut
                    // T` (i.e. a "borrow of a borrow") that is
                    // implicitly dereferenced.
                    //
                    // To borrow a borrow, we need that inner borrow
                    // to point to. So, create a temp for the inner
                    // borrow, and then take a reference to it.
                    //
                    // Note: the temp created here is *not* the one
                    // used by the arm body itself. This eases
                    // observing two-phase borrow restrictions.
                    let val_for_guard = self.storage_live_binding(
                        block, binding.var_id, binding.span, ValWithinGuard);
                    self.schedule_drop_for_binding(binding.var_id, binding.span, ValWithinGuard);

                    // rust-lang/rust#27282: We reuse the two-phase
                    // borrow infrastructure so that the mutable
                    // borrow (whose mutabilty is *unusable* within
                    // the guard) does not conflict with the implicit
                    // borrow of the whole match input. See additional
                    // discussion on rust-lang/rust#49870.
                    let borrow_kind = match borrow_kind {
                        BorrowKind::Shared | BorrowKind::Unique => borrow_kind,
                        BorrowKind::Mut { .. } => BorrowKind::Mut { allow_two_phase_borrow: true },
                    };
                    let rvalue = Rvalue::Ref(region, borrow_kind, binding.source.clone());
                    self.cfg.push_assign(block, source_info, &val_for_guard, rvalue);
                    let rvalue = Rvalue::Ref(region, BorrowKind::Shared, val_for_guard);
                    self.cfg.push_assign(block, source_info, &ref_for_guard, rvalue);
                }
            }
        }
    }

    fn bind_matched_candidate_for_arm_body(&mut self,
                                           block: BasicBlock,
                                           bindings: &[Binding<'tcx>]) {
        debug!("bind_matched_candidate_for_arm_body(block={:?}, bindings={:?}", block, bindings);

        // Assign each of the bindings. This may trigger moves out of the candidate.
        for binding in bindings {
            let source_info = self.source_info(binding.span);
            let local = self.storage_live_binding(block, binding.var_id, binding.span,
                                                  OutsideGuard);
            self.schedule_drop_for_binding(binding.var_id, binding.span, OutsideGuard);
            let rvalue = match binding.binding_mode {
                BindingMode::ByValue => {
                    Rvalue::Use(self.consume_by_copy_or_move(binding.source.clone()))
                }
                BindingMode::ByRef(region, borrow_kind) => {
                    Rvalue::Ref(region, borrow_kind, binding.source.clone())
                }
            };
            self.cfg.push_assign(block, source_info, &local, rvalue);
        }
    }

    /// Each binding (`ref mut var`/`ref var`/`mut var`/`var`, where
    /// the bound `var` has type `T` in the arm body) in a pattern
    /// maps to *two* locals. The first local is a binding for
    /// occurrences of `var` in the guard, which will all have type
    /// `&T`. The second local is a binding for occurrences of `var`
    /// in the arm body, which will have type `T`.
    fn declare_binding(&mut self,
                       source_info: SourceInfo,
                       visibility_scope: SourceScope,
                       mutability: Mutability,
                       name: Name,
                       mode: BindingMode,
                       var_id: NodeId,
                       var_ty: Ty<'tcx>,
                       has_guard: ArmHasGuard)
    {
        debug!("declare_binding(var_id={:?}, name={:?}, mode={:?}, var_ty={:?}, \
                visibility_scope={:?}, source_info={:?})",
               var_id, name, mode, var_ty, visibility_scope, source_info);

        let tcx = self.hir.tcx();
        let binding_mode = match mode {
            BindingMode::ByValue => ty::BindingMode::BindByValue(mutability.into()),
            BindingMode::ByRef { .. } => ty::BindingMode::BindByReference(mutability.into()),
        };
        let local = LocalDecl::<'tcx> {
            mutability,
            ty: var_ty.clone(),
            name: Some(name),
            source_info,
            visibility_scope,
            internal: false,
            is_user_variable: Some(ClearCrossCrate::Set(BindingForm::Var(VarBindingForm {
                binding_mode,
                // hypothetically, `visit_bindings` could try to unzip
                // an outermost hir::Ty as we descend, matching up
                // idents in pat; but complex w/ unclear UI payoff.
                // Instead, just abandon providing diagnostic info.
                opt_ty_info: None,
            }))),
        };
        let for_arm_body = self.local_decls.push(local.clone());
        let locals = if has_guard.0 && tcx.all_pat_vars_are_implicit_refs_within_guards() {
            let val_for_guard =  self.local_decls.push(local);
            let ref_for_guard = self.local_decls.push(LocalDecl::<'tcx> {
                mutability,
                ty: tcx.mk_imm_ref(tcx.types.re_empty, var_ty),
                name: Some(name),
                source_info,
                visibility_scope,
                // FIXME: should these secretly injected ref_for_guard's be marked as `internal`?
                internal: false,
                is_user_variable: None,
            });
            LocalsForNode::Three { val_for_guard, ref_for_guard, for_arm_body }
        } else {
            LocalsForNode::One(for_arm_body)
        };
        debug!("declare_binding: vars={:?}", locals);
        self.var_indices.insert(var_id, locals);
    }
}
