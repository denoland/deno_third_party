use std::ffi::CStr;
use std::io;
use std::str;

#[cfg(unix)]
use libc::{gethostname as c_gethostname, c_char};

#[cfg(windows)]
use winapi::ctypes::c_char;
#[cfg(windows)]
use winapi::um::winsock2::gethostname as c_gethostname;

/// Fetch the local hostname.
pub fn get_hostname() -> Result<String, io::Error> {
  // Prime windows.
  #[cfg(windows)]
  ::win::init_winsock();

  let mut c_name = [0 as c_char; 256 as usize];
  let res = unsafe {
    c_gethostname(c_name.as_mut_ptr(), c_name.len() as _)
  };

  // If an error occured, check errno for error message.
  if res != 0 {
    return Err(io::Error::last_os_error());
  }

  let hostname = unsafe {
    CStr::from_ptr(c_name.as_ptr())
  };

  str::from_utf8(hostname.to_bytes())
    .map(|h| h.to_owned())
    .map_err(|_| io::Error::new(io::ErrorKind::Other, "Non-UTF8 hostname"))
}
