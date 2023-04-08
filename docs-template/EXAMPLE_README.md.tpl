<!-- MD024 - The Headers from the Platform-Specific Examples should be identical  -->
<!-- Use 'cargo run -p build-templated-pages -- build-example-page' to generate the final example README.md -->
<!-- markdownlint-disable-file MD024 -->

# Examples

These examples demonstrate the main features of Bevy and how to use them.
To run an example, use the command `cargo run --example <Example>`, and add the option `--features x11` or `--features wayland` to force the example to run on a specific window compositor, e.g.

```sh
cargo run --features wayland --example hello_world
```

**⚠️ Note: for users of releases on crates.io!**

There are often large differences and incompatible API changes between the latest [crates.io](https://crates.io/crates/bevy) release and the development version of Bevy in the git main branch!

If you are using a released version of bevy, you need to make sure you are viewing the correct version of the examples!

- Latest release: [https://github.com/bevyengine/bevy/tree/latest/examples](https://github.com/bevyengine/bevy/tree/latest/examples)
- Specific version, such as `0.4`: [https://github.com/bevyengine/bevy/tree/v0.4.0/examples](https://github.com/bevyengine/bevy/tree/v0.4.0/examples)

When you clone the repo locally to run the examples, use `git checkout` to get the correct version:

```bash
# `latest` always points to the newest release
git checkout latest
# or use a specific version
git checkout v0.4.0
```

---

## Table of Contents

- [Examples](#examples)
  - [Table of Contents](#table-of-contents)
- [The Bare Minimum](#the-bare-minimum)
  - [Hello, World!](#hello-world)
- [Cross-Platform Examples](#cross-platform-examples)
{% for category, _ in all_examples %}  - [{{ category }}](#{{ category | slugify }})
{% endfor %}
- [Tests](#tests)
- [Platform-Specific Examples](#platform-specific-examples)
  - [Android](#android)
    - [Setup](#setup)
    - [Build & Run](#build--run)
    - [Old phones](#old-phones)
  - [iOS](#ios)
    - [Setup](#setup-1)
    - [Build & Run](#build--run-1)
  - [WASM](#wasm)
    - [Setup](#setup-2)
    - [Build & Run](#build--run-2)
    - [Loading Assets](#loading-assets)

# The Bare Minimum

<!-- MD026 - Hello, World! looks better with the ! -->
<!-- markdownlint-disable-next-line MD026 -->
## Hello, World!

Example | Description
--- | ---
[`hello_world.rs`](./hello_world.rs) | Runs a minimal example that outputs "hello world"

# Cross-Platform Examples
{% for category, details in all_examples %}
## {{ category }}

{% if details.description is string %}{{ details.description }}
{% endif %}Example | Description
--- | ---
{% for example in details.examples %}[{{ example.name }}](../{{ example.path }}) | {{ example.description }}
{% endfor %}{% endfor %}
# Tests

Example | Description
--- | ---
[How to Test Systems](../tests/how_to_test_systems.rs) | How to test systems with commands, queries or resources

# Platform-Specific Examples

## Android

### Setup

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi
cargo install cargo-apk
```

The Android SDK must be installed, and the environment variable `ANDROID_SDK_ROOT` set to the root Android `sdk` folder.

When using `NDK (Side by side)`, the environment variable `ANDROID_NDK_ROOT` must also be set to one of the NDKs in `sdk\ndk\[NDK number]`.

### Build & Run

To run on a device setup for Android development, run:

```sh
cargo apk run -p bevy_mobile_example
```

When using Bevy as a library, the following fields must be added to `Cargo.toml`:

```toml
[package.metadata.android]
build_targets = ["aarch64-linux-android", "armv7-linux-androideabi"]

[package.metadata.android.sdk]
target_sdk_version = 31
```

Please reference `cargo-apk` [README](https://crates.io/crates/cargo-apk) for other Android Manifest fields.

### Debugging

You can view the logs with the following command:

```sh
adb logcat | grep 'RustStdoutStderr\|bevy\|wgpu'
```

In case of an error getting a GPU or setting it up, you can try settings logs of `wgpu_hal` to `DEBUG` to get more information.

Sometimes, running the app complains about an unknown activity. This may be fixed by uninstalling the application:

```sh
adb uninstall org.bevyengine.example
```

### Old phones

Bevy by default targets Android API level 31 in its examples which is the <!-- markdown-link-check-disable -->
[Play Store's minimum API to upload or update apps](https://developer.android.com/distribute/best-practices/develop/target-sdk). <!-- markdown-link-check-enable -->
Users of older phones may want to use an older API when testing.

To use a different API, the following fields must be updated in Cargo.toml:

```toml
[package.metadata.android.sdk]
target_sdk_version = >>API<<
min_sdk_version = >>API or less<<
```

Example | File | Description
--- | --- | ---
`android` | [`mobile/src/lib.rs`](./mobile/src/lib.rs) | A 3d Scene with a button and playing sound

## iOS

### Setup

You need to install the correct rust targets:

- `aarch64-apple-ios`: iOS devices
- `x86_64-apple-ios`: iOS simulator on x86 processors
- `aarch64-apple-ios-sim`: iOS simulator on Apple processors

```sh
rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim
```

### Build & Run

Using bash:

```sh
cd examples/mobile
make run
```

In an ideal world, this will boot up, install and run the app for the first
iOS simulator in your `xcrun simctl devices list`. If this fails, you can
specify the simulator device UUID via:

```sh
DEVICE_ID=${YOUR_DEVICE_ID} make run
```

If you'd like to see xcode do stuff, you can run

```sh
open bevy_mobile_example.xcodeproj/
```

which will open xcode. You then must push the zoom zoom play button and wait
for the magic.

Example | File | Description
--- | --- | ---
`ios` | [`mobile/src/lib.rs`](./mobile/src/lib.rs) | A 3d Scene with a button and playing sound

## WASM

### Setup

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
```

### Build & Run

Following is an example for `lighting`. For other examples, change the `lighting` in the
following commands.

```sh
cargo build --release --example lighting --target wasm32-unknown-unknown --features webgl
wasm-bindgen --out-name wasm_example \
  --out-dir examples/wasm/target \
  --target web target/wasm32-unknown-unknown/release/examples/lighting.wasm
```

The first command will build the example for the wasm target, creating a binary. Then,
[wasm-bindgen-cli](https://rustwasm.github.io/wasm-bindgen/reference/cli.html) is used to create
javascript bindings to this wasm file, which can be loaded using this
[example HTML file](./wasm/index.html).

Then serve `examples/wasm` directory to browser. i.e.

```sh
# cargo install basic-http-server
basic-http-server examples/wasm

# with python
python3 -m http.server --directory examples/wasm

# with ruby
ruby -run -ehttpd examples/wasm
```

#### WebGL2 and WebGPU

Bevy support for WebGPU is being worked on, but is currently experimental.

If you don't enable the `webgl` feature, it will build for WebGPU by default, which may not work and has limited browser support.

### Optimizing

On the web, it's useful to reduce the size of the files that are distributed.
With rust, there are many ways to improve your executable sizes.
Here are some.

#### 1. Tweak your `Cargo.toml`

Add a new [profile](https://doc.rust-lang.org/cargo/reference/profiles.html)
to your `Cargo.toml`:

```toml
[profile.wasm-release]
# Use release profile as default values
inherits = "release"

# Optimize with size in mind, also try "s", sometimes it is better.
# This doesn't increase compilation times compared to -O3, great improvements
opt-level = "z"

# Do a second optimization pass removing duplicate or unused code from dependencies.
# Slows compile times, marginal improvements
lto = "fat"

# When building crates, optimize larger chunks at a time
# Slows compile times, marginal improvements
codegen-units = 1
```

Now, when building the final executable, use the `wasm-release` profile
by replacing `--release` by `--profile wasm-release` in the cargo command.

```sh
cargo build --profile wasm-release --example lighting --target wasm32-unknown-unknown
```

Make sure your final executable size is smaller, some of those optimizations
may not be worth keeping, due to compilation time increases.

#### 2. Use `wasm-opt` from the binaryen package

Binaryen is a set of tools for working with wasm. It has a `wasm-opt` CLI tool.

First download the `binaryen` package,
then locate the `.wasm` file generated by `wasm-bindgen`.
It should be in the `--out-dir` you specified in the command line,
the file name should end in `_bg.wasm`.

Then run `wasm-opt` with the `-Oz` flag. Note that `wasm-opt` is _very slow_.

Note that `wasm-opt` optimizations might not be as effective if you
didn't apply the optimizations from the previous section.

```sh
wasm-opt -Oz --output optimized.wasm examples/wasm/target/lighting_bg.wasm
mv optimized.wasm examples/wasm/target/lighting_bg.wasm
```

For a small project with a basic 3d model and two lights,
the generated file sizes are, as of Jully 2022 as following:

|profile                           | wasm-opt | no wasm-opt |
|----------------------------------|----------|-------------|
|Default                           | 8.5M     | 13.0M       |
|opt-level = "z"                   | 6.1M     | 12.7M       |
|"z" + lto = "thin"                | 5.9M     | 12M         |
|"z" + lto = "fat"                 | 5.1M     | 9.4M        |
|"z" + "thin" + codegen-units = 1  | 5.3M     | 11M         |
|"z" + "fat"  + codegen-units = 1  | 4.8M     | 8.5M        |

There are more advanced optimization options available,
check the following pages for more info:

- <https://rustwasm.github.io/book/reference/code-size.html>
- <https://rustwasm.github.io/docs/wasm-bindgen/reference/optimize-size.html>
- <https://rustwasm.github.io/book/game-of-life/code-size.html>

### Loading Assets

To load assets, they need to be available in the folder examples/wasm/assets. Cloning this
repository will set it up as a symlink on Linux and macOS, but you will need to manually move
the assets on Windows.
