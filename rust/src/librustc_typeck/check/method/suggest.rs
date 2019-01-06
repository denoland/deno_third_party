//! Give useful errors and suggestions to users when an item can't be
//! found or is otherwise invalid.

use check::FnCtxt;
use errors::{Applicability, DiagnosticBuilder};
use middle::lang_items::FnOnceTraitLangItem;
use namespace::Namespace;
use rustc_data_structures::sync::Lrc;
use rustc::hir::{self, ExprKind, Node, QPath};
use rustc::hir::def::Def;
use rustc::hir::def_id::{CRATE_DEF_INDEX, LOCAL_CRATE, DefId};
use rustc::hir::map as hir_map;
use rustc::hir::print;
use rustc::infer::type_variable::TypeVariableOrigin;
use rustc::traits::Obligation;
use rustc::ty::{self, Adt, Ty, TyCtxt, ToPolyTraitRef, ToPredicate, TypeFoldable};
use rustc::ty::item_path::with_crate_prefix;
use util::nodemap::FxHashSet;
use syntax_pos::{Span, FileName};
use syntax::ast;
use syntax::util::lev_distance::find_best_match_for_name;

use std::cmp::Ordering;

use super::{MethodError, NoMatchData, CandidateSource};
use super::probe::Mode;

impl<'a, 'gcx, 'tcx> FnCtxt<'a, 'gcx, 'tcx> {
    fn is_fn_ty(&self, ty: &Ty<'tcx>, span: Span) -> bool {
        let tcx = self.tcx;
        match ty.sty {
            // Not all of these (e.g., unsafe fns) implement `FnOnce`,
            // so we look for these beforehand.
            ty::Closure(..) |
            ty::FnDef(..) |
            ty::FnPtr(_) => true,
            // If it's not a simple function, look for things which implement `FnOnce`.
            _ => {
                let fn_once = match tcx.lang_items().require(FnOnceTraitLangItem) {
                    Ok(fn_once) => fn_once,
                    Err(..) => return false,
                };

                self.autoderef(span, ty).any(|(ty, _)| {
                    self.probe(|_| {
                        let fn_once_substs = tcx.mk_substs_trait(ty, &[
                            self.next_ty_var(TypeVariableOrigin::MiscVariable(span)).into()
                        ]);
                        let trait_ref = ty::TraitRef::new(fn_once, fn_once_substs);
                        let poly_trait_ref = trait_ref.to_poly_trait_ref();
                        let obligation =
                            Obligation::misc(span,
                                             self.body_id,
                                             self.param_env,
                                             poly_trait_ref.to_predicate());
                        self.predicate_may_hold(&obligation)
                    })
                })
            }
        }
    }

    pub fn report_method_error<'b>(&self,
                                   span: Span,
                                   rcvr_ty: Ty<'tcx>,
                                   item_name: ast::Ident,
                                   source: SelfSource<'b>,
                                   error: MethodError<'tcx>,
                                   args: Option<&'gcx [hir::Expr]>) {
        // Avoid suggestions when we don't know what's going on.
        if rcvr_ty.references_error() {
            return;
        }

