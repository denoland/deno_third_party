# Cargo GN integration

[![Build Status](<https://dev.azure.com/denoland/cargo_gn/_apis/build/status/denoland.cargo_gn%20(1)?branchName=master>)](https://dev.azure.com/denoland/cargo_gn/_build/latest?definitionId=6&branchName=master)

https://crates.io/crates/cargo_gn

This package allows Rust users to quickly hook into the GN build system.
It provides built-in gn and ninja tools that hook semi-automatically into
Cargo's `build.rs`.

Put the following in your `Cargo.toml`

```toml
[build-dependencies]
cargo_gn = "0.0.9"
```

Now you should be able to add a `.gn` file in the root of your project and
start using `BUILD.gn`. See the example directory for a complete example:
https://github.com/denoland/cargo_gn/tree/master/example

Use `cargo build -vv` in order to see ninja output.

Read more about gn here: https://gn.googlesource.com/gn

The GN/Ninja executables are assumed to be "gn" and "ninja" unless $GN and
$NINJA environmental variables are set.
