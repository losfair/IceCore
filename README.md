# Ice Core

[![Build Status](https://travis-ci.org/losfair/IceCore.svg?branch=master)](https://travis-ci.org/losfair/IceCore)

Ice Core is a high performance web server library written in Rust.

# Install

Prebuilt binaries for Linux and macOS can be found in [Releases](https://github.com/losfair/IceCore/releases/latest).

To build from source, make sure you have the Rust toolchain installed, and then `cargo build --release`.

To build with Cervus Engine enabled, LLVM 3.8 is required. Then, build with `cervus` feature enabled: `cargo build --release --features cervus`

After you've got `libice_core.so` or `libice_core.dylib` built or downloaded, put it in the OS's default path for shared libraries. This is typically `/usr/lib` for Linux and `$HOME/lib` for macOS.

# Performance

![Benchmark result](https://i.imgur.com/fo6xskA.png)

The Node.js web framework, [Ice-node](https://github.com/losfair/ice-node), built on Ice Core, is 125% faster than raw Node HTTP implementation.

The Python 3 bindings for Ice Core, named Ice-python ([pyice_base](https://github.com/losfair/pyice_base)), is at least 6x faster than other tested Python web frameworks, including Sanic, aiohttp, BaseHTTPServer and Flask.

For requests that hit the Cervus Engine before being dispatched to endpoints, the performance is even better, 40% faster than Go `net/http`.

# Core Integration 

Both Ice-node and pyice_base are based on [ice-cpp](https://github.com/losfair/ice-cpp), which wraps all core APIs into C++.

To begin with, it's suggested to read [test.cpp](https://github.com/losfair/ice-cpp/blob/master/test.cpp), which implements a simple server.

If you want to use the core APIs directly instead of the C++ wrapper, [imports.h](https://github.com/losfair/ice-cpp/blob/master/imports.h) contains exported C symbols and is available for use.

# Development

- [Ice-node](https://github.com/losfair/ice-node) is ready as a full-featured Web microframework.
- [pyice_base](https://github.com/losfair/pyice_base) needs further development for user-friendly abstractions.

It's easy to write bindings and frameworks for other languages, following the Core Integration section. Contributions are always welcome!
