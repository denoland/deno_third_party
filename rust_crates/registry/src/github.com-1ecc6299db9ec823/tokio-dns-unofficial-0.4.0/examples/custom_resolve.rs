extern crate futures;
extern crate tokio;
extern crate tokio_dns;

use futures::Future;
use tokio_dns::CpuPoolResolver;

fn main() {
    // create a custom, 10 thread CpuPoolResolver
    let resolver = CpuPoolResolver::new(10);

    // resolver is moved into the function, cloning it allows it to be used again after this call
    let fut = tokio_dns::resolve_sock_addr_with("rust-lang.org:80", resolver.clone())
        .map_err(|err| println!("Error resolve address {:?}", err))
        .map(|addrs| println!("Socket addresses {:#?}", addrs));

    tokio::run(fut);
}
