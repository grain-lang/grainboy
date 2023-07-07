# Grainboy

Make and play games with Grain 🌾

## Overview

This is a Rust app that uses `winit` for windowing, `wgpu` for cross-platform graphics rendering, and `wasmtime` for running wasm game code.

## Quick Start

**1. Build the game**

```sh
# will hot-reload the game if it's already running
rm *.wasm && grain compile --no-wasm-tail-call hello.gr
```

**2. Start the app**

```
cargo run
```

**3. Run the game**

Drag `hello.gr.wasm` onto the window and play! 🎮

## Development

- windowing: `src/lib.rs`
- graphics rendering: `src/gpu.rs`
- user input structs:`src/input.rs`
- wasm runtime: `src/wasm.rs`
- spritesheet: `src/spritesheet`.
- shader: `src/main.wgsl`.
- grainboy bindings: `grainboy.gr`
- demo game: `hello.gr`

To start the app, exec `cargo run`.

## Troubleshooting

If you see a message like
```
Failed to load dropped file: unknown import: `GRAIN$MODULE$runtime/gc::malloc` has not been defined
```
you may have dragged `grainboy.gr.wasm` onto the window instead of your entrypoint.