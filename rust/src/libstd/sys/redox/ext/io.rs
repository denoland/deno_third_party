// Copyright 2016 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Unix-specific extensions to general I/O primitives

#![stable(feature = "rust1", since = "1.0.0")]

use fs;
use net;
use sys;
use io;
use sys_common::{self, AsInner, FromInner, IntoInner};

/// Raw file descriptors.
#[stable(feature = "rust1", since = "1.0.0")]
pub type RawFd = usize;

/// A trait to extract the raw unix file descriptor from an underlying
/// object.
///
/// This is only available on unix platforms and must be imported in order
/// to call the method. Windows platforms have a corresponding `AsRawHandle`
/// and `AsRawSocket` set of traits.
#[stable(feature = "rust1", since = "1.0.0")]
pub trait AsRawFd {
    /// Extracts the raw file descriptor.
    ///
    /// This method does **not** pass ownership of the raw file descriptor
    /// to the caller. The descriptor is only guaranteed to be valid while
    /// the original object has not yet been destroyed.
    #[stable(feature = "rust1", since = "1.0.0")]
    fn as_raw_fd(&self) -> RawFd;
}

/// A trait to express the ability to construct an object from a raw file
/// descriptor.
#[stable(feature = "from_raw_os", since = "1.1.0")]
pub trait FromRawFd {
    /// Constructs a new instances of `Self` from the given raw file
    /// descriptor.
    ///
    /// This function **consumes ownership** of the specified file
    /// descriptor. The returned object will take responsibility for closing
    /// it when the object goes out of scope.
    ///
    /// This function is also unsafe as the primitives currently returned
    /// have the contract that they are the sole owner of the file
    /// descriptor they are wrapping. Usage of this function could
    /// accidentally allow violating this contract which can cause memory
    /// unsafety in code that relies on it being true.
    #[stable(feature = "from_raw_os", since = "1.1.0")]
    unsafe fn from_raw_fd(fd: RawFd) -> Self;
}

/// A trait to express the ability to consume an object and acquire ownership of
/// its raw file descriptor.
#[stable(feature = "into_raw_os", since = "1.4.0")]
pub trait IntoRawFd {
    /// Consumes this object, returning the raw underlying file descriptor.
    ///
    /// This function **transfers ownership** of the underlying file descriptor
    /// to the caller. Callers are then the unique owners of the file descriptor
    /// and must close the descriptor once it's no longer needed.
    #[stable(feature = "into_raw_os", since = "1.4.0")]
    fn into_raw_fd(self) -> RawFd;
}

#[stable(feature = "rust1", since = "1.0.0")]
impl AsRawFd for fs::File {
    fn as_raw_fd(&self) -> RawFd {
        self.as_inner().fd().raw()
    }
}
#[stable(feature = "from_raw_os", since = "1.1.0")]
impl FromRawFd for fs::File {
    unsafe fn from_raw_fd(fd: RawFd) -> fs::File {
        fs::File::from_inner(sys::fs::File::from_inner(fd))
    }
}
#[stable(feature = "into_raw_os", since = "1.4.0")]
impl IntoRawFd for fs::File {
    fn into_raw_fd(self) -> RawFd {
        self.into_inner().into_fd().into_raw()
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl AsRawFd for net::TcpStream {
    fn as_raw_fd(&self) -> RawFd {
        self.as_inner().as_inner().fd().raw()
    }
}
#[stable(feature = "rust1", since = "1.0.0")]
impl AsRawFd for net::TcpListener {
    fn as_raw_fd(&self) -> RawFd {
        self.as_inner().as_inner().fd().raw()
    }
}
#[stable(feature = "rust1", since = "1.0.0")]
impl AsRawFd for net::UdpSocket {
    fn as_raw_fd(&self) -> RawFd {
        self.as_inner().as_inner().fd().raw()
    }
}

#[stable(feature = "asraw_stdio", since = "1.21.0")]
impl AsRawFd for io::Stdin {
    fn as_raw_fd(&self) -> RawFd { 0 }
}

#[stable(feature = "asraw_stdio", since = "1.21.0")]
impl AsRawFd for io::Stdout {
    fn as_raw_fd(&self) -> RawFd { 1 }
}

#[stable(feature = "asraw_stdio", since = "1.21.0")]
impl AsRawFd for io::Stderr {
    fn as_raw_fd(&self) -> RawFd { 2 }
}

#[stable(feature = "from_raw_os", since = "1.1.0")]
impl FromRawFd for net::TcpStream {
    unsafe fn from_raw_fd(fd: RawFd) -> net::TcpStream {
        let file = sys::fs::File::from_inner(fd);
        net::TcpStream::from_inner(sys_common::net::TcpStream::from_inner(file))
    }
}
#[stable(feature = "from_raw_os", since = "1.1.0")]
impl FromRawFd for net::TcpListener {
    unsafe fn from_raw_fd(fd: RawFd) -> net::TcpListener {
        let file = sys::fs::File::from_inner(fd);
        net::TcpListener::from_inner(sys_common::net::TcpListener::from_inner(file))
    }
}
#[stable(feature = "from_raw_os", since = "1.1.0")]
impl FromRawFd for net::UdpSocket {
    unsafe fn from_raw_fd(fd: RawFd) -> net::UdpSocket {
        let file = sys::fs::File::from_inner(fd);
        net::UdpSocket::from_inner(sys_common::net::UdpSocket::from_inner(file))
    }
}

#[stable(feature = "into_raw_os", since = "1.4.0")]
impl IntoRawFd for net::TcpStream {
    fn into_raw_fd(self) -> RawFd {
        self.into_inner().into_inner().into_fd().into_raw()
    }
}
#[stable(feature = "into_raw_os", since = "1.4.0")]
impl IntoRawFd for net::TcpListener {
    fn into_raw_fd(self) -> RawFd {
        self.into_inner().into_inner().into_fd().into_raw()
    }
}
#[stable(feature = "into_raw_os", since = "1.4.0")]
impl IntoRawFd for net::UdpSocket {
    fn into_raw_fd(self) -> RawFd {
        self.into_inner().into_inner().into_fd().into_raw()
    }
}
