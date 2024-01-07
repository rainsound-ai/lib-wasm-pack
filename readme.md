# lib-wasm-pack

A library that makes it trivial to invoke [wasm-pack](https://github.com/rustwasm/wasm-pack) from your Rust code. Useful for [build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html), among other things.

If you need a binary to run from the command line, follow the normal `wasm-pack` [install instructions](https://rustwasm.github.io/wasm-pack/installer/) instead of using this crate.

```rust
// build.rs

fn main() {
    // Tell Cargo to rerun this build script if anything
    // in the wasm-crate folder changes.
    println!("cargo:rerun-if-changed=wasm-crate/**/*");

    let args = vec![
        "build",
        "--out-dir",
        // If we just passed "target/built-wasm-crate",
        // the output would be in "./wasm-crate/target/built-wasm-crate".
        // So instead, we go up a directory.
        "../target/built-wasm-crate",
        // The input crate path is relative to the current directory.
        "wasm-crate",
    ];

    match lib_wasm_pack::run(&args) {
        Ok(output) => {
            println!("wasm-pack completed.");
            println!("stdout:\n{}", output.stdout());
            println!("stderr:\n{}", output.stderr());
        },
        Err(error) => {
            // If we got as far as executing the CLI, `error` will
            // contain the stdout and stderr from the process.
            //
            // If present, they're also included when converting the
            // error to a string.
            println!("wasm-pack failed.");
            println!("{}", error);
        },
    }
}
```

## Versioning

Versions of this crate follow the form `v0.12.1-0.1.0`, where `0.12.1` is the wasm-pack version and `-0.1.0` is the crate version, in case we need to publish additional crate versions without bumping the wasm-pack version.
