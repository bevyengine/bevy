# Examples

These examples demonstrate the main features of Bevy and how to use them.
To run an example, use the command `cargo run --example <Example>`, and add the option `--features x11` or `--features wayland` to force the example to run on a specific window compositor, e.g.

```sh
cargo run --features wayland --example hello_world
```

## Hello, World!

Example | Main | Description
--- | --- | ---
`hello_world` | [`hello_world.rs`](./hello_world.rs) | Runs a minimal example that outputs "hello world"

## 2D Rendering

Example | Main | Description
--- | --- | ---
`sprite` | [`2d/sprite.rs`](./2d/sprite.rs) | Renders a sprite
`sprite_sheet` | [`2d/sprite_sheet.rs`](./2d/sprite_sheet.rs) | Renders an animated sprite
`texture_atlas` | [`2d/texture_atlas.rs`](./2d/texture_atlas.rs) | Generates a texture atlas (sprite sheet) from individual sprites

## 3D Rendering

Example | File | Description
--- | --- | ---
`load_gltf` | [`3d/load_gltf.rs`](./3d/load_gltf.rs) | Loads and renders a gltf file as a scene
`msaa` | [`3d/msaa.rs`](./3d/msaa.rs) | Configures MSAA (Multi-Sample Anti-Aliasing) for smoother edges
`parenting` | [`3d/parenting.rs`](./3d/parenting.rs) | Demonstrates parent->child relationships and relative transformations
`3d_scene` | [`3d/3d_scene.rs`](./3d/3d_scene.rs) | Simple 3D scene with basic shapes and lighting
`spawner` | [`3d/spawner.rs`](./3d/spawner.rs) | Renders a large number of cubes with changing position and material
`texture` | [`3d/texture.rs`](./3d/texture.rs) | Shows configuration of texture materials
`z_sort_debug` | [`3d/z_sort_debug.rs`](./3d/z_sort_debug.rs) | Visualizes camera Z-ordering

## Application

Example | File | Description
--- | --- | ---
`empty` | [`app/empty.rs`](./app/empty.rs) | An empty application (does nothing)
`empty_defaults` | [`app/empty_defaults.rs`](./app/empty_defaults.rs) | An empty application with default plugins
`headless` | [`app/headless.rs`](./app/headless.rs) | An application that runs without default plugins
`plugin` | [`app/plugin.rs`](./app/plugin.rs) | Demonstrates the creation and registration of a custom plugin
`thread_pool_resources` | [`app/thread_pool_resources.rs`](./app/thread_pool_resources.rs) | Creates and customizes the internal thread pool

## Assets

Example | File | Description
--- | --- | ---
`asset_loading` | [`asset/asset_loading.rs`](./asset/asset_loading.rs) | Demonstrates various methods to load assets
`hot_asset_reloading` | [`asset/hot_asset_reloading.rs`](./asset/hot_asset_reloading.rs) | Demonstrates automatic reloading of assets when modified on disk

## Audio

Example | File | Description
--- | --- | ---
`audio` | [`audio/audio.rs`](./audio/audio.rs) | Shows how to load and play an audio file

## Diagnostics

Example | File | Description
--- | --- | ---
`custom_diagnostic` | [`diagnostics/custom_diagnostic.rs`](./diagnostics/custom_diagnostic.rs) | Shows how to create a custom diagnostic
`print_diagnostics` | [`diagnostics/print_diagnostics.rs`](./diagnostics/print_diagnostics.rs) | Add a plugin that prints diagnostics to the console

## ECS (Entity Component System)

Example | File | Description
--- | --- | ---
`event` | [`ecs/event.rs`](./ecs/event.rs) | Illustrates event creation, activation, and reception
`ecs_guide` | [`ecs/ecs_guide.rs`](./ecs/ecs_guide.rs) | Full guide to Bevy's ECS
`parallel_query` | [`ecs/parallel_query.rs`](./ecs/parallel_query.rs) | Illustrates parallel queries with `ParallelIterator`
`startup_system` | [`ecs/startup_system.rs`](./ecs/startup_system.rs) | Demonstrates a startup system (one that runs once when the app starts up)

## Games

Example | File | Description
--- | --- | ---
`breakout` | [`game/breakout.rs`](./game/breakout.rs) | An implementation of the classic game "Breakout"

