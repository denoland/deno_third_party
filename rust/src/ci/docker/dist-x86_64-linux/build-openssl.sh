#!/usr/bin/env bash

set -ex
source shared.sh

VERSION=1.0.2k
URL=https://s3-us-west-1.amazonaws.com/rust-lang-ci2/rust-ci-mirror/openssl-$VERSION.tar.gz

curl $URL | tar xzf -

cd openssl-$VERSION
hide_output ./config --prefix=/rustroot shared -fPIC
hide_output make -j10
hide_output make install
cd ..
rm -rf openssl-$VERSION

# Make the system cert collection available to the new install.
ln -nsf /etc/pki/tls/cert.pem /rustroot/ssl/
