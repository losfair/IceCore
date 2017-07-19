# Ice Core

[![Build Status](https://travis-ci.org/losfair/IceCore.svg?branch=master)](https://travis-ci.org/losfair/IceCore)

Ice Core is a high performance web server library written in Rust and C++.

# Install

Prebuilt binaries for Linux and macOS can be found in [Releases](https://github.com/losfair/IceCore/releases/latest).

To build from source, make sure you have the Rust toolchain installed, and then `cargo build --release`.

After you've got `libice_core.so` or `libice_core.dylib` built or downloaded, put it in the OS's default path for shared libraries. This is typically `/usr/lib` for Linux and `$HOME/lib` for macOS.

# Documentation

TODO - There is no documentation for the core library now, but the web framework for Node based on Ice Core, [ice-node](https://github.com/losfair/ice-node), is a good example.
