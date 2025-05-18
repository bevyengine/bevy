# WASM Supporting Files

See the [main examples README](../) for general information about Bevy's examples.

---

This folder contains the minimal extra files you need to run Bevy in a web browser.

Most of our [rich collection of examples](../) can be run on the Web, but some
special configuration and build steps are needed.

---

## Setup

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
```

## Build & Run

Following is an example for `lighting`. For other examples, change the `lighting` in the
following commands.

```sh
cargo build --release --example lighting --target wasm32-unknown-unknown
wasm-bindgen --out-name wasm_example \
  --out-dir examples/wasm/target \
  --target web target/wasm32-unknown-unknown/release/examples/lighting.wasm
```

The first command will build the example for the wasm target, creating a binary. Then,
[wasm-bindgen-cli](https://rustwasm.github.io/wasm-bindgen/reference/cli.html) is used to create
javascript bindings to this wasm file in the output file `examples/wasm/target/wasm_example.js`, which can be loaded using this
[example HTML file](./index.html).

Then serve `examples/wasm` directory to browser. i.e.

```sh
# cargo install basic-http-server
basic-http-server examples/wasm

# with python
python3 -m http.server --directory examples/wasm

# with ruby
ruby -run -ehttpd examples/wasm
```

### WebGL2 and WebGPU

Bevy support for WebGPU is being worked on, but is currently experimental.

To build for WebGPU, you'll need to enable the `webgpu` feature. This will override the `webgl2` feature, and builds with the `webgpu` feature enabled won't be able to run on browsers that don't support WebGPU.

Bevy has a helper to build its examples:

- Build for WebGL2: `cargo run -p build-wasm-example -- --api webgl2 load_gltf`
- Build for WebGPU: `cargo run -p build-wasm-example -- --api webgpu load_gltf`

This helper will log the command used to build the examples.

## Audio in the browsers

For the moment, everything is single threaded, this can lead to stuttering when playing audio in browsers. Not all browsers react the same way for all games, you will have to experiment for your game.

In browsers, audio is not authorized to start without being triggered by an user interaction. This is to avoid multiple tabs all starting to auto play some sounds. You can find more context and explanation for this on [Google Chrome blog](https://developer.chrome.com/blog/web-audio-autoplay/). This page also describes a JS workaround to resume audio as soon as the user interact with your game.

## Optimizing

On the web, it's useful to reduce the size of the files that are distributed.
With rust, there are many ways to improve your executable sizes, starting with
the steps described in [the quick-start guide](https://bevyengine.org/learn/quick-start/getting-started/setup/#compile-with-performance-optimizations).

Now, when building the executable, use `--profile wasm-release` instead of `--release`:

```sh
cargo build --profile wasm-release --example lighting --target wasm32-unknown-unknown
```

To apply `wasm-opt`, first locate the `.wasm` file generated in the `--out-dir` of the
earlier `wasm-bindgen-cli` command (the filename should end with `_bg.wasm`), then run:

```sh
wasm-opt -Oz --output optimized.wasm examples/wasm/target/lighting_bg.wasm
mv optimized.wasm examples/wasm/target/lighting_bg.wasm
```

Make sure your final executable size is actually smaller. Some optimizations
may not be worth keeping due to compilation time increases.

For a small project with a basic 3d model and two lights,
the generated file sizes are, as of July 2022, as follows:

profile                           | wasm-opt | no wasm-opt
----------------------------------|----------|-------------
Default                           | 8.5M     | 13.0M
opt-level = "z"                   | 6.1M     | 12.7M
"z" + lto = "thin"                | 5.9M     | 12M
"z" + lto = "fat"                 | 5.1M     | 9.4M
"z" + "thin" + codegen-units = 1  | 5.3M     | 11M
"z" + "fat"  + codegen-units = 1  | 4.8M     | 8.5M

## Loading Assets

To load assets, they need to be available in the folder examples/wasm/assets. Cloning this
repository will set it up as a symlink on Linux and macOS, but you will need to manually move
the assets on Windows.