        let report_candidates = |err: &mut DiagnosticBuilder, mut sources: Vec<CandidateSource>| {
            sources.sort();
            sources.dedup();
            // Dynamic limit to avoid hiding just one candidate, which is silly.
            let limit = if sources.len() == 5 { 5 } else { 4 };

            for (idx, source) in sources.iter().take(limit).enumerate() {
                match *source {
                    CandidateSource::ImplSource(impl_did) => {
                        // Provide the best span we can. Use the item, if local to crate, else
                        // the impl, if local to crate (item may be defaulted), else nothing.
                        let item = self.associated_item(impl_did, item_name, Namespace::Value)
                            .or_else(|| {
                                self.associated_item(
                                    self.tcx.impl_trait_ref(impl_did).unwrap().def_id,
                                    item_name,
                                    Namespace::Value,
                                )
                            }).unwrap();
                        let note_span = self.tcx.hir().span_if_local(item.def_id).or_else(|| {
                            self.tcx.hir().span_if_local(impl_did)
                        });

                        let impl_ty = self.impl_self_ty(span, impl_did).ty;

                        let insertion = match self.tcx.impl_trait_ref(impl_did) {
                            None => String::new(),
                            Some(trait_ref) => {
                                format!(" of the trait `{}`",
                                        self.tcx.item_path_str(trait_ref.def_id))
                            }
                        };

                        let note_str = if sources.len() > 1 {
                            format!("candidate #{} is defined in an impl{} for the type `{}`",
                                    idx + 1,
                                    insertion,
                                    impl_ty)
                        } else {
                            format!("the candidate is defined in an impl{} for the type `{}`",
                                    insertion,
                                    impl_ty)
                        };
                        if let Some(note_span) = note_span {
                            // We have a span pointing to the method. Show note with snippet.
                            err.span_note(self.tcx.sess.source_map().def_span(note_span),
                                          &note_str);
                        } else {
                            err.note(&note_str);
                        }
                    }
                    CandidateSource::TraitSource(trait_did) => {
                        let item = self
                            .associated_item(trait_did, item_name, Namespace::Value)
                            .unwrap();
                        let item_span = self.tcx.sess.source_map()
                            .def_span(self.tcx.def_span(item.def_id));
                        if sources.len() > 1 {
                            span_note!(err,
                                       item_span,
                                       "candidate #{} is defined in the trait `{}`",
                                       idx + 1,
                                       self.tcx.item_path_str(trait_did));
                        } else {
                            span_note!(err,
                                       item_span,
                                       "the candidate is defined in the trait `{}`",
                                       self.tcx.item_path_str(trait_did));
                        }
                        err.help(&format!("to disambiguate the method call, write `{}::{}({}{})` \
                                          instead",
                                          self.tcx.item_path_str(trait_did),
                                          item_name,
                                          if rcvr_ty.is_region_ptr() && args.is_some() {
                                              if rcvr_ty.is_mutable_pointer() {
                                                  "&mut "
                                              } else {
                                                  "&"
                                              }
                                          } else {
                                              ""
                                          },
                                          args.map(|arg| arg.iter()
                                              .map(|arg| print::to_string(print::NO_ANN,
                                                                          |s| s.print_expr(arg)))
                                              .collect::<Vec<_>>()
                                              .join(", ")).unwrap_or_else(|| "...".to_owned())));
                    }
                }
            }
            if sources.len() > limit {
                err.note(&format!("and {} others", sources.len() - limit));
            }
        };

