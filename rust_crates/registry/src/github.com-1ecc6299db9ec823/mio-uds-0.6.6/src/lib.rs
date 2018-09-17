//! MIO bindings for Unix Domain Sockets

#![cfg(unix)]
#![deny(missing_docs)]
#![doc(html_root_url = "https://docs.rs/mio-uds/0.6")]

extern crate iovec;
extern crate libc;
extern crate mio;

use std::io;

mod datagram;
mod listener;
mod socket;
mod stream;

pub use stream::UnixStream;
pub use listener::UnixListener;
pub use datagram::UnixDatagram;

#[cfg(not(all(target_arch = "aarch64",target_os = "android")))]
type Len = libc::socklen_t;
// Match Android weirdness for aarch64 found in libc.
#[cfg(all(target_arch = "aarch64",target_os = "android"))]
type Len = libc::c_int;

fn cvt(i: libc::c_int) -> io::Result<libc::c_int> {
    if i == -1 {
        Err(io::Error::last_os_error())
    } else {
        Ok(i)
    }
}
