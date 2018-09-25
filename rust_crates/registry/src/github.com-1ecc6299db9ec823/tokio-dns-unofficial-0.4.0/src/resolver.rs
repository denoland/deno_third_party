use std::net::{IpAddr, ToSocketAddrs};
use std::str;

use futures_cpupool::CpuPool;

use {boxed, IoFuture};

/// The Resolver trait represents an object capable of
/// resolving host names into IP addresses.
pub trait Resolver {
    /// Given a host name, this function returns a Future which
    /// will eventually produce a list of IP addresses.
    fn resolve(&self, host: &str) -> IoFuture<Vec<IpAddr>>;
}

/// A resolver based on a thread pool.
///
/// This resolver uses the `ToSocketAddrs` trait inside
/// a thread to provide non-blocking address resolving.
#[derive(Clone)]
pub struct CpuPoolResolver {
    pool: CpuPool,
}

impl CpuPoolResolver {
    /// Create a new CpuPoolResolver with the given number of threads.
    pub fn new(num_threads: usize) -> Self {
        CpuPoolResolver {
            pool: CpuPool::new(num_threads),
        }
    }
}

impl Resolver for CpuPoolResolver {
    fn resolve(&self, host: &str) -> IoFuture<Vec<IpAddr>> {
        let host = format!("{}:0", host);
        boxed(
            self.pool
                .spawn_fn(move || match host[..].to_socket_addrs() {
                    Ok(it) => Ok(it.map(|s| s.ip()).collect()),
                    Err(e) => Err(e),
                }),
        )
    }
}