        match error {
            MethodError::NoMatch(NoMatchData {
                static_candidates: static_sources,
                unsatisfied_predicates,
                out_of_scope_traits,
                lev_candidate,
                mode,
            }) => {
                let tcx = self.tcx;

                let actual = self.resolve_type_vars_if_possible(&rcvr_ty);
                let ty_str = self.ty_to_string(actual);
                let is_method = mode == Mode::MethodCall;
                let mut suggestion = None;
                let item_kind = if is_method {
                    "method"
                } else if actual.is_enum() {
                    if let Adt(ref adt_def, _) = actual.sty {
                        let names = adt_def.variants.iter().map(|s| &s.ident.name);
                        suggestion = find_best_match_for_name(names,
                                                              &item_name.as_str(),
                                                              None);
                    }
                    "variant"
                } else {
                    match (item_name.as_str().chars().next(), actual.is_fresh_ty()) {
                        (Some(name), false) if name.is_lowercase() => {
                            "function or associated item"
                        }
                        (Some(_), false) => "associated item",
                        (Some(_), true) | (None, false) => {
                            "variant or associated item"
                        }
                        (None, true) => "variant",
                    }
                };
                let mut err = if !actual.references_error() {
                    // Suggest clamping down the type if the method that is being attempted to
                    // be used exists at all, and the type is an ambiuous numeric type
                    // ({integer}/{float}).
                    let mut candidates = all_traits(self.tcx)
                        .into_iter()
                        .filter_map(|info|
                            self.associated_item(info.def_id, item_name, Namespace::Value)
                        );
                    if let (true, false, SelfSource::MethodCall(expr), Some(_)) =
                           (actual.is_numeric(),
                            actual.has_concrete_skeleton(),
                            source,
                            candidates.next()) {
                        let mut err = struct_span_err!(
                            tcx.sess,
                            span,
                            E0689,
                            "can't call {} `{}` on ambiguous numeric type `{}`",
                            item_kind,
                            item_name,
                            ty_str
                        );
                        let concrete_type = if actual.is_integral() {
                            "i32"
                        } else {
                            "f32"
                        };
                        match expr.node {
                            ExprKind::Lit(ref lit) => {
                                // numeric literal
                                let snippet = tcx.sess.source_map().span_to_snippet(lit.span)
                                    .unwrap_or_else(|_| "<numeric literal>".to_owned());

                                err.span_suggestion_with_applicability(
                                                    lit.span,
                                                    &format!("you must specify a concrete type for \
                                                              this numeric value, like `{}`",
                                                             concrete_type),
                                                    format!("{}_{}",
                                                            snippet,
                                                            concrete_type),
                                                    Applicability::MaybeIncorrect,
                                );
                            }
                            ExprKind::Path(ref qpath) => {
                                // local binding
                                if let &QPath::Resolved(_, ref path) = &qpath {
                                    if let hir::def::Def::Local(node_id) = path.def {
                                        let span = tcx.hir().span(node_id);
                                        let snippet = tcx.sess.source_map().span_to_snippet(span)
                                            .unwrap();
                                        let filename = tcx.sess.source_map().span_to_filename(span);

                                        let parent_node = self.tcx.hir().get(
                                            self.tcx.hir().get_parent_node(node_id),
                                        );
                                        let msg = format!(
                                            "you must specify a type for this binding, like `{}`",
                                            concrete_type,
                                        );

                                        match (filename, parent_node) {
                                            (FileName::Real(_), Node::Local(hir::Local {
                                                source: hir::LocalSource::Normal,
                                                ty,
                                                ..
                                            })) => {
                                                err.span_suggestion_with_applicability(
                                                    // account for `let x: _ = 42;`
                                                    //                  ^^^^
                                                    span.to(ty.as_ref().map(|ty| ty.span)
                                                        .unwrap_or(span)),
                                                    &msg,
                                                    format!("{}: {}", snippet, concrete_type),
                                                    Applicability::MaybeIncorrect,
                                                );
                                            }
                                            _ => {
                                                err.span_label(span, msg);
                                            }
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                        err.emit();
                        return;
                    } else {
                        let mut err = struct_span_err!(
                            tcx.sess,
                            item_name.span,
                            E0599,
                            "no {} named `{}` found for type `{}` in the current scope",
                            item_kind,
                            item_name,
                            ty_str
                        );
                        if let Some(suggestion) = suggestion {
                            // enum variant
                            err.help(&format!("did you mean `{}`?", suggestion));
                        }
                        err
                    }
                } else {
                    tcx.sess.diagnostic().struct_dummy()
                };

                if let Some(def) = actual.ty_adt_def() {
                    if let Some(full_sp) = tcx.hir().span_if_local(def.did) {
                        let def_sp = tcx.sess.source_map().def_span(full_sp);
                        err.span_label(def_sp, format!("{} `{}` not found {}",
                                                       item_kind,
                                                       item_name,
                                                       if def.is_enum() && !is_method {
                                                           "here"
                                                       } else {
                                                           "for this"
                                                       }));
                    }
                }

                // If the method name is the name of a field with a function or closure type,
                // give a helping note that it has to be called as `(x.f)(...)`.
                if let SelfSource::MethodCall(expr) = source {
                    for (ty, _) in self.autoderef(span, rcvr_ty) {
                        if let ty::Adt(def, substs) = ty.sty {
                            if !def.is_enum() {
                                let variant = &def.non_enum_variant();
                                if let Some(index) = self.tcx.find_field_index(item_name, variant) {
                                    let field = &variant.fields[index];
                                    let snippet = tcx.sess.source_map().span_to_snippet(expr.span);
                                    let expr_string = match snippet {
                                        Ok(expr_string) => expr_string,
                                        _ => "s".into(), // Default to a generic placeholder for the
                                                         // expression when we can't generate a
                                                         // string snippet.
                                    };

                                    let field_ty = field.ty(tcx, substs);
                                    let scope = self.tcx.hir().get_module_parent(self.body_id);
                                    if field.vis.is_accessible_from(scope, self.tcx) {
                                        if self.is_fn_ty(&field_ty, span) {
                                            err.help(&format!("use `({0}.{1})(...)` if you \
                                                               meant to call the function \
                                                               stored in the `{1}` field",
                                                              expr_string,
                                                              item_name));
                                        } else {
                                            err.help(&format!("did you mean to write `{0}.{1}` \
                                                               instead of `{0}.{1}(...)`?",
                                                              expr_string,
                                                              item_name));
                                        }
                                        err.span_label(span, "field, not a method");
                                    } else {
                                        err.span_label(span, "private field, not a method");
                                    }
                                    break;
                                }
                            }
                        }
                    }
                } else {
                    err.span_label(span, format!("{} not found in `{}`", item_kind, ty_str));
                }

                if self.is_fn_ty(&rcvr_ty, span) {
                    macro_rules! report_function {
                        ($span:expr, $name:expr) => {
                            err.note(&format!("{} is a function, perhaps you wish to call it",
                                              $name));
                        }
                    }

                    if let SelfSource::MethodCall(expr) = source {
                        if let Ok(expr_string) = tcx.sess.source_map().span_to_snippet(expr.span) {
                            report_function!(expr.span, expr_string);
                        } else if let ExprKind::Path(QPath::Resolved(_, ref path)) =
                            expr.node
                        {
                            if let Some(segment) = path.segments.last() {
                                report_function!(expr.span, segment.ident);
                            }
                        }
                    }
                }

                if !static_sources.is_empty() {
                    err.note("found the following associated functions; to be used as methods, \
                              functions must have a `self` parameter");
                    err.span_label(span, "this is an associated function, not a method");
                }
                if static_sources.len() == 1 {
                    if let SelfSource::MethodCall(expr) = source {
                        err.span_suggestion_with_applicability(expr.span.to(span),
                                            "use associated function syntax instead",
                                            format!("{}::{}",
                                                    self.ty_to_string(actual),
                                                    item_name),
                                            Applicability::MachineApplicable);
                    } else {
                        err.help(&format!("try with `{}::{}`",
                                          self.ty_to_string(actual), item_name));
                    }

                    report_candidates(&mut err, static_sources);
                } else if static_sources.len() > 1 {
                    report_candidates(&mut err, static_sources);
                }

                if !unsatisfied_predicates.is_empty() {
                    let mut bound_list = unsatisfied_predicates.iter()
                        .map(|p| format!("`{} : {}`", p.self_ty(), p))
                        .collect::<Vec<_>>();
                    bound_list.sort();
                    bound_list.dedup();  // #35677
                    let bound_list = bound_list.join("\n");
                    err.note(&format!("the method `{}` exists but the following trait bounds \
                                       were not satisfied:\n{}",
                                      item_name,
                                      bound_list));
                }

                if actual.is_numeric() && actual.is_fresh() {

                } else {
                    self.suggest_traits_to_import(&mut err,
                                                  span,
                                                  rcvr_ty,
                                                  item_name,
                                                  source,
                                                  out_of_scope_traits);
                }

                if let Some(lev_candidate) = lev_candidate {
                    err.help(&format!("did you mean `{}`?", lev_candidate.ident));
                }
                err.emit();
            }

            MethodError::Ambiguity(sources) => {
                let mut err = struct_span_err!(self.sess(),
                                               span,
                                               E0034,
                                               "multiple applicable items in scope");
                err.span_label(span, format!("multiple `{}` found", item_name));

                report_candidates(&mut err, sources);
                err.emit();
            }

            MethodError::PrivateMatch(def, out_of_scope_traits) => {
                let mut err = struct_span_err!(self.tcx.sess, span, E0624,
                                               "{} `{}` is private", def.kind_name(), item_name);
                self.suggest_valid_traits(&mut err, out_of_scope_traits);
                err.emit();
            }

            MethodError::IllegalSizedBound(candidates) => {
                let msg = format!("the `{}` method cannot be invoked on a trait object", item_name);
                let mut err = self.sess().struct_span_err(span, &msg);
                if !candidates.is_empty() {
                    let help = format!("{an}other candidate{s} {were} found in the following \
                                        trait{s}, perhaps add a `use` for {one_of_them}:",
                                    an = if candidates.len() == 1 {"an" } else { "" },
                                    s = if candidates.len() == 1 { "" } else { "s" },
                                    were = if candidates.len() == 1 { "was" } else { "were" },
                                    one_of_them = if candidates.len() == 1 {
                                        "it"
                                    } else {
                                        "one_of_them"
                                    });
                    self.suggest_use_candidates(&mut err, help, candidates);
                }
                err.emit();
            }

            MethodError::BadReturnType => {
                bug!("no return type expectations but got BadReturnType")
            }
        }
    }

    fn suggest_use_candidates(&self,
                              err: &mut DiagnosticBuilder,
                              mut msg: String,
                              candidates: Vec<DefId>) {
        let module_did = self.tcx.hir().get_module_parent(self.body_id);
        let module_id = self.tcx.hir().as_local_node_id(module_did).unwrap();
        let krate = self.tcx.hir().krate();
        let (span, found_use) = UsePlacementFinder::check(self.tcx, krate, module_id);
        if let Some(span) = span {
            let path_strings = candidates.iter().map(|did| {
                // Produce an additional newline to separate the new use statement
                // from the directly following item.
                let additional_newline = if found_use {
                    ""
                } else {
                    "\n"
                };
                format!(
                    "use {};\n{}",
                    with_crate_prefix(|| self.tcx.item_path_str(*did)),
                    additional_newline
                )
            });

            err.span_suggestions_with_applicability(
                                                    span,
                                                    &msg,
                                                    path_strings,
                                                    Applicability::MaybeIncorrect,
            );
        } else {
            let limit = if candidates.len() == 5 { 5 } else { 4 };
            for (i, trait_did) in candidates.iter().take(limit).enumerate() {
                if candidates.len() > 1 {
                    msg.push_str(
                        &format!(
                            "\ncandidate #{}: `use {};`",
                            i + 1,
                            with_crate_prefix(|| self.tcx.item_path_str(*trait_did))
                        )
                    );
                } else {
                    msg.push_str(
                        &format!(
                            "\n`use {};`",
                            with_crate_prefix(|| self.tcx.item_path_str(*trait_did))
                        )
                    );
                }
            }
            if candidates.len() > limit {
                msg.push_str(&format!("\nand {} others", candidates.len() - limit));
            }
            err.note(&msg[..]);
        }
    }

    fn suggest_valid_traits(&self,
                            err: &mut DiagnosticBuilder,
                            valid_out_of_scope_traits: Vec<DefId>) -> bool {
        if !valid_out_of_scope_traits.is_empty() {
            let mut candidates = valid_out_of_scope_traits;
            candidates.sort();
            candidates.dedup();
            err.help("items from traits can only be used if the trait is in scope");
            let msg = format!("the following {traits_are} implemented but not in scope, \
                               perhaps add a `use` for {one_of_them}:",
                            traits_are = if candidates.len() == 1 {
                                "trait is"
                            } else {
                                "traits are"
                            },
                            one_of_them = if candidates.len() == 1 {
                                "it"
                            } else {
                                "one of them"
                            });

            self.suggest_use_candidates(err, msg, candidates);
            true
        } else {
            false
        }
    }

    fn suggest_traits_to_import<'b>(&self,
                                    err: &mut DiagnosticBuilder,
                                    span: Span,
                                    rcvr_ty: Ty<'tcx>,
                                    item_name: ast::Ident,
                                    source: SelfSource<'b>,
                                    valid_out_of_scope_traits: Vec<DefId>) {
        if self.suggest_valid_traits(err, valid_out_of_scope_traits) {
            return;
        }

        let type_is_local = self.type_derefs_to_local(span, rcvr_ty, source);

        // There are no traits implemented, so lets suggest some traits to
        // implement, by finding ones that have the item name, and are
        // legal to implement.
        let mut candidates = all_traits(self.tcx)
            .into_iter()
            .filter(|info| {
                // We approximate the coherence rules to only suggest
                // traits that are legal to implement by requiring that
                // either the type or trait is local. Multi-dispatch means
                // this isn't perfect (that is, there are cases when
                // implementing a trait would be legal but is rejected
                // here).
                (type_is_local || info.def_id.is_local()) &&
                    self.associated_item(info.def_id, item_name, Namespace::Value)
                        .filter(|item| {
                            // We only want to suggest public or local traits (#45781).
                            item.vis == ty::Visibility::Public || info.def_id.is_local()
                        })
                        .is_some()
            })
            .collect::<Vec<_>>();

        if !candidates.is_empty() {
            // Sort from most relevant to least relevant.
            candidates.sort_by(|a, b| a.cmp(b).reverse());
            candidates.dedup();

            // FIXME #21673: this help message could be tuned to the case
            // of a type parameter: suggest adding a trait bound rather
            // than implementing.
            err.help("items from traits can only be used if the trait is implemented and in scope");
            let mut msg = format!("the following {traits_define} an item `{name}`, \
                                   perhaps you need to implement {one_of_them}:",
                                  traits_define = if candidates.len() == 1 {
                                      "trait defines"
                                  } else {
                                      "traits define"
                                  },
                                  one_of_them = if candidates.len() == 1 {
                                      "it"
                                  } else {
                                      "one of them"
                                  },
                                  name = item_name);

            for (i, trait_info) in candidates.iter().enumerate() {
                msg.push_str(&format!("\ncandidate #{}: `{}`",
                                      i + 1,
                                      self.tcx.item_path_str(trait_info.def_id)));
            }
            err.note(&msg[..]);
        }
    }

    /// Checks whether there is a local type somewhere in the chain of
    /// autoderefs of `rcvr_ty`.
    fn type_derefs_to_local(&self,
                            span: Span,
                            rcvr_ty: Ty<'tcx>,
                            source: SelfSource) -> bool {
        fn is_local(ty: Ty) -> bool {
            match ty.sty {
                ty::Adt(def, _) => def.did.is_local(),
                ty::Foreign(did) => did.is_local(),

                ty::Dynamic(ref tr, ..) =>
                    tr.principal().map(|d| d.def_id().is_local()).unwrap_or(false),

                ty::Param(_) => true,

                // Everything else (primitive types, etc.) is effectively
                // non-local (there are "edge" cases, e.g., `(LocalType,)`, but
                // the noise from these sort of types is usually just really
                // annoying, rather than any sort of help).
                _ => false,
            }
        }

        // This occurs for UFCS desugaring of `T::method`, where there is no
        // receiver expression for the method call, and thus no autoderef.
        if let SelfSource::QPath(_) = source {
            return is_local(self.resolve_type_vars_with_obligations(rcvr_ty));
        }

        self.autoderef(span, rcvr_ty).any(|(ty, _)| is_local(ty))
    }
}

#[derive(Copy, Clone)]
pub enum SelfSource<'a> {
    QPath(&'a hir::Ty),
    MethodCall(&'a hir::Expr /* rcvr */),
}

#[derive(Copy, Clone)]
pub struct TraitInfo {
    pub def_id: DefId,
}

impl PartialEq for TraitInfo {
    fn eq(&self, other: &TraitInfo) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for TraitInfo {}
impl PartialOrd for TraitInfo {
    fn partial_cmp(&self, other: &TraitInfo) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for TraitInfo {
    fn cmp(&self, other: &TraitInfo) -> Ordering {
        // Local crates are more important than remote ones (local:
        // `cnum == 0`), and otherwise we throw in the defid for totality.

        let lhs = (other.def_id.krate, other.def_id);
        let rhs = (self.def_id.krate, self.def_id);
        lhs.cmp(&rhs)
    }
}

/// Retrieve all traits in this crate and any dependent crates.
pub fn all_traits<'a, 'gcx, 'tcx>(tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Vec<TraitInfo> {
    tcx.all_traits(LOCAL_CRATE).iter().map(|&def_id| TraitInfo { def_id }).collect()
}

/// Compute all traits in this crate and any dependent crates.
fn compute_all_traits<'a, 'gcx, 'tcx>(tcx: TyCtxt<'a, 'gcx, 'tcx>) -> Vec<DefId> {
    use hir::itemlikevisit;

    let mut traits = vec![];

    // Crate-local:

    struct Visitor<'a, 'tcx: 'a> {
        map: &'a hir_map::Map<'tcx>,
        traits: &'a mut Vec<DefId>,
    }

    impl<'v, 'a, 'tcx> itemlikevisit::ItemLikeVisitor<'v> for Visitor<'a, 'tcx> {
        fn visit_item(&mut self, i: &'v hir::Item) {
            if let hir::ItemKind::Trait(..) = i.node {
                let def_id = self.map.local_def_id(i.id);
                self.traits.push(def_id);
            }
        }

        fn visit_trait_item(&mut self, _trait_item: &hir::TraitItem) {}

        fn visit_impl_item(&mut self, _impl_item: &hir::ImplItem) {}
    }

    tcx.hir().krate().visit_all_item_likes(&mut Visitor {
        map: &tcx.hir(),
        traits: &mut traits,
    });

    // Cross-crate:

    let mut external_mods = FxHashSet::default();
    fn handle_external_def(tcx: TyCtxt,
                           traits: &mut Vec<DefId>,
                           external_mods: &mut FxHashSet<DefId>,
                           def: Def) {
        let def_id = def.def_id();
        match def {
            Def::Trait(..) => {
                traits.push(def_id);
            }
            Def::Mod(..) => {
                if !external_mods.insert(def_id) {
                    return;
                }
                for child in tcx.item_children(def_id).iter() {
                    handle_external_def(tcx, traits, external_mods, child.def)
                }
            }
            _ => {}
        }
    }
    for &cnum in tcx.crates().iter() {
        let def_id = DefId {
            krate: cnum,
            index: CRATE_DEF_INDEX,
        };
        handle_external_def(tcx, &mut traits, &mut external_mods, Def::Mod(def_id));
    }

    traits
}

pub fn provide(providers: &mut ty::query::Providers) {
    providers.all_traits = |tcx, cnum| {
        assert_eq!(cnum, LOCAL_CRATE);
        Lrc::new(compute_all_traits(tcx))
    }
}

struct UsePlacementFinder<'a, 'tcx: 'a, 'gcx: 'tcx> {
    target_module: ast::NodeId,
    span: Option<Span>,
    found_use: bool,
    tcx: TyCtxt<'a, 'gcx, 'tcx>
}

impl<'a, 'tcx, 'gcx> UsePlacementFinder<'a, 'tcx, 'gcx> {
    fn check(
        tcx: TyCtxt<'a, 'gcx, 'tcx>,
        krate: &'tcx hir::Crate,
        target_module: ast::NodeId,
    ) -> (Option<Span>, bool) {
        let mut finder = UsePlacementFinder {
            target_module,
            span: None,
            found_use: false,
            tcx,
        };
        hir::intravisit::walk_crate(&mut finder, krate);
        (finder.span, finder.found_use)
    }
}

impl<'a, 'tcx, 'gcx> hir::intravisit::Visitor<'tcx> for UsePlacementFinder<'a, 'tcx, 'gcx> {
    fn visit_mod(
        &mut self,
        module: &'tcx hir::Mod,
        _: Span,
        node_id: ast::NodeId,
    ) {
        if self.span.is_some() {
            return;
        }
        if node_id != self.target_module {
            hir::intravisit::walk_mod(self, module, node_id);
            return;
        }
        // Find a `use` statement.
        for item_id in &module.item_ids {
            let item = self.tcx.hir().expect_item(item_id.id);
            match item.node {
                hir::ItemKind::Use(..) => {
                    // Don't suggest placing a `use` before the prelude
                    // import or other generated ones.
                    if item.span.ctxt().outer().expn_info().is_none() {
                        self.span = Some(item.span.shrink_to_lo());
                        self.found_use = true;
                        return;
                    }
                },
                // Don't place `use` before `extern crate`...
                hir::ItemKind::ExternCrate(_) => {}
                // ...but do place them before the first other item.
                _ => if self.span.map_or(true, |span| item.span < span ) {
                    if item.span.ctxt().outer().expn_info().is_none() {
                        // Don't insert between attributes and an item.
                        if item.attrs.is_empty() {
                            self.span = Some(item.span.shrink_to_lo());
                        } else {
                            // Find the first attribute on the item.
                            for attr in &item.attrs {
                                if self.span.map_or(true, |span| attr.span < span) {
                                    self.span = Some(attr.span.shrink_to_lo());
                                }
                            }
                        }
                    }
                },
            }
        }
    }

    fn nested_visit_map<'this>(
        &'this mut self
    ) -> hir::intravisit::NestedVisitorMap<'this, 'tcx> {
        hir::intravisit::NestedVisitorMap::None
    }
}
