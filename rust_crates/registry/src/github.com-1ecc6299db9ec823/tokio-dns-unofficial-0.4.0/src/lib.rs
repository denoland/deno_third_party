//! This crate offers tools for asynchronous name resolution, and extensions to
//! the `tokio_core` crate.
//!
//! First, `Endpoint` and `ToEndpoint` behave very much like `SocketAddr` and
//! `ToSocketAddrs` from the standard library. The main difference is that the
//! `ToEndpoint` trait does not perform any name resolution. If simply detect
//! whether the given endpoint is a socket address or a host name. Then, it
//! is up to a resolver to perform name resolution.
//!
//! The `Resolver` trait describes an abstract, asynchronous resolver. This crate
//! provides one (for now) implementation of a resolver, the `CpuPoolResolver`.
//! It uses a thread pool and the `ToSocketAddrs` trait to perform name resolution.
//!
//! The crate level functions `tcp_connect`, `tcp_listen` and `udp_bind` support
//! name resolution via a lazy static `CpuPoolResolver` using 5 threads. Their
//!`*_with` counterpart take a resolver as an argument.
//!
//! [Git Repository](https://github.com/sbstp/tokio-dns)
#![warn(missing_docs)]

extern crate futures;
extern crate futures_cpupool;
extern crate tokio;

#[macro_use]
extern crate lazy_static;

mod endpoint;
mod net;
mod resolver;

use std::io;

use futures::future::Future;

/// An alias for the futures produced by this library.
pub type IoFuture<T> = Box<Future<Item = T, Error = io::Error> + Send>;

fn boxed<F>(fut: F) -> Box<Future<Item = F::Item, Error = F::Error> + Send>
where
    F: Future + Send + 'static,
{
    Box::new(fut)
}

pub use endpoint::{Endpoint, ToEndpoint};
#[allow(deprecated)]
pub use net::{
    resolve, resolve_ip_addr, resolve_ip_addr_with, resolve_sock_addr, resolve_sock_addr_with,
    TcpListener, TcpStream, UdpSocket,
};
pub use resolver::{CpuPoolResolver, Resolver};
