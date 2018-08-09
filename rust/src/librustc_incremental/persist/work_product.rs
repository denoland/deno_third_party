// Copyright 2012-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! This module contains files for saving intermediate work-products.

use persist::fs::*;
use rustc::dep_graph::{WorkProduct, WorkProductId, WorkProductFileKind};
use rustc::session::Session;
use rustc::util::fs::link_or_copy;
use std::path::PathBuf;
use std::fs as std_fs;

pub fn copy_cgu_workproducts_to_incr_comp_cache_dir(
    sess: &Session,
    cgu_name: &str,
    files: &[(WorkProductFileKind, PathBuf)]
) -> Option<(WorkProductId, WorkProduct)> {
    debug!("copy_cgu_workproducts_to_incr_comp_cache_dir({:?},{:?})",
           cgu_name,
           files);
    if sess.opts.incremental.is_none() {
        return None
    }

    let saved_files: Option<Vec<_>> =
        files.iter()
             .map(|&(kind, ref path)| {
                 let extension = match kind {
                     WorkProductFileKind::Object => "o",
                     WorkProductFileKind::Bytecode => "bc",
                     WorkProductFileKind::BytecodeCompressed => "bc.z",
                 };
                 let file_name = format!("{}.{}", cgu_name, extension);
                 let path_in_incr_dir = in_incr_comp_dir_sess(sess, &file_name);
                 match link_or_copy(path, &path_in_incr_dir) {
                     Ok(_) => Some((kind, file_name)),
                     Err(err) => {
                         sess.warn(&format!("error copying object file `{}` \
                                             to incremental directory as `{}`: {}",
                                            path.display(),
                                            path_in_incr_dir.display(),
                                            err));
                         None
                     }
                 }
             })
             .collect();
    let saved_files = match saved_files {
        None => return None,
        Some(v) => v,
    };

    let work_product = WorkProduct {
        cgu_name: cgu_name.to_string(),
        saved_files,
    };

    let work_product_id = WorkProductId::from_cgu_name(cgu_name);
    Some((work_product_id, work_product))
}

pub fn delete_workproduct_files(sess: &Session, work_product: &WorkProduct) {
    for &(_, ref file_name) in &work_product.saved_files {
        let path = in_incr_comp_dir_sess(sess, file_name);
        match std_fs::remove_file(&path) {
            Ok(()) => { }
            Err(err) => {
                sess.warn(
                    &format!("file-system error deleting outdated file `{}`: {}",
                             path.display(), err));
            }
        }
    }
}
