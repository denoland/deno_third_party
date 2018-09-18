// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use io;
use sys::c;
use libc::c_char;
use path::PathBuf;
use fs::{OpenOptions, File};
use sys::ext::fs::OpenOptionsExt;
use sys::handle::Handle;
use super::super::{fill_utf16_buf, os2path, to_u16s, wide_char_to_multi_byte};

fn query_full_process_image_name() -> io::Result<PathBuf> {
    unsafe {
        let process_handle = Handle::new(c::OpenProcess(c::PROCESS_QUERY_INFORMATION,
                                                        c::FALSE,
                                                        c::GetCurrentProcessId()));
        fill_utf16_buf(|buf, mut sz| {
            if c::QueryFullProcessImageNameW(process_handle.raw(), 0, buf, &mut sz) == 0 {
                0
            } else {
                sz
            }
        }, os2path)
    }
}

fn lock_and_get_executable_filename() -> io::Result<(PathBuf, File)> {
    // We query the current image name, open the file without FILE_SHARE_DELETE so it
    // can't be moved and then get the current image name again. If the names are the
    // same than we have successfully locked the file
    let image_name1 = query_full_process_image_name()?;
    let file = OpenOptions::new()
                .read(true)
                .share_mode(c::FILE_SHARE_READ | c::FILE_SHARE_WRITE)
                .open(&image_name1)?;
    let image_name2 = query_full_process_image_name()?;

    if image_name1 != image_name2 {
        return Err(io::Error::new(io::ErrorKind::Other,
                                  "executable moved while trying to lock it"));
    }

    Ok((image_name1, file))
}

// Get the executable filename for libbacktrace
// This returns the path in the ANSI code page and a File which should remain open
// for as long as the path should remain valid
pub fn get_executable_filename() -> io::Result<(Vec<c_char>, File)> {
    let (executable, file) = lock_and_get_executable_filename()?;
    let u16_executable = to_u16s(executable.into_os_string())?;
    Ok((wide_char_to_multi_byte(c::CP_ACP, c::WC_NO_BEST_FIT_CHARS,
                                &u16_executable, true)?, file))
}
