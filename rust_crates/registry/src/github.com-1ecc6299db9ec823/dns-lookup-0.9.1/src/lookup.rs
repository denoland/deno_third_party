use std::io;
use std::net::IpAddr;
use std::str;

#[cfg(unix)]
use libc::SOCK_STREAM;

#[cfg(windows)]
use winapi::shared::ws2def::SOCK_STREAM;

use addrinfo::{getaddrinfo, AddrInfoHints};
use nameinfo::getnameinfo;

/// Lookup the address for a given hostname via DNS.
///
/// Returns an iterator of IP Addresses, or an `io::Error` on failure.
pub fn lookup_host(host: &str) -> io::Result<Vec<IpAddr>> {
  let hints = AddrInfoHints {
    socktype: SOCK_STREAM,
    ..AddrInfoHints::default()
  };

  match getaddrinfo(Some(host), None, Some(hints)) {
    Ok(addrs) => {
      let addrs: io::Result<Vec<_>> = addrs.map(|r| r.map(|a| a.sockaddr.ip())).collect();
      addrs
    },
    #[cfg(unix)]
    Err(e) => {
        use libc;
        // The lookup failure could be caused by using a stale /etc/resolv.conf.
        // See https://github.com/rust-lang/rust/issues/41570.
        // We therefore force a reload of the nameserver information.
        unsafe {
          libc::res_init();
        }
        // Use ? to convert to io::Result>
        Err(e)?
    },
    // the cfg is needed here to avoid an "unreachable pattern" warning
    #[cfg(not(unix))]
    // Use ? to convert to io::Result.
    Err(e) => Err(e)?,
  }
}

/// Lookup the hostname of a given IP Address via DNS.
///
/// Returns the hostname as a String, or an `io::Error` on failure.
pub fn lookup_addr(addr: &IpAddr) -> io::Result<String> {
  let sock = (*addr, 0).into();
  match getnameinfo(&sock, 0) {
    Ok((name, _)) => Ok(name),
    #[cfg(unix)]
    Err(e) => {
      use libc;
      // The lookup failure could be caused by using a stale /etc/resolv.conf.
      // See https://github.com/rust-lang/rust/issues/41570.
      // We therefore force a reload of the nameserver information.
      unsafe {
        libc::res_init();
      }
      Err(e)?
    },
    // the cfg is needed here to avoid an "unreachable pattern" warning
    #[cfg(not(unix))]
    Err(e) => Err(e)?,
  }
}

#[test]
fn test_localhost() {
  let ips = lookup_host("localhost").unwrap();
  assert!(ips.contains(&IpAddr::V4("127.0.0.1".parse().unwrap())));
  assert!(!ips.contains(&IpAddr::V4("10.0.0.1".parse().unwrap())));
}

#[cfg(unix)]
#[test]
fn test_rev_localhost() {
  let name = lookup_addr(&IpAddr::V4("127.0.0.1".parse().unwrap()));
  assert_eq!(name.unwrap(), "localhost");
}

#[cfg(windows)]
#[test]
fn test_hostname() {
  // Get machine's hostname.
  let hostname = ::hostname::get_hostname().unwrap();

  // Do reverse lookup of 127.0.0.1.
  let rev_name = lookup_addr(&IpAddr::V4("127.0.0.1".parse().unwrap()));

  assert_eq!(rev_name.unwrap(), hostname);
}