## Input

Example | File | Description
--- | --- | ---
`mouse_input` | [`input/mouse_input.rs`](./input/mouse_input.rs) | Demonstrates handling a mouse button press/release
`mouse_input_events` | [`input/mouse_input_events.rs`](./input/mouse_input_events.rs) | Prints out all mouse events (buttons, movement, etc.)
`keyboard_input` | [`input/keyboard_input.rs`](./input/keyboard_input.rs) | Demonstrates handling a key press/release
`keyboard_input_events` | [`input/keyboard_input_events.rs`](./input/keyboard_input_events.rs) | Prints out all keyboard events

## Scene

Example | File | Description
--- | --- | ---
`scene` | [`scene/scene.rs`](./scene/scene.rs) | Demonstrates loading from and saving scenes to files
`properties` | [`scene/properties.rs`](./scene/properties.rs) | Demonstrates Properties (similar to reflections in other languages) in Bevy

## Shaders

Example | File | Description
--- | --- | ---
`shader_custom_material` | [`shader/shader_custom_material.rs`](./shader/shader_custom_material.rs) | Illustrates creating a custom material and a shader that uses it
`shader_defs` | [`shader/shader_defs.rs`](./shader/shader_defs.rs) | Demonstrates creating a custom material that uses "shaders defs" (a tool to selectively toggle parts of a shader)

## UI (User Interface)

Example | File | Description
--- | --- | ---
`button` | [`ui/button.rs`](./ui/button.rs) | Illustrates creating and updating a button
`text` | [`ui/text.rs`](./ui/text.rs) | Illustrates creating and updating text
`font_atlas_debug` | [`ui/font_atlas_debug.rs`](./ui/font_atlas_debug.rs) | Illustrates how FontAtlases are populated (used to optimize text rendering internally)
`ui` | [`ui/ui.rs`](./ui/ui.rs) | Illustrates various features of Bevy UI

## Window

Example | File | Description
--- | --- | ---
`clear_color` | [`window/clear_color.rs`](./window/clear_color.rs) | Creates a solid color window
`multiple_windows` | [`window/multiple_windows.rs`](./window/multiple_windows.rs) | Creates two windows and cameras viewing the same mesh
`window_settings` | [`window/window_settings.rs`](./window/window_settings.rs) | Demonstrates customizing default window settings

## WASM

#### Setup

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
```

#### Build & Run

Following is an example for `headless_wasm`. For other examples in wasm/ directory,
change the `headless_wasm` in the following commands **and edit** `examples/wasm/index.html`
to point to the correct `.js` file.

```sh
cargo build --example headless_wasm --target wasm32-unknown-unknown --no-default-features
wasm-bindgen --out-dir examples/wasm/target --target web target/wasm32-unknown-unknown/debug/examples/headless_wasm.wasm
```

Then serve `examples/wasm` dir to browser. i.e.

```sh
basic-http-server examples/wasm
```

## iOS

#### Setup

```sh
rustup target add aarch64-apple-ios x86_64-apple-ios
cargo install cargo-lipo
```

#### Build & Run

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

The Xcode build GUI will by default build the rust library for both
`x86_64-apple-ios`, and `aarch64-apple-ios` which may take a while. If you'd
like speed this up, you update the `IOS_TARGETS` User-Defined environment
variable in the "`cargo_ios` target" to be either `x86_64-apple-ios` or
`aarch64-applo-ios` depending on your goal.

Note: if you update this variable in Xcode, it will also change the default
used for the `Makefile`.

## Android

#### Setup

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi
cargo install cargo-apk
```

The Android SDK must be installed, and the environment variable `ANDROID_SDK_ROOT` set to the root Android `sdk` folder.

When using `NDK (Side by side)`, the environment variable `ANDROID_NDK_ROOT` must also be set to one of the NDKs in `sdk\ndk\[NDK number]`.

#### Build & Run

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

#### Old phones

Bevy by default targets Android API level 29 in its examples which is the [Play Store's minimum API to upload or update apps](https://developer.android.com/distribute/best-practices/develop/target-sdk). Users of older phones may want to use an older API when testing.

To use a different API, the following fields must be updated in Cargo.toml:

```toml
[package.metadata.android]
target_sdk_version = >>API<<
min_sdk_version = >>API or less<<
```
