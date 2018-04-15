# Ice Core

[![Crates.io](https://img.shields.io/crates/v/ice_core.svg)](https://crates.io/crates/ice_core)
[![Build Status](https://api.travis-ci.org/losfair/IceCore.svg?branch=master)](https://travis-ci.org/losfair/IceCore)

Build efficient and reliable backend applications in WebAssembly.

# What is it?

Ice is a container for backend applications in WebAssembly.

[WebAssembly](http://webassembly.org/), which is mainly used to build client-side Web applications, can also be used to build server-side applications. With a managed execution environment and the underlying JIT ([wasm-core](https://github.com/losfair/wasm-core)) based on LLVM, Ice is able to achieve a higher level of security (and additional safety for C/C++ applications), provide platform-independent high-level abstractions, and bring a few special features like dynamic inter-machine application migration and more accurate service monitoring.

# Build

Latest nightly Rust and LLVM 6 are required.

```
cargo build --release
```

# Get started

First, create a root directory to place configurations & applications:

```
mkdir my_ice_root
cd my_ice_root
```

Then, create a config file `config.yaml` in the root directory, whose format is defined in `config.rs`/`Config`:

```yaml
applications:
  - name: hello_world
    path: ./apps/hello_world
```

Here we've specified an application named `hello_world` located at `./apps/hello_world`, and the application will be automatically initialized when `ice_core` is launched.

Now let's initialize the `hello_world` application:

```
mkdir apps
cd apps
cargo new --lib hello_world
cd hello_world
```

Add a `[lib]` section and the runtime library `ia` to the newly-created `Cargo.toml`:

```toml
[lib]
name = "hello_world"
crate-type = ["cdylib"]

[dependencies]
ia = "0.1"
```

And create another `config.yaml` in the `hello_world` directory, which is the application-level metadata definition (defined in `config.rs`/`AppMetadata`:

```yaml
package_name: com.example.hello_world
bin: target/wasm32-unknown-unknown/release/hello_world.wasm
```

Write some code to print "Hello, world!" in `src/lib.rs`:

```rust
#[macro_use]
extern crate ia;

app_init!({
    println!("Hello, world!");
    0
});
```

Build it:

```
cargo build --release --target wasm32-unknown-unknown
```

cd back to my_ice_root and launch `ice_core`:

```
ice_core config.yaml
```

and you should see your first `hello_world` application running!

# Comparison with native binaries

The WebAssembly VM has to do some necessary checks and translations to ensure things work correctly. Therefore, it is always a little slower than precompiled native binaries. However, the difference is quite small and normally doesn't become the performance bottleneck for real-world applications.

In addition, Ice Core is able to provide a few features that a native environment doesn't provide:

- Accurate permission control
- Run-time inter-machine application migration (in progress)
- Service monitoring and management (in progress)

# Roadmap

- [x] Get WebAssembly code running
- [x] Provide native interfaces with permission control
- [x] TCP networking
- [x] Blocking file I/O
- [ ] Asynchronous file I/O
- [ ] UDP networking
- [ ] Built-in high-level abstraction for HTTP services
- [ ] Profiling & statistics
- [ ] Manager API & management script
- [ ] Runtime application migration across machines
