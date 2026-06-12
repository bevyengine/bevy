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

If you are using a released version of Bevy, you need to make sure you are viewing the correct version of the examples!

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
    - [Debugging](#debugging)
    - [Old phones](#old-phones)
      - [`GameActivity` vs `NativeActivity`](#gameactivity-vs-nativeactivity)
      - [Migrating from `GameActivity` to `NativeActivity`](#migrating-from-gameactivity-to-nativeactivity)
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

Example | File | Description
--- | --- | ---
`mobile` | [`mobile/src/lib.rs`](./mobile/src/lib.rs) | A 3d Scene with a button and playing sound

#### Setup

```sh
rustup target add aarch64-linux-android
cargo install cargo-ndk
```

The Android SDK must be installed, and the environment variable `ANDROID_SDK_ROOT` set to the root Android `sdk` folder.

When using `NDK (Side by side)`, the environment variable `ANDROID_NDK_ROOT` must also be set to one of the NDKs in `sdk\ndk\[NDK number]`.

Alternatively, you can install Android Studio.

#### Build & Run

**⚠️ Note:** In order to run this example on `x86_64`, you may need to use the `--release` flag.

**⚠️ Note:** The `-P 26` flag is currently required for building the example. It sets the correct API level required by `bevy_audio`.

1. Build shared object files for the target architecture with `cargo-ndk`:

    ```sh
    cargo ndk build -t <target_name> -P 26 -o <project_path>/app/src/main/jniLibs
    ```

    *Setting the output path ensures the shared object files can be found in target-specific directories under `jniLibs` where the JNI can find them. See the `cargo-ndk` [README](https://crates.io/crates/cargo-ndk) for additional options.*

    **Additional Info:**

    <details>
    <summary>Example for arm64-v8a target_name</summary>

    Build for `arm64-v8a`/`aarch64-linux-android` via:

    ```sh
    cargo ndk build -t arm64-v8a -P 26 -o ./android/app/src/main/jniLibs
    ```

    </details>

    <details>
    <summary>Get target_name from adb</summary>

    Print the required `target_name` for a device connected via `adb` via:

    ```sh
    adb shell getprop ro.product.cpu.abi
    ```

    </details>

2. Run Gradle via `./gradlew` (macOS, Linux, or BSD) or `gradlew.bat` (Windows) to install the app:

    Install the app via:

    ```sh
    cd ./android
    ./gradlew installDebug
    ```

    *This step installs the app to a device connected via `adb`. Afterwards you can open the app on the device. You can also use Android Studio for this step.*

    **Additional Info:**

    <details>
    <summary>Additional Gradle tasks</summary>

    Only build the app via:

    ```sh
    ./gradlew build
    ```

    Print additional tasks via:

    ```sh
    ./gradlew tasks
    ```

    </details>

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

**⚠️ Note:** If you are using `bevy_audio` the minimum supported Android API version is 26 (Android 8/Oreo).

In its example, Bevy uses Android API 37 as `targetSdk` to be able to benefit from security and performance improvements. For backwards compatibility, the example specifies Android API 26 as `minSdk`. This approach is recommended in the [Android Developers documentation](https://developer.android.com/google/play/requirements/target-sdk#why-target).

If you want to support older APIs, you can set a lower `minSdk`. You should however make sure that dependencies in `android/gradle/libs.versions.toml` support your API. You might also have to migrate to `NativeActivity`.

##### [`GameActivity`](https://developer.android.com/games/agdk/game-activity) vs [`NativeActivity`](https://developer.android.com/reference/android/app/NativeActivity)

Bevy uses `GameActivity`, which only works for Android API 23 and higher.

Quoting from [Android Developers](https://developer.android.com/games/agdk/game-activity), the major differences are as follows:

> If you are already familiar with `NativeActivity`, the major differences between `GameActivity` and `NativeActivity` are as follows:
>
> - `GameActivity` renders into a [`SurfaceView`](https://developer.android.com/reference/android/view/SurfaceView), making it much easier for games to interact with other UI components.
> - For touch and key input events, `GameActivity` has a completely new implementation with the [`android_input_buffer`](https://developer.android.com/reference/games/game-activity/structandroid/input-buffer) interface, separate from the [`InputQueue`](https://developer.android.com/reference/android/view/InputQueue) that NativeActivity uses.
> - `GameActivity` is a derived class of `AppCompatActivity`, which lets you seamlessly use other Jetpack components. [`ActionBar`](https://developer.android.com/reference/android/app/ActionBar), [`Fragment`](https://developer.android.com/guide/fragments), and others are all available.
> - `GameActivity` adds text input functionality by integrating [`the GameTextInput library`](https://developer.android.com/games/agdk/add-support-for-text-input).
> - Apps derived from `GameActivity` are expected to build all three parts of C/C++ code into one library. On the other hand, `NativeActivity`'s JNI functions are a part of the framework (always loaded by OS). Hence, only the `native_app_glue` and application’s C/C++ code are expected to be built into one library.
> - `NativeActivity` is a part of Android framework and follows its release cycle (typically yearly). GameActivity is a part of the Jetpack library, which has a much more frequent release cycle (typically biweekly); new features and bug fixes can arrive much more quickly.
>
> **Note:** We strongly recommend using **GameActivity** for new games and other C/C++ intensive applications. If you have an existing **NativeActivity** application, we recommend migrating to **GameActivity**.

If you still want to use `NativeActivity`, please see the next section.

##### Migrating from `GameActivity` to `NativeActivity`

1. Replace `android-game-activity` feature with `android-native-activity` in `Cargo.toml`.
    <details>
    <summary>Required Changes (Example)</summary>

    ```diff
    --- a/examples/mobile/Cargo.toml
    +++ b/examples/mobile/Cargo.toml
    [dependencies]
    -bevy = { version = "0.19", features = ["android-game-activity"] }
    +bevy = { version = "0.19", features = ["android-native-activity"] }
    ```

    </details>
2. Remove unnecessary dependencies in `android/gradle/libs.versions.toml`.
    <details>
    <summary>Required Changes (Example)</summary>

    ```diff
    --- a/examples/mobile/android/gradle/libs.versions.toml
    +++ b/examples/mobile/android/gradle/libs.versions.toml
    [versions]
    agp = "9.2.1"
    appcompat = "1.7.1"
    core = "1.18.0"
    -gamesActivity = "4.4.2" # Note: This must be compatible with `android-activity` crate used by bevy.
    material = "1.13.0"
    coreKtx = "1.18.0"

    [libraries]
    appcompat = { group = "androidx.appcompat", name = "appcompat", version.ref = "appcompat" }
    core = { group = "androidx.core", name = "core", version.ref = "core" }
    -games-activity = { group = "androidx.games", name = "games-activity", version.ref = "gamesActivity" }
    material = { group = "com.google.android.material", name = "material", version.ref = "material" }
    core-ktx = { group = "androidx.core", name = "core-ktx", version.ref = "coreKtx" }
    ```

    </details>
3. Remove unnecessary dependencies in `android/app/build.gradle.kts`.
    <details>
    <summary>Required Changes (Example)</summary>

    ```diff
    --- a/examples/mobile/android/app/build.gradle.kts
    +++ b/examples/mobile/android/app/build.gradle.kts
    dependencies {
        implementation(libs.appcompat)
        implementation(libs.core)
        implementation(libs.material)
    -    implementation(libs.games.activity)
        implementation(libs.core.ktx)
    }
    ```

    </details>
4. Use `NativeActivity` in `MainActivity.kt`.
    <details>
    <summary>Required Changes (Example)</summary>

    ```diff
    --- a/examples/mobile/android/app/src/main/kotlin/org/bevyengine/example/MainActivity.kt
    +++ b/examples/mobile/android/app/src/main/kotlin/org/bevyengine/example/MainActivity.kt
    package org.bevyengine.example

    +import android.app.NativeActivity
    import android.os.Bundle
    import androidx.core.view.WindowCompat
    import androidx.core.view.WindowInsetsCompat
    import androidx.core.view.WindowInsetsControllerCompat
    -import com.google.androidgamesdk.GameActivity

    /**
    * Load rust library and handle android specifics to integrate with it.
    *
    *
    * The library is loaded at class initialization and provided by jniLibs.
    */
    -class MainActivity : GameActivity() {
    +class MainActivity : NativeActivity() {
    ```

    </details>

### iOS

Example | File | Description
--- | --- | ---
`mobile` | [`mobile/src/lib.rs`](./mobile/src/lib.rs) | A 3d Scene with a button and playing sound

#### Setup

You need to install the correct Rust targets:

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

In browsers, audio is not authorized to start without being triggered by an user interaction. This is to avoid multiple tabs all starting to auto play some sounds. You can find more context and explanation for this on [Google Chrome blog](https://developer.chrome.com/blog/web-audio-autoplay/). This page also describes a JS workaround to resume audio as soon as the user interacts with your game.

#### Optimizing

On the web, it's useful to reduce the size of the files that are distributed.
With Rust, there are many ways to improve your executable sizes, starting with
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
