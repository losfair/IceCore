# Ice Core

Build high-performance backend applications in WebAssembly. (WIP)

# What's it?

Ice Core is an application container for backend applications in WebAssembly.

[WebAssembly](http://webassembly.org/), which is mainly used to build client-side Web applications, can also be used to build server-side applications. With a managed execution environment and the underlying JIT ([wasm-core](https://github.com/losfair/wasm-core)) based on LLVM, Ice Core is able to achieve a much higher level of security (and additional safety for C/C++ applications) and bring a few exciting features like dynamic inter-machine migration of applications, while still keeping performance comparable to native binaries.

# Build

Latest nightly Rust and LLVM 6 are required.

```
cargo build --release
```

# Comparison with other 

### Native

The WebAssembly VM has to do some necessary checks and translations to ensure things work correctly. Therefore, it is always slower than precompiled native binaries. However, the difference is quite small and can be ignored most of the time for real-world applications.

In addition, Ice Core is able to provide a few features that a native environment doesn't provide:

- Run-time inter-machine application migration
- Easy management of the detailed states of all running services
- Concrete permission control

### Node.js

With the underlying V8 engine, Node.js supports WebAssembly quite well. However, Node.js is single-threaded, and the JS layer (which all external API calls must go through) is really slow.
