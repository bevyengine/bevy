<!-- MD024 - The Headers from the Platform-Specific Examples should be identical  -->
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
  - [2D Rendering](#2d-rendering)
  - [3D Rendering](#3d-rendering)
  - [Animation](#animation)
  - [Application](#application)

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

## 2D Rendering

Example | Description
--- | ---
[Move Sprite](../examples/2d/move_sprite.rs) | Changes the transform of a sprite
[2D Rotation](../examples/2d/rotation.rs) | Demonstrates rotating entities in 2D with quaternions
[Mesh 2D](../examples/2d/mesh2d.rs) | Renders a 2d mesh
[Manual Mesh 2D](../examples/2d/mesh2d_manual.rs) | Renders a custom mesh "manually" with "mid-level" renderer apis
[Mesh 2D With Vertex Colors](../examples/2d/mesh2d_vertex_color_texture.rs) | Renders a 2d mesh with vertex color attributes
[Shapes](../examples/2d/shapes.rs) | Renders a rectangle, circle, and hexagon
[Sprite](../examples/2d/sprite.rs) | Renders a sprite
[Sprite Flipping](../examples/2d/sprite_flipping.rs) | Renders a sprite flipped along an axis
[Sprite Sheet](../examples/2d/sprite_sheet.rs) | Renders an animated sprite
[Text 2D](../examples/2d/text2d.rs) | Generates text in 2D
[Texture Atlas](../examples/2d/texture_atlas.rs) | Generates a texture atlas (sprite sheet) from individual sprites

## 3D Rendering

Example | Description
--- | ---
[3D Scene](../examples/3d/3d_scene.rs) | Simple 3D scene with basic shapes and lighting
[3D Shapes](../examples/3d/shapes.rs) | A scene showcasing the built-in 3D shapes
[Lighting](../examples/3d/lighting.rs) | Illustrates various lighting options in a simple scene
[Load glTF](../examples/3d/load_gltf.rs) | Loads and renders a glTF file as a scene
[MSAA](../examples/3d/msaa.rs) | Configures MSAA (Multi-Sample Anti-Aliasing) for smoother edges
[Orthographic View](../examples/3d/orthographic.rs) | Shows how to create a 3D orthographic view (for isometric-look in games or CAD applications)
[Parenting](../examples/3d/parenting.rs) | Demonstrates parent->child relationships and relative transformations
[Physically Based Rendering](../examples/3d/pbr.rs) | Demonstrates use of Physically Based Rendering (PBR) properties
[Render to Texture](../examples/3d/render_to_texture.rs) | Shows how to render to a texture, useful for mirrors, UI, or exporting images
[Shadow Biases](../examples/3d/shadow_biases.rs) | Demonstrates how shadow biases affect shadows in a 3d scene
[Shadow Caster and Receiver](../examples/3d/shadow_caster_receiver.rs) | Demonstrates how to prevent meshes from casting/receiving shadows in a 3d scene
[Spherical Area Lights](../examples/3d/spherical_area_lights.rs) | Demonstrates how point light radius values affect light behavior
[Split Screen](../examples/3d/split_screen.rs) | Demonstrates how to render two cameras to the same window to accomplish "split screen"
[Texture](../examples/3d/texture.rs) | Shows configuration of texture materials
[Transparency in 3D](../examples/3d/transparency_3d.rs) | Demonstrates transparency in 3d
[Two Passes](../examples/3d/two_passes.rs) | Renders two 3d passes to the same window from different perspectives
[Update glTF Scene](../examples/3d/update_gltf_scene.rs) | Update a scene from a glTF file, either by spawning the scene as a child of another entity, or by accessing the entities of the scene
[Vertex Colors](../examples/3d/vertex_colors.rs) | Shows the use of vertex colors
[Texture](../examples/3d/wireframe.rs) | Showcases wireframe rendering

## Animation

Example | Description
--- | ---
[Animated Fox](../examples/animation/animated_fox.rs) | Plays an animation from a skinned glTF
[Animated Transform](../examples/animation/animated_transform.rs) | Create and play an animation defined by code that operates on the `Transform` component
[Custom Skinned Mesh](../examples/animation/custom_skinned_mesh.rs) | Skinned mesh example with mesh and joints data defined in code
[glTF Skinned Mesh](../examples/animation/gltf_skinned_mesh.rs) | Skinned mesh example with mesh and joints data loaded from a glTF file

## Application

Example | Description
--- | ---
[Custom Loop](../examples/app/custom_loop.rs) | Demonstrates how to create a custom runner (to update an app manually)
[Drag and Drop](../examples/app/drag_and_drop.rs) | An example that shows how to handle drag and drop in an app
[Empty](../examples/app/empty.rs) | An empty application (does nothing)
[Empty with Defaults](../examples/app/empty_defaults.rs) | An empty application with default plugins
[Headless](../examples/app/headless.rs) | An application that runs without default plugins
[Logs](../examples/app/logs.rs) | Illustrate how to use generate log output
[Plugin](../examples/app/plugin.rs) | Demonstrates the creation and registration of a custom plugin
[Plugin Group](../examples/app/plugin_group.rs) | Demonstrates the creation and registration of a custom plugin group
[Return after Run](../examples/app/return_after_run.rs) | Show how to return to main after the Bevy app has exited
[Thread Pool Resources](../examples/app/thread_pool_resources.rs) | Creates and customizes the internal thread pool
[Headless with Defaults](../examples/app/headless_defaults.rs) | An application that runs with default plugins, but without an actual renderer
[Without Winit](../examples/app/without_winit.rs) | Create an application without winit (runs single time, no event loop)

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
cargo apk run --example android
```

:warning: At this time Bevy does not work in Android Emulator.

When using Bevy as a library, the following fields must be added to `Cargo.toml`:

```toml
[package.metadata.android]
build_targets = ["aarch64-linux-android", "armv7-linux-androideabi"]
target_sdk_version = 29
min_sdk_version = 16
```

Please reference `cargo-apk` [README](https://crates.io/crates/cargo-apk) for other Android Manifest fields.

### Old phones

Bevy by default targets Android API level 29 in its examples which is the <!-- markdown-link-check-disable -->
[Play Store's minimum API to upload or update apps](https://developer.android.com/distribute/best-practices/develop/target-sdk). <!-- markdown-link-check-enable -->
Users of older phones may want to use an older API when testing.

To use a different API, the following fields must be updated in Cargo.toml:

```toml
[package.metadata.android]
target_sdk_version = >>API<<
min_sdk_version = >>API or less<<
```

Example | File | Description
--- | --- | ---
`android` | [`android/android.rs`](./android/android.rs) | The `3d/3d_scene.rs` example for Android

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
cd examples/ios
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
open bevy_ios_example.xcodeproj/
```

which will open xcode. You then must push the zoom zoom play button and wait
for the magic.

Example | File | Description
--- | --- | ---
`ios` | [`ios/src/lib.rs`](./ios/src/lib.rs) | The `3d/3d_scene.rs` example for iOS

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
cargo build --release --example lighting --target wasm32-unknown-unknown
wasm-bindgen --out-name wasm_example --out-dir examples/wasm/target --target web target/wasm32-unknown-unknown/release/examples/lighting.wasm
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

### Loading Assets

To load assets, they need to be available in the folder examples/wasm/assets. Cloning this
repository will set it up as a symlink on Linux and macOS, but you will need to manually move
the assets on Windows.
