extern crate futures;
extern crate tokio;
extern crate tokio_dns;

use futures::stream::Stream;
use futures::Future;
use tokio_dns::TcpListener;

fn main() {
    // connect using the built-in resolver.
    let server = TcpListener::bind("localhost:3000")
        .and_then(|listener| {
            println!("Ready to accept");
            listener.incoming().for_each(|sock| {
                println!("Accepted connection from {:?}", sock.peer_addr().unwrap());
                Ok(())
            })
        })
        .map_err(|err| println!("Error binding socket {:?}", err));

    tokio::run(server);
}
