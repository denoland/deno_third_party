// Copyright 2012-2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::ffi::OsStr;
use std::env;
use std::path::PathBuf;
use common::Config;

/// Conversion table from triple OS name to Rust SYSNAME
const OS_TABLE: &'static [(&'static str, &'static str)] = &[
    ("android", "android"),
    ("androideabi", "android"),
    ("bitrig", "bitrig"),
    ("cloudabi", "cloudabi"),
    ("darwin", "macos"),
    ("dragonfly", "dragonfly"),
    ("emscripten", "emscripten"),
    ("freebsd", "freebsd"),
    ("fuchsia", "fuchsia"),
    ("haiku", "haiku"),
    ("ios", "ios"),
    ("l4re", "l4re"),
    ("linux", "linux"),
    ("mingw32", "windows"),
    ("netbsd", "netbsd"),
    ("openbsd", "openbsd"),
    ("redox", "redox"),
    ("solaris", "solaris"),
    ("win32", "windows"),
    ("windows", "windows"),
];

const ARCH_TABLE: &'static [(&'static str, &'static str)] = &[
    ("aarch64", "aarch64"),
    ("amd64", "x86_64"),
    ("arm", "arm"),
    ("arm64", "aarch64"),
    ("armv4t", "arm"),
    ("armv5te", "arm"),
    ("armv7", "arm"),
    ("armv7s", "arm"),
    ("asmjs", "asmjs"),
    ("hexagon", "hexagon"),
    ("i386", "x86"),
    ("i586", "x86"),
    ("i686", "x86"),
    ("mips", "mips"),
    ("mips64", "mips64"),
    ("mips64el", "mips64"),
    ("mipsel", "mips"),
    ("msp430", "msp430"),
    ("powerpc", "powerpc"),
    ("powerpc64", "powerpc64"),
    ("powerpc64le", "powerpc64"),
    ("s390x", "s390x"),
    ("sparc", "sparc"),
    ("sparc64", "sparc64"),
    ("sparcv9", "sparc64"),
    ("thumbv6m", "thumb"),
    ("thumbv7em", "thumb"),
    ("thumbv7m", "thumb"),
    ("wasm32", "wasm32"),
    ("x86_64", "x86_64"),
    ("xcore", "xcore"),
];

pub fn matches_os(triple: &str, name: &str) -> bool {
    // For the wasm32 bare target we ignore anything also ignored on emscripten
    // and then we also recognize `wasm32-bare` as the os for the target
    if triple == "wasm32-unknown-unknown" {
        return name == "emscripten" || name == "wasm32-bare";
    }
    let triple: Vec<_> = triple.split('-').collect();
    for &(triple_os, os) in OS_TABLE {
        if triple.contains(&triple_os) {
            return os == name;
        }
    }
    panic!("Cannot determine OS from triple");
}
pub fn get_arch(triple: &str) -> &'static str {
    let triple: Vec<_> = triple.split('-').collect();
    for &(triple_arch, arch) in ARCH_TABLE {
        if triple.contains(&triple_arch) {
            return arch;
        }
    }
    panic!("Cannot determine Architecture from triple");
}

pub fn get_env(triple: &str) -> Option<&str> {
    triple.split('-').nth(3)
}

pub fn get_pointer_width(triple: &str) -> &'static str {
    if (triple.contains("64") && !triple.ends_with("gnux32")) || triple.starts_with("s390x") {
        "64bit"
    } else {
        "32bit"
    }
}

pub fn make_new_path(path: &str) -> String {
    assert!(cfg!(windows));
    // Windows just uses PATH as the library search path, so we have to
    // maintain the current value while adding our own
    match env::var(lib_path_env_var()) {
        Ok(curr) => format!("{}{}{}", path, path_div(), curr),
        Err(..) => path.to_owned(),
    }
}

pub fn lib_path_env_var() -> &'static str {
    "PATH"
}
fn path_div() -> &'static str {
    ";"
}

pub fn logv(config: &Config, s: String) {
    debug!("{}", s);
    if config.verbose {
        println!("{}", s);
    }
}

pub trait PathBufExt {
    /// Append an extension to the path, even if it already has one.
    fn with_extra_extension<S: AsRef<OsStr>>(&self, extension: S) -> PathBuf;
}

impl PathBufExt for PathBuf {
    fn with_extra_extension<S: AsRef<OsStr>>(&self, extension: S) -> PathBuf {
        if extension.as_ref().len() == 0 {
            self.clone()
        } else {
            let mut fname = self.file_name().unwrap().to_os_string();
            if !extension.as_ref().to_str().unwrap().starts_with(".") {
                fname.push(".");
            }
            fname.push(extension);
            self.with_file_name(fname)
        }
    }
}
