#!/bin/sh
export NINJA=/Users/rld/src/deno/third_party/depot_tools/ninja
export GN=/Users/rld/src/deno/buildtools/mac/gn
cargo test -vv --all
