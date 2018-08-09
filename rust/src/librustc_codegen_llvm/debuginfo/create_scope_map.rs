// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use super::{FunctionDebugContext, FunctionDebugContextData};
use super::metadata::file_metadata;
use super::utils::{DIB, span_start};

use llvm;
use llvm::debuginfo::DIScope;
use common::CodegenCx;
use rustc::mir::{Mir, SourceScope};

use libc::c_uint;
use std::ptr;

use syntax_pos::Pos;

use rustc_data_structures::bitvec::BitVector;
use rustc_data_structures::indexed_vec::{Idx, IndexVec};

use syntax_pos::BytePos;

#[derive(Clone, Copy, Debug)]
pub struct MirDebugScope {
    pub scope_metadata: DIScope,
    // Start and end offsets of the file to which this DIScope belongs.
    // These are used to quickly determine whether some span refers to the same file.
    pub file_start_pos: BytePos,
    pub file_end_pos: BytePos,
}

impl MirDebugScope {
    pub fn is_valid(&self) -> bool {
        !self.scope_metadata.is_null()
    }
}

/// Produce DIScope DIEs for each MIR Scope which has variables defined in it.
/// If debuginfo is disabled, the returned vector is empty.
pub fn create_mir_scopes(cx: &CodegenCx, mir: &Mir, debug_context: &FunctionDebugContext)
    -> IndexVec<SourceScope, MirDebugScope> {
    let null_scope = MirDebugScope {
        scope_metadata: ptr::null_mut(),
        file_start_pos: BytePos(0),
        file_end_pos: BytePos(0)
    };
    let mut scopes = IndexVec::from_elem(null_scope, &mir.source_scopes);

    let debug_context = match *debug_context {
        FunctionDebugContext::RegularContext(ref data) => data,
        FunctionDebugContext::DebugInfoDisabled |
        FunctionDebugContext::FunctionWithoutDebugInfo => {
            return scopes;
        }
    };

    // Find all the scopes with variables defined in them.
    let mut has_variables = BitVector::new(mir.source_scopes.len());
    for var in mir.vars_iter() {
        let decl = &mir.local_decls[var];
        has_variables.insert(decl.visibility_scope.index());
    }

    // Instantiate all scopes.
    for idx in 0..mir.source_scopes.len() {
        let scope = SourceScope::new(idx);
        make_mir_scope(cx, &mir, &has_variables, debug_context, scope, &mut scopes);
    }

    scopes
}

fn make_mir_scope(cx: &CodegenCx,
                  mir: &Mir,
                  has_variables: &BitVector,
                  debug_context: &FunctionDebugContextData,
                  scope: SourceScope,
                  scopes: &mut IndexVec<SourceScope, MirDebugScope>) {
    if scopes[scope].is_valid() {
        return;
    }

    let scope_data = &mir.source_scopes[scope];
    let parent_scope = if let Some(parent) = scope_data.parent_scope {
        make_mir_scope(cx, mir, has_variables, debug_context, parent, scopes);
        scopes[parent]
    } else {
        // The root is the function itself.
        let loc = span_start(cx, mir.span);
        scopes[scope] = MirDebugScope {
            scope_metadata: debug_context.fn_metadata,
            file_start_pos: loc.file.start_pos,
            file_end_pos: loc.file.end_pos,
        };
        return;
    };

    if !has_variables.contains(scope.index()) {
        // Do not create a DIScope if there are no variables
        // defined in this MIR Scope, to avoid debuginfo bloat.

        // However, we don't skip creating a nested scope if
        // our parent is the root, because we might want to
        // put arguments in the root and not have shadowing.
        if parent_scope.scope_metadata != debug_context.fn_metadata {
            scopes[scope] = parent_scope;
            return;
        }
    }

    let loc = span_start(cx, scope_data.span);
    let file_metadata = file_metadata(cx,
                                      &loc.file.name,
                                      debug_context.defining_crate);

    let scope_metadata = unsafe {
        llvm::LLVMRustDIBuilderCreateLexicalBlock(
            DIB(cx),
            parent_scope.scope_metadata,
            file_metadata,
            loc.line as c_uint,
            loc.col.to_usize() as c_uint)
    };
    scopes[scope] = MirDebugScope {
        scope_metadata,
        file_start_pos: loc.file.start_pos,
        file_end_pos: loc.file.end_pos,
    };
}
