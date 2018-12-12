use std::env;
use std::mem;
use std::slice;
use winapi::um::winbase::{STD_OUTPUT_HANDLE, STD_ERROR_HANDLE};
use winapi::um::handleapi::{INVALID_HANDLE_VALUE};
use winapi::um::consoleapi::{GetConsoleMode, SetConsoleMode};
use winapi::um::processenv::{GetStdHandle};
use winapi::um::winnt::WCHAR;
use winapi::ctypes::c_void;
use winapi::um::winbase::GetFileInformationByHandleEx;
use winapi::um::fileapi::FILE_NAME_INFO;
use winapi::um::minwinbase::FileNameInfo;
use winapi::shared::minwindef::MAX_PATH;
use atty;

const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x4;


pub fn is_a_terminal() -> bool {
    atty::is(atty::Stream::Stdout)
}

#[cfg(feature="terminal_autoconfig")]
pub fn is_a_color_terminal() -> bool {
    if !is_a_terminal() {
        return false;
    }
    if msys_tty_on_stdout() {
        return msys_color_check();
    }
    enable_ansi_mode()
}

#[cfg(not(feature="terminal_autoconfig"))]
pub fn is_a_color_terminal() -> bool {
    if msys_tty_on_stdout() {
        return msys_color_check();
    }
    false
}

fn msys_color_check() -> bool {
    match env::var("TERM") {
        Ok(term) => term != "dumb",
        Err(_) => true
    }
}

fn enable_ansi_on(handle: u32) -> bool {
    unsafe {
        let handle = GetStdHandle(handle);
        if handle == INVALID_HANDLE_VALUE {
            return false;
        }

        let mut dw_mode = 0;
        if GetConsoleMode(handle, &mut dw_mode) == 0 {
            return false;
        }

        dw_mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING;
        if SetConsoleMode(handle, dw_mode) == 0 {
            return false;
        }

        true
    }
}

pub fn enable_ansi_mode() -> bool {
    enable_ansi_on(STD_OUTPUT_HANDLE) || enable_ansi_on(STD_ERROR_HANDLE)
}

fn msys_tty_on_stdout() -> bool {
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let size = mem::size_of::<FILE_NAME_INFO>();
        let mut name_info_bytes = vec![0u8; size + MAX_PATH * mem::size_of::<WCHAR>()];
        let res = GetFileInformationByHandleEx(
            handle as *mut _,
            FileNameInfo,
            &mut *name_info_bytes as *mut _ as *mut c_void,
            name_info_bytes.len() as u32,
        );
        if res == 0 {
            return false;
        }
        let name_info: &FILE_NAME_INFO = &*(name_info_bytes.as_ptr() as *const FILE_NAME_INFO);
        let s = slice::from_raw_parts(
            name_info.FileName.as_ptr(),
            name_info.FileNameLength as usize / 2,
        );
        let name = String::from_utf16_lossy(s);
        // This checks whether 'pty' exists in the file name, which indicates that
        // a pseudo-terminal is attached. To mitigate against false positives
        // (e.g., an actual file name that contains 'pty'), we also require that
        // either the strings 'msys-' or 'cygwin-' are in the file name as well.)
        let is_msys = name.contains("msys-") || name.contains("cygwin-");
        let is_pty = name.contains("-pty");
        is_msys && is_pty
    }
}
