# Ice Core

Internet Core Engine (Ice) is a unified platform for building high-performance and scalable server-side applications and backend architectures.

Ice Core is the platform-independent and language-independent core library for Ice, written in Rust.

[![Build Status](https://travis-ci.org/losfair/IceCore.svg?branch=master)](https://travis-ci.org/losfair/IceCore)

[Documentation](https://docs.rs/ice_core/)

# Install

Prebuilt binaries for Linux can be found in [Releases](https://github.com/losfair/IceCore/releases/latest).

To build from source, make sure you have the latest Rust **nightly** toolchain installed and activated, and then `cargo build --release`.

After you've got the `.dll`, `.so` or `.dylib` built or downloaded, put it in the OS's default path for shared libraries. This is typically `/usr/lib` for Linux and `$HOME/lib` for macOS.

# Performance

##### Requests per second, higher is better

![Benchmark result](https://i.imgur.com/yU7vGAR.png)

The Node.js server framework, [Ice-node](https://github.com/losfair/ice-node), built on Ice Core, is 170% faster than raw Node HTTP implementation.

The Python 3 bindings for Ice Core, named Ice-python, is 2-6x faster than other tested Python web frameworks, including Sanic, aiohttp, BaseHTTPServer and Flask.

# Core Integration

Since v0.4, Ice Core tries to provide a set of standardized and stable C APIs for easy integration into other languages and platforms.

[ice-api-v4](https://github.com/losfair/ice-api-v4)

# Development

- [Ice-node](https://github.com/losfair/ice-node): Ice for Node.js
- [SharpIce](https://github.com/losfair/SharpIce): Ice for .NET Core
