# Installing macOS dependencies

When building on macOS, you'll need the Xcode command line tools. Those can be obtained by [installing Xcode from the App Store](https://apps.apple.com/app/xcode/id497799835) and launching it at least once, _or_ by running `xcode-select --install` from the terminal.

If you get an error about not having accepted the Xcode and Apple SDKs Agreement, run `sudo xcodebuild -license` in the terminal. It will show you the agreement, and you'll need to type in `agree` and hit enter to accept it.

## Xcode 15.x

Xcode 15.x introduces a few `clang` changes that can cause the build to fail due an identifier format not expected by older versions of `bindgen`. The error looks like this:

```
error: failed to run custom build command for `coreaudio-sys v0.2.10`

Caused by:
  process didn't exit successfully: `<path to your project>/target/debug/build/coreaudio-sys-7e0a7d7c97b2ec33/build-script-build` (exit status: 101)
  --- stdout
  cargo:rerun-if-env-changed=COREAUDIO_SDK_PATH
  cargo:rustc-link-lib=framework=AudioUnit
  cargo:rustc-link-lib=framework=CoreAudio
  cargo:rerun-if-env-changed=BINDGEN_EXTRA_CLANG_ARGS

  --- stderr
  thread 'main' panicked at '"enum_(unnamed_at_/Applications/Xcode_app/Contents/Developer/Platforms/MacOSX_platform/Developer/SDKs/MacOSX14_0_sdk/usr/include/MacTypes_h_382_1)" is not a valid Ident', <path to your user folder>/.cargo/registry/src/index.crates.io-6f17d22bba15001f/proc-macro2-1.0.56/src/fallback.rs:811:9
  note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

There's an upstream [issue](https://github.com/RustAudio/coreaudio-sys/issues/85) in `coreaudio-sys` to update `bindgen`. In the mean time you can work around this by installing clang from [Homebrew](https://brew.sh):

```sh
brew install llvm@15
```

Make sure to **follow the additional instructions** provided by Homebrew after the installation, as **you'll need to add the `llvm` binaries to your `PATH`.**

This issue seems to affect both macOS 14.x (Sonoma) as well as 13.6. (Ventura)
