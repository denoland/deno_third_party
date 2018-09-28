use socket2::SockAddr;
use std::ffi::{CStr, CString};
use std::io;
use std::mem;
use std::net::SocketAddr;
use std::ptr;

#[cfg(unix)]
use libc::{getaddrinfo as c_getaddrinfo, freeaddrinfo as c_freeaddrinfo, addrinfo as c_addrinfo,
           AF_INET, AF_INET6, socklen_t};

#[cfg(windows)]
use winapi::shared::ws2def::{ADDRINFOA as c_addrinfo, AF_INET, AF_INET6};
#[cfg(windows)]
use winapi::um::ws2tcpip::{getaddrinfo as c_getaddrinfo, freeaddrinfo as c_freeaddrinfo, socklen_t};

use err::LookupError;

/// A struct used as the hints argument to getaddrinfo.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct AddrInfoHints {
  /// Type of this socket. 0 for none.
  ///
  /// Values are defined by the libc on your system.
  pub socktype: i32,
  /// Protcol for this socket. 0 for none.
  ///
  /// Values are defined by the libc on your system.
  pub protocol: i32,
  /// Address family for this socket. 0 for none.
  ///
  /// Values are defined by the libc on your system.
  pub address: i32,
  /// Optional bitmask arguments. Bitwise OR bitflags to change the
  /// behaviour of getaddrinfo. 0 for none.
  ///
  /// Values are defined by the libc on your system.
  pub flags: i32,
}

impl AddrInfoHints {
  unsafe fn as_addrinfo(&self) -> c_addrinfo {
    let mut addrinfo: c_addrinfo = mem::zeroed();
    addrinfo.ai_socktype = self.socktype;
    addrinfo.ai_protocol = self.protocol;
    addrinfo.ai_family = self.address;
    addrinfo.ai_flags = self.flags;
    addrinfo
  }
}

impl Default for AddrInfoHints {
  /// Generate a blank AddrInfoHints struct, so new values can easily
  /// be specified.
  fn default() -> Self {
    AddrInfoHints {
      socktype: 0,
      protocol: 0,
      address: 0,
      flags: 0,
    }
  }
}

/// Struct that stores socket information, as returned by getaddrinfo.
///
/// This maps to the same definition provided by libc backends.
#[derive(Clone, Debug, PartialEq)]
pub struct AddrInfo {
  /// Type of this socket.
  ///
  /// Values are defined by the libc on your system.
  pub socktype: i32,
  /// Protcol family for this socket.
  ///
  /// Values are defined by the libc on your system.
  pub protocol: i32,
  /// Address family for this socket (usually matches protocol family).
  ///
  /// Values are defined by the libc on your system.
  pub address: i32,
  /// Socket address for this socket, usually containing an actual
  /// IP Address and port.
  pub sockaddr: SocketAddr,
  /// If requested, this is the canonical name for this socket/host.
  pub canonname: Option<String>,
  /// Optional bitmask arguments, usually set to zero.
  pub flags: i32,
}

impl AddrInfo {
  /// Copy the informataion from the given addrinfo pointer, and
  /// create a new AddrInfo struct with that information.
  ///
  /// Used for interfacing with getaddrinfo.
  unsafe fn from_ptr(a: *mut c_addrinfo) -> io::Result<Self> {
    if a.is_null() {
      return Err(
        io::Error::new(io::ErrorKind::Other,
        "Supplied pointer is null."
      ))?;
    }

    let addrinfo = *a;
    let sockaddr = SockAddr::from_raw_parts(addrinfo.ai_addr, addrinfo.ai_addrlen as socklen_t);
    let sock = match sockaddr.family().into() {
      AF_INET => SocketAddr::V4(sockaddr.as_inet().expect("Failed to decode INET")),
      AF_INET6 => SocketAddr::V6(sockaddr.as_inet6().expect("Failed to decode INET_6")),
      err => return Err(io::Error::new(
        io::ErrorKind::Other,
        format!("Found unknown address family: {}", err)
      ))?,
    };
    Ok(AddrInfo {
      socktype: addrinfo.ai_socktype,
      protocol: addrinfo.ai_protocol,
      address: addrinfo.ai_family,
      sockaddr: sock,
      canonname: addrinfo.ai_canonname.as_ref().map(|s|
        CStr::from_ptr(s).to_str().unwrap().to_owned()
      ),
      flags: 0,
    })
  }
}

/// An iterator of `AddrInfo` structs, wrapping a linked-list
/// returned by getaddrinfo.
///
/// It's recommended to use `.collect<io::Result<..>>()` on this
/// to collapse possible errors.
pub struct AddrInfoIter {
  orig: *mut c_addrinfo,
  cur: *mut c_addrinfo,
}

impl Iterator for AddrInfoIter {
  type Item = io::Result<AddrInfo>;

  fn next(&mut self) -> Option<Self::Item> {
    unsafe {
      if self.cur.is_null() { return None; }
      let ret = AddrInfo::from_ptr(self.cur);
      self.cur = (*self.cur).ai_next as *mut c_addrinfo;
      Some(ret)
    }
  }
}

unsafe impl Sync for AddrInfoIter {}
unsafe impl Send for AddrInfoIter {}

impl Drop for AddrInfoIter {
    fn drop(&mut self) {
        unsafe { c_freeaddrinfo(self.orig) }
    }
}

/// Retrieve socket information for a host, service, or both. Acts as a thin
/// wrapper around the libc getaddrinfo.
///
/// The only portable way to support International Domain Names (UTF8 DNS
/// names) is to manually convert to puny code before calling this function -
/// which can be done using the `idna` crate. However some libc backends may
/// support this natively, or by using bitflags in the hints argument.
///
/// Resolving names from non-UTF8 locales is currently not supported (as the
/// interface uses &str). Raise an issue if this is a concern for you.
pub fn getaddrinfo(host: Option<&str>, service: Option<&str>, hints: Option<AddrInfoHints>)
    -> Result<AddrInfoIter, LookupError> {
  // We must have at least host or service.
  if host.is_none() && service.is_none() {
    return Err(io::Error::new(
      io::ErrorKind::Other,
      "Either host or service must be supplied"
    ))?;
  }

  // Allocate CStrings, and keep around to free.
  let host = match host {
    Some(host_str) => Some(CString::new(host_str)?),
    None => None
  };
  let c_host = host.as_ref().map_or(ptr::null(), |s| s.as_ptr());
  let service = match service {
    Some(service_str) => Some(CString::new(service_str)?),
    None => None
  };
  let c_service = service.as_ref().map_or(ptr::null(), |s| s.as_ptr());

  let c_hints = unsafe {
    match hints {
      Some(hints) => hints.as_addrinfo(),
      None => mem::zeroed(),
    }
  };

  let mut res = ptr::null_mut();

  // Prime windows.
  #[cfg(windows)]
  ::win::init_winsock();

  unsafe {
    LookupError::match_gai_error(
      c_getaddrinfo(c_host, c_service, &c_hints, &mut res)
    )?;
  }

  Ok(AddrInfoIter { orig: res, cur: res })
}
