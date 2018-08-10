// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use path::Prefix;
use ffi::OsStr;
use mem;

fn os_str_as_u8_slice(s: &OsStr) -> &[u8] {
    unsafe { mem::transmute(s) }
}
unsafe fn u8_slice_as_os_str(s: &[u8]) -> &OsStr {
    mem::transmute(s)
}

#[inline]
pub fn is_sep_byte(b: u8) -> bool {
    b == b'/' || b == b'\\'
}

#[inline]
pub fn is_verbatim_sep(b: u8) -> bool {
    b == b'\\'
}

pub fn parse_prefix<'a>(path: &'a OsStr) -> Option<Prefix> {
    use path::Prefix::*;
    unsafe {
        // The unsafety here stems from converting between &OsStr and &[u8]
        // and back. This is safe to do because (1) we only look at ASCII
        // contents of the encoding and (2) new &OsStr values are produced
        // only from ASCII-bounded slices of existing &OsStr values.
        let mut path = os_str_as_u8_slice(path);

        if path.starts_with(br"\\") {
            // \\
            path = &path[2..];
            if path.starts_with(br"?\") {
                // \\?\
                path = &path[2..];
                if path.starts_with(br"UNC\") {
                    // \\?\UNC\server\share
                    path = &path[4..];
                    let (server, share) = match parse_two_comps(path, is_verbatim_sep) {
                        Some((server, share)) =>
                            (u8_slice_as_os_str(server), u8_slice_as_os_str(share)),
                        None => (u8_slice_as_os_str(path), u8_slice_as_os_str(&[])),
                    };
                    return Some(VerbatimUNC(server, share));
                } else {
                    // \\?\path
                    let idx = path.iter().position(|&b| b == b'\\');
                    if idx == Some(2) && path[1] == b':' {
                        let c = path[0];
                        if c.is_ascii() && (c as char).is_alphabetic() {
                            // \\?\C:\ path
                            return Some(VerbatimDisk(c.to_ascii_uppercase()));
                        }
                    }
                    let slice = &path[..idx.unwrap_or(path.len())];
                    return Some(Verbatim(u8_slice_as_os_str(slice)));
                }
            } else if path.starts_with(b".\\") {
                // \\.\path
                path = &path[2..];
                let pos = path.iter().position(|&b| b == b'\\');
                let slice = &path[..pos.unwrap_or(path.len())];
                return Some(DeviceNS(u8_slice_as_os_str(slice)));
            }
            match parse_two_comps(path, is_sep_byte) {
                Some((server, share)) if !server.is_empty() && !share.is_empty() => {
                    // \\server\share
                    return Some(UNC(u8_slice_as_os_str(server), u8_slice_as_os_str(share)));
                }
                _ => (),
            }
        } else if path.get(1) == Some(& b':') {
            // C:
            let c = path[0];
            if c.is_ascii() && (c as char).is_alphabetic() {
                return Some(Disk(c.to_ascii_uppercase()));
            }
        }
        return None;
    }

    fn parse_two_comps(mut path: &[u8], f: fn(u8) -> bool) -> Option<(&[u8], &[u8])> {
        let first = match path.iter().position(|x| f(*x)) {
            None => return None,
            Some(x) => &path[..x],
        };
        path = &path[(first.len() + 1)..];
        let idx = path.iter().position(|x| f(*x));
        let second = &path[..idx.unwrap_or(path.len())];
        Some((first, second))
    }
}

pub const MAIN_SEP_STR: &'static str = "\\";
pub const MAIN_SEP: char = '\\';
