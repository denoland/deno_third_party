use std::io;
use std::net::{IpAddr, SocketAddr};

use futures::stream::Stream;
use futures::{self, Future, IntoFuture};
use tokio::net;

use endpoint::{Endpoint, ToEndpoint};
use resolver::{CpuPoolResolver, Resolver};
use {boxed, IoFuture};

lazy_static! {
    static ref POOL: CpuPoolResolver = CpuPoolResolver::new(5);
}

/// Resolve a hostname to a sequence of ip addresses using the default resolver.
#[deprecated(since = "0.4.0", note = "used `resolve_ip_addr` instead")]
pub fn resolve(host: &str) -> IoFuture<Vec<IpAddr>> {
    resolve_ip_addr(host)
}

/// Resolve a hostname to a sequence of ip addresses using the default resolver.
///
/// # Example
/// ```
/// tokio_dns::resolve_ip_addr("rust-lang.org");
/// ```
pub fn resolve_ip_addr(host: &str) -> IoFuture<Vec<IpAddr>> {
    POOL.resolve(host)
}

/// Resolve a hostname to a sequence of ip addresses using a custom resolver.
///
/// # Example
/// ```
/// # use tokio_dns::CpuPoolResolver;
/// let resolver = CpuPoolResolver::new(10);
///
/// tokio_dns::resolve_ip_addr_with("rust-lang.org", resolver.clone());
/// ```
pub fn resolve_ip_addr_with<R>(host: &str, resolver: R) -> IoFuture<Vec<IpAddr>>
where
    R: Resolver,
{
    resolver.resolve(host)
}

/// Resolve an endpoint to a sequence of socket addresses using the default resolver.
///
/// # Example
/// ```
/// tokio_dns::resolve_sock_addr(("rust-lang.org", 80));
/// ```
pub fn resolve_sock_addr<'a, T>(endpoint: T) -> IoFuture<Vec<SocketAddr>>
where
    T: ToEndpoint<'a>,
{
    resolve_endpoint(endpoint, POOL.clone())
}

/// Resolve an endpoint to a sequence of socket addresses using a custom resolver.
///
/// # Example
/// ```
/// # use tokio_dns::CpuPoolResolver;
/// let resolver = CpuPoolResolver::new(10);
///
/// tokio_dns::resolve_sock_addr_with(("rust-lang.org", 80), resolver.clone());
/// ```
pub fn resolve_sock_addr_with<'a, T, R>(endpoint: T, resolver: R) -> IoFuture<Vec<SocketAddr>>
where
    T: ToEndpoint<'a>,
    R: Resolver,
{
    resolve_endpoint(endpoint, resolver)
}

/// Shim for tokio::net::TcpStream
pub struct TcpStream;

impl TcpStream {
    /// Connect to the endpoint using the default resolver.
    pub fn connect<'a, T>(ep: T) -> IoFuture<net::TcpStream>
    where
        T: ToEndpoint<'a>,
    {
        TcpStream::connect_with(ep, POOL.clone())
    }

    /// Connect to the endpoint using a custom resolver.
    pub fn connect_with<'a, T, R>(ep: T, resolver: R) -> IoFuture<net::TcpStream>
    where
        T: ToEndpoint<'a>,
        R: Resolver,
    {
        boxed(
            resolve_endpoint(ep, resolver).and_then(move |addrs| {
                try_until_ok(addrs, move |addr| net::TcpStream::connect(&addr))
            }),
        )
    }
}

/// Shim for tokio::net::TcpListener
pub struct TcpListener;

impl TcpListener {
    /// Bind to the endpoint using the default resolver.
    pub fn bind<'a, T>(ep: T) -> IoFuture<net::TcpListener>
    where
        T: ToEndpoint<'a>,
    {
        TcpListener::bind_with(ep, POOL.clone())
    }

    /// Bind to the endpoint using a custom resolver.
    pub fn bind_with<'a, T, R>(ep: T, resolver: R) -> IoFuture<net::TcpListener>
    where
        T: ToEndpoint<'a>,
        R: Resolver,
    {
        boxed(
            resolve_endpoint(ep, resolver).and_then(move |addrs| {
                try_until_ok(addrs, move |addr| net::TcpListener::bind(&addr))
            }),
        )
    }
}

/// Shim for tokio::net::UdpSocket
pub struct UdpSocket;

impl UdpSocket {
    /// Bind to the endpoint using the default resolver.
    pub fn bind<'a, T>(ep: T) -> IoFuture<net::UdpSocket>
    where
        T: ToEndpoint<'a>,
    {
        UdpSocket::bind_with(ep, POOL.clone())
    }

    /// Bind to the endpoint using a custom resolver.
    pub fn bind_with<'a, T, R>(ep: T, resolver: R) -> IoFuture<net::UdpSocket>
    where
        T: ToEndpoint<'a>,
        R: Resolver,
    {
        boxed(
            resolve_endpoint(ep, resolver).and_then(move |addrs| {
                try_until_ok(addrs, move |addr| net::UdpSocket::bind(&addr))
            }),
        )
    }
}

/// Resolves endpoint into a vector of socket addresses.
fn resolve_endpoint<'a, T, R>(ep: T, resolver: R) -> IoFuture<Vec<SocketAddr>>
where
    R: Resolver,
    T: ToEndpoint<'a>,
{
    let ep = match ep.to_endpoint() {
        Ok(ep) => ep,
        Err(e) => return boxed(futures::failed(e)),
    };
    match ep {
        Endpoint::Host(host, port) => boxed(resolver.resolve(host).map(move |addrs| {
            addrs
                .into_iter()
                .map(|addr| SocketAddr::new(addr, port))
                .collect()
        })),
        Endpoint::SocketAddr(addr) => boxed(futures::finished(vec![addr])),
    }
}

fn try_until_ok<F, R, I>(addrs: Vec<SocketAddr>, f: F) -> IoFuture<I>
where
    F: Fn(SocketAddr) -> R + Send + 'static,
    R: IntoFuture<Item = I, Error = io::Error> + 'static,
    R::Future: Send + 'static,
    <R::Future as Future>::Error: From<io::Error>,
    I: Send + 'static,
{
    let result = Err(io::Error::new(
        io::ErrorKind::Other,
        "could not resolve to any address",
    ));
    boxed(
        futures::stream::iter_ok(addrs.into_iter())
            .fold::<_, _, Box<Future<Item = _, Error = io::Error> + Send>>(
                result,
                move |prev, addr| {
                    match prev {
                        Ok(i) => {
                            // Keep first successful result.
                            boxed(futures::finished(Ok(i)))
                        }
                        Err(..) => {
                            // Ignore previous error and try next address.
                            let future = f(addr).into_future();
                            // Lift future error into item to avoid short-circuit exit from fold.
                            boxed(future.then(Ok))
                        }
                    }
                },
            )
            .and_then(|r| r),
    )
}
