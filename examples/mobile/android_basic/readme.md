# Basic Android Example Instruction

This folder instructs you how to build android apps with `cargo-apk`, a deprecated Android apk building tool.

## Setup

```sh
rustup target add aarch64-linux-android armv7-linux-androideabi
cargo install cargo-apk
```

Please refer example [README](../../README.md#setup) for NDK/SDK related instructions.

## Build & Run

When using `cargo-apk`, it must use `NativeActivity`, so you need to edit it in `Cargo.toml` manually like this:

```toml
bevy = { version = "0.14", default-features = false, features = ["android-native-activity", ...] }
```

Then the following fields must be added to `Cargo.toml`:

```toml
[package.metadata.android]
build_targets = ["aarch64-linux-android", "armv7-linux-androideabi"]

[package.metadata.android.sdk]
target_sdk_version = 33
```

Please refer `cargo-apk` [README](https://crates.io/crates/cargo-apk) for other Android Manifest fields.

For this example, you can replace the `Cargo.toml` with the one within this folder.

After setup, you can run it on a device for Android development:

```sh
cargo apk run -p bevy_mobile_example
```

Please refer example [README](../../README.md#debugging) for debugging instructions.

## Old phones

Bevy by default targets Android API level 33 in its examples which is the <!-- markdown-link-check-disable -->
[Play Store's minimum API to upload or update apps](https://developer.android.com/distribute/best-practices/develop/target-sdk). <!-- markdown-link-check-enable -->
Users of older phones may want to use an older API when testing.

To use a different API, the following fields must be updated in `Cargo.toml`:

```toml
[package.metadata.android.sdk]
target_sdk_version = >>API<<
min_sdk_version = >>API or less<<
```
