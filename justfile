export RUSTFLAGS := "-Dwarnings"

default_android_target := "aarch64-linux-android"

[doc("Runs clippy on all of Bevy's crates (TODO)")]
[group("ci")]
clippy:
    @just bevy_a11y
    @just bevy_app
    @just bevy_animation
    @just bevy_anti_alias
    @just bevy_ecs

[doc("Runs clippy on all of Bevy's crates for Android targets (TODO)")]
[group("ci")]
clippy_android target=default_android_target:
    @just bevy_android {{target}}

# These are crates that have features that require `DLSS_SDK`.
[doc("Runs clippy on all of Bevy's crates that use DLSS (TODO)")]
[group("ci")]
clippy_dlss:
    cargo clippy -p bevy_anti_alias --no-default-features --features="dlss"

# Runs clippy on `bevy_android`, with all this permutations of features
# * --no-default-features
# * "default features"
# * --all-features
#
# `bevy_android` only exists for `target_os="android"`. Which means that it
# requires either one of these targets:
# * aarch64-linux-android
# * List is not exhaustive
[doc("Runs clippy for `bevy_android`")]
[group("clippy")]
bevy_android target=default_android_target:
    cargo clippy -p bevy_android --no-default-features --target="{{target}}"
    cargo clippy -p bevy_android --target="{{target}}"
    cargo clippy -p bevy_android --all-features --target="{{target}}"

# Runs clippy on `bevy_anti_alias`, with all this permutations of features
# * --no-default-features
# * --no-default-features --features="*each feature in Cargo.toml individually*"
# * "default features"
# * --all-features
#
# Some of the features require that either `bevy_image/zstd_rust` or
# `bevy_image/zstd_c` be enabled.
#
# The `dlss` feature, without the `force_disable_dlss`, requires the environment
# variable `DLSS_SDK`. See `dlss_wgpu` README for more information.
[doc("Runs clippy for `bevy_anti_alias`")]
[group("clippy")]
bevy_anti_alias:
    cargo clippy -p bevy_anti_alias --no-default-features
    cargo clippy -p bevy_anti_alias --no-default-features --features="trace"
    cargo clippy -p bevy_anti_alias --no-default-features --features="smaa_luts bevy_image/zstd_rust"
    cargo clippy -p bevy_anti_alias --no-default-features --features="smaa_luts bevy_image/zstd_c"
    cargo clippy -p bevy_anti_alias --no-default-features --features="dlss force_disable_dlss"
    cargo clippy -p bevy_anti_alias
    cargo clippy -p bevy_anti_alias --all-features --features="bevy_image/zstd_rust"
    cargo clippy -p bevy_anti_alias --all-features --features="bevy_image/zstd_c"
