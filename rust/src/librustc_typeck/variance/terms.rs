// Representing terms
//
// Terms are structured as a straightforward tree. Rather than rely on
// GC, we allocate terms out of a bounded arena (the lifetime of this
// arena is the lifetime 'a that is threaded around).
//
// We assign a unique index to each type/region parameter whose variance
// is to be inferred. We refer to such variables as "inferreds". An
// `InferredIndex` is a newtype'd int representing the index of such
// a variable.

use arena::TypedArena;
use rustc::ty::{self, TyCtxt};
use std::fmt;
use syntax::ast;
use rustc::hir;
use rustc::hir::itemlikevisit::ItemLikeVisitor;
use util::nodemap::NodeMap;

use self::VarianceTerm::*;

pub type VarianceTermPtr<'a> = &'a VarianceTerm<'a>;

#[derive(Copy, Clone, Debug)]
pub struct InferredIndex(pub usize);

#[derive(Copy, Clone)]
pub enum VarianceTerm<'a> {
    ConstantTerm(ty::Variance),
    TransformTerm(VarianceTermPtr<'a>, VarianceTermPtr<'a>),
    InferredTerm(InferredIndex),
}

impl<'a> fmt::Debug for VarianceTerm<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ConstantTerm(c1) => write!(f, "{:?}", c1),
            TransformTerm(v1, v2) => write!(f, "({:?} \u{00D7} {:?})", v1, v2),
            InferredTerm(id) => {
                write!(f, "[{}]", {
                    let InferredIndex(i) = id;
                    i
                })
            }
        }
    }
}

// The first pass over the crate simply builds up the set of inferreds.

pub struct TermsContext<'a, 'tcx: 'a> {
    pub tcx: TyCtxt<'a, 'tcx, 'tcx>,
    pub arena: &'a TypedArena<VarianceTerm<'a>>,

    // For marker types, UnsafeCell, and other lang items where
    // variance is hardcoded, records the item-id and the hardcoded
    // variance.
    pub lang_items: Vec<(ast::NodeId, Vec<ty::Variance>)>,

    // Maps from the node id of an item to the first inferred index
    // used for its type & region parameters.
    pub inferred_starts: NodeMap<InferredIndex>,

    // Maps from an InferredIndex to the term for that variable.
    pub inferred_terms: Vec<VarianceTermPtr<'a>>,
}

pub fn determine_parameters_to_be_inferred<'a, 'tcx>(tcx: TyCtxt<'a, 'tcx, 'tcx>,
                                                     arena: &'a mut TypedArena<VarianceTerm<'a>>)
                                                     -> TermsContext<'a, 'tcx> {
    let mut terms_cx = TermsContext {
        tcx,
        arena,
        inferred_starts: Default::default(),
        inferred_terms: vec![],

        lang_items: lang_items(tcx),
    };

    // See the following for a discussion on dep-graph management.
    //
    // - https://rust-lang.github.io/rustc-guide/query.html
    // - https://rust-lang.github.io/rustc-guide/variance.html
    tcx.hir().krate().visit_all_item_likes(&mut terms_cx);

    terms_cx
}

fn lang_items(tcx: TyCtxt) -> Vec<(ast::NodeId, Vec<ty::Variance>)> {
    let lang_items = tcx.lang_items();
    let all = vec![
        (lang_items.phantom_data(), vec![ty::Covariant]),
        (lang_items.unsafe_cell_type(), vec![ty::Invariant]),
        ];

    all.into_iter() // iterating over (Option<DefId>, Variance)
       .filter(|&(ref d,_)| d.is_some())
       .map(|(d, v)| (d.unwrap(), v)) // (DefId, Variance)
       .filter_map(|(d, v)| tcx.hir().as_local_node_id(d).map(|n| (n, v))) // (NodeId, Variance)
       .collect()
}

impl<'a, 'tcx> TermsContext<'a, 'tcx> {
    fn add_inferreds_for_item(&mut self, id: ast::NodeId) {
        let tcx = self.tcx;
        let def_id = tcx.hir().local_def_id(id);
        let count = tcx.generics_of(def_id).count();

        if count == 0 {
            return;
        }

        // Record the start of this item's inferreds.
        let start = self.inferred_terms.len();
        let newly_added = self.inferred_starts.insert(id, InferredIndex(start)).is_none();
        assert!(newly_added);

        // N.B., in the code below for writing the results back into the
        // `CrateVariancesMap`, we rely on the fact that all inferreds
        // for a particular item are assigned continuous indices.

        let arena = self.arena;
        self.inferred_terms.extend((start..start+count).map(|i| {
            &*arena.alloc(InferredTerm(InferredIndex(i)))
        }));
    }
}

impl<'a, 'tcx, 'v> ItemLikeVisitor<'v> for TermsContext<'a, 'tcx> {
    fn visit_item(&mut self, item: &hir::Item) {
        debug!("add_inferreds for item {}",
               self.tcx.hir().node_to_string(item.id));

        match item.node {
            hir::ItemKind::Struct(ref struct_def, _) |
            hir::ItemKind::Union(ref struct_def, _) => {
                self.add_inferreds_for_item(item.id);

                if let hir::VariantData::Tuple(..) = *struct_def {
                    self.add_inferreds_for_item(struct_def.id());
                }
            }

            hir::ItemKind::Enum(ref enum_def, _) => {
                self.add_inferreds_for_item(item.id);

                for variant in &enum_def.variants {
                    if let hir::VariantData::Tuple(..) = variant.node.data {
                        self.add_inferreds_for_item(variant.node.data.id());
                    }
                }
            }

            hir::ItemKind::Fn(..) => {
                self.add_inferreds_for_item(item.id);
            }

            hir::ItemKind::ForeignMod(ref foreign_mod) => {
                for foreign_item in &foreign_mod.items {
                    if let hir::ForeignItemKind::Fn(..) = foreign_item.node {
                        self.add_inferreds_for_item(foreign_item.id);
                    }
                }
            }

            _ => {}
        }
    }

    fn visit_trait_item(&mut self, trait_item: &hir::TraitItem) {
        if let hir::TraitItemKind::Method(..) = trait_item.node {
            self.add_inferreds_for_item(trait_item.id);
        }
    }

    fn visit_impl_item(&mut self, impl_item: &hir::ImplItem) {
        if let hir::ImplItemKind::Method(..) = impl_item.node {
            self.add_inferreds_for_item(impl_item.id);
        }
    }
}
