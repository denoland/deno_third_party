# tokio-dns
Asynchronous name resolution utilities for the `futures` and `tokio-core` crates. Look at the crate-level documentation for more details.

[![BuildStatus](https://api.travis-ci.org/sbstp/tokio-dns.svg?branch=master)](https://travis-ci.org/sbstp/tokio-dns)

[Documentation](https://docs.rs/tokio-dns-unofficial)

This library [has been packaged to crates.io](https://crates.io/crates/tokio-dns-unofficial). Note that its name on crates.io is `tokio-dns-unofficial`, but the crate's name is `tokio_dns` (when using `extern crate ...`).

## Changelog

### 0.4.0
* Added a ton of combinations of `IpAdrr`, `SocketAddr`, and `port` to the `ToEndpoint` trait.
* Added new free functions to resolve a host/endpoint to a sequence of ip addresses or socket addresses, thanks @Fedcomp .
* Small docs changes and new examples.


### 0.3.1
* Fix a `rustc` regression, thanks @mehcode .

### 0.3.0
* Update to the new `tokio` crate.
* Change the API to look more like `tokio`'s API.
* New `resolve` free function to resolve a hostname asynchronously using the default resolver.


## Demo
```rust
// Taken from examples/basic.rs
use tokio_dns::TcpStream;

// connect using the built-in resolver.
let conn = TcpStream::connect("rust-lang.org:80").and_then(|sock| {
    println!("conncted to {}", sock.peer_addr().unwrap());
    Ok(())
});
```

## License
[MIT](LICENSE-MIT) or [Apache](LICENSE-APACHE)
