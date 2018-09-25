extern crate futures;
extern crate tokio;
extern crate tokio_dns;

use futures::Future;
use tokio_dns::TcpStream;

fn main() {
    // connect using the built-in resolver.
    let connector = TcpStream::connect("rust-lang.org:80")
        .map(|sock| println!("Connected to {}", sock.peer_addr().unwrap()))
        .map_err(|err| println!("Error connecting {:?}", err));

    tokio::run(connector);
}
