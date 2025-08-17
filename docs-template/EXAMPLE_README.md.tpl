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
    - [About `libc++_shared.so`](#about-libc_sharedso)
    - [Old phones](#old-phones)
    - [About `cargo-apk`](#about-cargo-apk)
  - [iOS](#ios)
    - [Setup](#setup-1)
    - [Build & Run](#build--run-1)
  - [Wasm](#wasm)
    - [Setup](#setup-2)
    - [Build & Run](#build--run-2)
    - [WebGL2 and WebGPU](#webgl2-and-webgpu)
    - [Audio in the browsers](#audio-in-the-browsers)
    - [Optimizing](#optimizing)
    - [Loading Assets](#loading-assets)

## The Bare Minimum

<!-- MD026 - Hello, World! looks better with the ! -->
<!-- markdownlint-disable-next-line MD026 -->
### Hello, World!

Example | Description
--- | ---
[`hello_world.rs`](./hello_world.rs) | Runs a minimal example that outputs "hello world"

## Cross-Platform Examples
{% for category, details in all_examples %}
### {{ category }}

{% if details.description is string %}{{ details.description }}
{% endif %}Example | Description
--- | ---
{% for example in details.examples %}[{{ example.name }}](../{{ example.path }}) | {{ example.description }}
{% endfor %}{% endfor %}
## Tests

Example | Description
--- | ---
[How to Test Apps](../tests/how_to_test_apps.rs) | How to test apps (simple integration testing)
[How to Test Systems](../tests/how_to_test_systems.rs) | How to test systems with commands, queries or resources

## Platform-Specific Examples

### Android

#### Setup

```sh
rustup target add aarch64-linux-android
cargo install cargo-ndk
```

The Android SDK must be installed, and the environment variable `ANDROID_SDK_ROOT` set to the root Android `sdk` folder.

When using `NDK (Side by side)`, the environment variable `ANDROID_NDK_ROOT` must also be set to one of the NDKs in `sdk\ndk\[NDK number]`.

Alternatively, you can install Android Studio.

#### Build & Run

To build an Android app, you first need to build shared object files for the target architecture with `cargo-ndk`:

```sh
cargo ndk -t <target_name> -o <project_name>/app/src/main/jniLibs build
```

For example, to compile to a 64-bit ARM platform:

```sh
cargo ndk -t arm64-v8a -o android_example/app/src/main/jniLibs build
```

Setting the output path ensures the shared object files can be found in target-specific directories under `jniLibs` where the JNI can find them.

See the `cargo-ndk` [README](https://crates.io/crates/cargo-ndk) for other options.

After this you can build it with `gradlew`:

```sh
./gradlew build
```

Or build it with Android Studio.

Then you can test it in your Android project.

##### About `libc++_shared.so`

Bevy may require `libc++_shared.so` to run on Android, as it is needed by the `oboe` crate, but typically `cargo-ndk` does not copy this file automatically.

To include it, you can manually obtain it from NDK source or use a `build.rs` script for automation, as described in the `cargo-ndk` [README](https://github.com/bbqsrc/cargo-ndk?tab=readme-ov-file#linking-against-and-copying-libc_sharedso-into-the-relevant-places-in-the-output-directory).

Alternatively, you can modify project files to include it when building an APK. To understand the specific steps taken in this project, please refer to the comments within the project files for detailed instructions(`app/CMakeList.txt`, `app/build.gradle`, `app/src/main/cpp/dummy.cpp`).

#### Debugging

You can view the logs with the following command:

```sh
adb logcat | grep 'RustStdoutStderr\|bevy\|wgpu'
```

In case of an error getting a GPU or setting it up, you can try settings logs of `wgpu_hal` to `DEBUG` to get more information.

Sometimes, running the app complains about an unknown activity. This may be fixed by uninstalling the application:

```sh
adb uninstall org.bevyengine.example
```

#### Old phones

In its examples, Bevy targets the minimum Android API that Play Store  <!-- markdown-link-check-disable -->
[requires](https://developer.android.com/distribute/best-practices/develop/target-sdk) to upload and update apps. <!-- markdown-link-check-enable -->
Users of older phones may want to use an older API when testing. By default, Bevy uses [`GameActivity`](https://developer.android.com/games/agdk/game-activity), which only works for Android API level 31 and higher, so if you want to use older API, you need to switch to `NativeActivity`.

To use `NativeActivity`, you need to edit it in `cargo.toml` manually like this:

```toml
bevy = { version = "0.14", default-features = false, features = ["android-native-activity", ...] }
```

Then build it as the [Build & Run](#build--run) section stated above.

##### About `cargo-apk`

You can also build an APK with `cargo-apk`, a simpler and deprecated tool which doesn't support `GameActivity`. If you want to use this, there is a [folder](./mobile/android_basic) inside the mobile example with instructions.

Example | File | Description
--- | --- | ---
`android` | [`mobile/src/lib.rs`](./mobile/src/lib.rs) | A 3d Scene with a button and playing sound

### iOS

#### Setup

You need to install the correct rust targets:

- `aarch64-apple-ios`: iOS devices
- `x86_64-apple-ios`: iOS simulator on x86 processors
- `aarch64-apple-ios-sim`: iOS simulator on Apple processors

```sh
rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim
```

#### Build & Run

Using bash:

```sh
cd examples/mobile
make run
```

In an ideal world, this will boot up, install and run the app for the first
iOS simulator in your `xcrun simctl list devices`. If this fails, you can
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

### Wasm

#### Setup

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
```

#### Build & Run

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
[example HTML file](./wasm/index.html).

Then serve `examples/wasm` directory to browser. i.e.

```sh
## cargo install basic-http-server
basic-http-server examples/wasm

## with python
python3 -m http.server --directory examples/wasm

## with ruby
ruby -run -ehttpd examples/wasm
```

##### WebGL2 and WebGPU

Bevy support for WebGPU is being worked on, but is currently experimental.

To build for WebGPU, you'll need to enable the `webgpu` feature. This will override the `webgl2` feature, and builds with the `webgpu` feature enabled won't be able to run on browsers that don't support WebGPU.

Bevy has a helper to build its examples:

- Build for WebGL2: `cargo run -p build-wasm-example -- --api webgl2 load_gltf`
- Build for WebGPU: `cargo run -p build-wasm-example -- --api webgpu load_gltf`
- Debug: `cargo run -p build-wasm-example -- --debug --api webgl2 load_gltf`

This helper will log the command used to build the examples.

#### Audio in the browsers

For the moment, everything is single threaded, this can lead to stuttering when playing audio in browsers. Not all browsers react the same way for all games, you will have to experiment for your game.

In browsers, audio is not authorized to start without being triggered by an user interaction. This is to avoid multiple tabs all starting to auto play some sounds. You can find more context and explanation for this on [Google Chrome blog](https://developer.chrome.com/blog/web-audio-autoplay/). This page also describes a JS workaround to resume audio as soon as the user interact with your game.

#### Optimizing

On the web, it's useful to reduce the size of the files that are distributed.
With rust, there are many ways to improve your executable sizes, starting with
the steps described in [the quick-start guide](https://bevy.org/learn/quick-start/getting-started/setup/#compile-with-performance-optimizations).

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

#### Loading Assets

To load assets, they need to be available in the folder examples/wasm/assets. Cloning this
repository will set it up as a symlink on Linux and macOS, but you will need to manually move
the assets on Windows.
