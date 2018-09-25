use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::str::FromStr;

/// An Endpoint is a way of identifying the target of a connection.
///
/// It can be a socket address or a host name which needs to be resolved
/// into a list of IP addresses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Endpoint<'a> {
    /// Endpoint is a host.
    ///
    /// The `&str` is the name and the `u16` is the port.
    Host(&'a str, u16),
    /// Endpoint is a `SocketAddr`.
    SocketAddr(SocketAddr),
}

/// A trait for objects that can be converted into an Endpoint.
///
/// This trait is implemented for the following types:
///
/// * `SocketAddr`, `SocketAddrV4`, `SocketAddrV6` - a socket address.
/// * `(IpAddr, u16)`, `(Ipv4Addr, u16)`, `(Ipv6Addr, u16)` - an IP address and a port
/// * `(&str, u16)` - a `<target>` and a port.
/// * `&str` - a string formatted as `<target>:port`.
///
/// Where `<target>` is a host name or an IP address.
///
/// This trait is similar to the `ToSocketAddrs` trait, except
/// that it does not perform host name resolution.
pub trait ToEndpoint<'a> {
    /// Create an endpoint from this object.
    fn to_endpoint(self) -> io::Result<Endpoint<'a>>;
}

impl<'a> ToEndpoint<'a> for SocketAddr {
    fn to_endpoint(self) -> io::Result<Endpoint<'a>> {
        Ok(Endpoint::SocketAddr(self))
    }
}

impl<'a, 'b> ToEndpoint<'a> for &'b SocketAddr {
    fn to_endpoint(self) -> io::Result<Endpoint<'a>> {
        Ok(Endpoint::SocketAddr(*self))
    }
}

impl<'a> ToEndpoint<'a> for SocketAddrV4 {
    fn to_endpoint(self) -> io::Result<Endpoint<'a>> {
        Ok(Endpoint::SocketAddr(SocketAddr::V4(self)))
    }
}

impl<'a, 'b> ToEndpoint<'a> for &'b SocketAddrV4 {
    fn to_endpoint(self) -> io::Result<Endpoint<'a>> {
        Ok(Endpoint::SocketAddr(SocketAddr::V4(*self)))
    }
}

impl<'a> ToEndpoint<'a> for SocketAddrV6 {
    fn to_endpoint(self) -> io::Result<Endpoint<'a>> {
        Ok(Endpoint::SocketAddr(SocketAddr::V6(self)))
    }
}

impl<'a, 'b> ToEndpoint<'a> for &'b SocketAddrV6 {
    fn to_endpoint(self) -> io::Result<Endpoint<'a>> {
        Ok(Endpoint::SocketAddr(SocketAddr::V6(*self)))
    }
}

impl<'a> ToEndpoint<'a> for (IpAddr, u16) {
    fn to_endpoint(self) -> io::Result<Endpoint<'a>> {
        Ok(Endpoint::SocketAddr(SocketAddr::new(self.0, self.1)))
    }
}

impl<'a> ToEndpoint<'a> for (Ipv4Addr, u16) {
    fn to_endpoint(self) -> io::Result<Endpoint<'a>> {
        Ok(Endpoint::SocketAddr(SocketAddr::new(
            IpAddr::V4(self.0),
            self.1,
        )))
    }
}

impl<'a> ToEndpoint<'a> for (Ipv6Addr, u16) {
    fn to_endpoint(self) -> io::Result<Endpoint<'a>> {
        Ok(Endpoint::SocketAddr(SocketAddr::new(
            IpAddr::V6(self.0),
            self.1,
        )))
    }
}

impl<'a> ToEndpoint<'a> for (&'a str, u16) {
    fn to_endpoint(self) -> io::Result<Endpoint<'a>> {
        match IpAddr::from_str(self.0) {
            Ok(addr) => (addr, self.1).to_endpoint(),
            Err(_) => Ok(Endpoint::Host(self.0, self.1)),
        }
    }
}

impl<'a> ToEndpoint<'a> for &'a str {
    fn to_endpoint(self) -> io::Result<Endpoint<'a>> {
        // try to parse as a socket address first
        if let Ok(addr) = self.parse() {
            return Ok(Endpoint::SocketAddr(addr));
        }

        fn parse_port(port: &str) -> io::Result<u16> {
            u16::from_str(port).map_err(|_| io::Error::new(io::ErrorKind::Other, "invalid port"))
        }

        match self.rfind(":") {
            Some(idx) => {
                let host = &self[..idx];
                let port = try!(parse_port(&self[idx + 1..]));
                Ok(Endpoint::Host(host, port))
            }
            None => Err(io::Error::new(io::ErrorKind::Other, "invalid endpoint")),
        }
    }
}

#[test]
fn test_resolve_localhost() {
    use super::{CpuPoolResolver, Resolver};
    use futures::Future;

    let resolver = CpuPoolResolver::new(1);

    let fut = resolver.resolve("localhost").and_then(|addrs| {
        for addr in addrs {
            // TODO 1.12 addr.is_loopback()
            assert!(match addr {
                IpAddr::V4(a) => a.is_loopback(),
                IpAddr::V6(a) => a.is_loopback(),
            });
        }
        Ok(())
    });

    let _ = fut.wait();
}

#[test]
fn test_endpoint_str_port() {
    use std::net::Ipv4Addr;

    let ep = ("0.0.0.0", 1227).to_endpoint().unwrap();
    match ep {
        Endpoint::SocketAddr(addr) => {
            assert_eq!(addr.ip(), IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));
            assert_eq!(addr.port(), 1227);
        }
        _ => panic!(),
    }
}

#[test]
fn test_endpoint_str() {
    let ep = "localhost:1227".to_endpoint().unwrap();
    match ep {
        Endpoint::Host(host, port) => {
            assert_eq!(host, "localhost");
            assert_eq!(port, 1227);
        }
        _ => panic!(),
    }
}

#[test]
fn test_endpoint_str_ipv4() {
    use std::net::SocketAddrV4;

    let ep = "0.0.0.0:1227".to_endpoint().unwrap();
    match ep {
        Endpoint::SocketAddr(SocketAddr::V4(addr)) => {
            assert_eq!(addr, SocketAddrV4::from_str("0.0.0.0:1227").unwrap());
        }
        _ => panic!(),
    }
}

#[test]
fn test_endpoint_str_ipv6() {
    use std::net::SocketAddrV6;

    let ep = "[::]:1227".to_endpoint().unwrap();
    match ep {
        Endpoint::SocketAddr(SocketAddr::V6(addr)) => {
            assert_eq!(addr, SocketAddrV6::from_str("[::]:1227").unwrap());
        }
        _ => panic!(),
    }
}
