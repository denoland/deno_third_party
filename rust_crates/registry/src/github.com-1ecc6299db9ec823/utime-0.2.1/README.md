utime
========
A missing utime function for Rust.

- [API Documentation](https://docs.rs/utime)
- [See `utime` in crates.io](https://crates.io/crates/utime)

Standard library of Rust doesn't provide stable way to set atime/mtime of a
file. This crate provides stable way to change a file's last modification and
access time.

```toml
[dependencies]
utime = "0.2"
```
```rust
use std::fs::File;
use utime::*;

File::create("target/testdummy").unwrap();
set_file_times("target/testdummy", 1000000, 1000000000).unwrap();

let (accessed, modified) = get_file_times("target/testdummy").unwrap();
assert_eq!(accessed, 1000000);
assert_eq!(modified, 1000000000);
```

### Build Status

Linux / macOS | Windows
:------------:|:--------:
[![Travis Build Status]][travis] | [![AppVeyor Build Status]][appveyor]

<br>

--------
*utime* is primarily distributed under the terms of both the [MIT
license] and the [Apache License (Version 2.0)]. See [COPYRIGHT] for details.

[Travis Build Status]: https://badgen.net/travis/simnalamburt/utime/master?icon=travis&label=build
[travis]: https://travis-ci.org/simnalamburt/utime
[AppVeyor Build Status]: https://badgen.net/appveyor/ci/simnalamburt/utime/master?icon=appveyor&label=build
[appveyor]: https://ci.appveyor.com/project/simnalamburt/utime/branch/master
[MIT license]: LICENSE-MIT
[Apache License (Version 2.0)]: LICENSE-APACHE
[COPYRIGHT]: COPYRIGHT
