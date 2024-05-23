# Basic Android Example Instruction

This folder instructs you how to build android apps with `cargo-apk`, a deprecated Android apk building tool.

## Setup

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
target_sdk_version = 33
```

Please reference `cargo-apk` [README](https://crates.io/crates/cargo-apk) for other Android Manifest fields.

For this example, you can replace the `Cargo.toml` with the one within this folder.

Please refer example [README](../../README.md#debugging) for debugging instructions.

### Old phones

Bevy by default targets Android API level 33 in its examples which is the <!-- markdown-link-check-disable -->
[Play Store's minimum API to upload or update apps](https://developer.android.com/distribute/best-practices/develop/target-sdk). <!-- markdown-link-check-enable -->
Users of older phones may want to use an older API when testing.

To use a different API, the following fields must be updated in Cargo.toml:

```toml
[package.metadata.android.sdk]
target_sdk_version = >>API<<
min_sdk_version = >>API or less<<
```

