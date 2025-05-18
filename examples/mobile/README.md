# Mobile Example

See the [main examples README](../) for general information about Bevy's examples.

---

This directory contains an example project structure when developing for Android and iOS.
These platforms require special SDKs, toolchains, build configuration, etc.

Here we show you what you need to get started.

---

## Android

### Setup

```sh
rustup target add aarch64-linux-android
cargo install cargo-ndk
```

The Android SDK must be installed, and the environment variable `ANDROID_SDK_ROOT` set to the root Android `sdk` folder.

When using `NDK (Side by side)`, the environment variable `ANDROID_NDK_ROOT` must also be set to one of the NDKs in `sdk\ndk\[NDK number]`.

Alternatively, you can install Android Studio.

### Build & Run

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

#### About `libc++_shared.so`

Bevy may require `libc++_shared.so` to run on Android, as it is needed by the `oboe` crate, but typically `cargo-ndk` does not copy this file automatically.

To include it, you can manually obtain it from NDK source or use a `build.rs` script for automation, as described in the `cargo-ndk` [README](https://github.com/bbqsrc/cargo-ndk?tab=readme-ov-file#linking-against-and-copying-libc_sharedso-into-the-relevant-places-in-the-output-directory).

Alternatively, you can modify project files to include it when building an APK. To understand the specific steps taken in this project, please refer to the comments within the project files for detailed instructions(`app/CMakeList.txt`, `app/build.gradle`, `app/src/main/cpp/dummy.cpp`).

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

In its examples, Bevy targets the minimum Android API that Play Store  <!-- markdown-link-check-disable -->
[requires](https://developer.android.com/distribute/best-practices/develop/target-sdk) to upload and update apps. <!-- markdown-link-check-enable -->
Users of older phones may want to use an older API when testing. By default, Bevy uses [`GameActivity`](https://developer.android.com/games/agdk/game-activity), which only works for Android API level 31 and higher, so if you want to use older API, you need to switch to `NativeActivity`.

To use `NativeActivity`, you need to edit it in `cargo.toml` manually like this:

```toml
bevy = { version = "0.14", default-features = false, features = ["android-native-activity", ...] }
```

Then build it as the [Build & Run](#build--run) section stated above.

#### About `cargo-apk`

You can also build an APK with `cargo-apk`, a simpler and deprecated tool which doesn't support `GameActivity`. If you want to use this, there is a [folder](./mobile/android_basic) inside the mobile example with instructions.

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
