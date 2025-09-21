export RUSTFLAGS := "-Dwarnings"

default_android_target := "aarch64-linux-android"

[doc("Runs clippy on all of Bevy's crates (TODO)")]
[group("ci")]
clippy:
    @just bevy_a11y
    @just bevy_app
    @just bevy_ecs

[doc("Runs clippy on all of Bevy's crates for Android targets (TODO)")]
[group("ci")]
clippy_android $TARGET=default_android_target:
    @just bevy_android {{TARGET}}

# Runs clippy on `bevy_a11y`, with all this permutations of features
# * --no-default-features
# * --no-default-features --features="*each feature in Cargo.toml individually*"
# * "default features"
# * --all-features
[doc("Runs clippy for `bevy_a11y`")]
[group("clippy")]
bevy_a11y:
    cargo clippy -p bevy_a11y --no-default-features
    cargo clippy -p bevy_a11y --no-default-features --features="bevy_reflect"
    cargo clippy -p bevy_a11y --no-default-features --features="serialize"
    cargo clippy -p bevy_a11y --no-default-features --features="std"
    cargo clippy -p bevy_a11y --no-default-features --features="critical-section"
    cargo clippy -p bevy_a11y
    cargo clippy -p bevy_a11y --all-features

# Runs clippy on `bevy_android`, with all this permutations of features
# * --no-default-features
# * "default features"
# * --all-features
#
# `bevy_android` only exists for `target_os="android"`.
[doc("Runs clippy for `bevy_android`")]
[group("clippy")]
bevy_android $TARGET=default_android_target:
    cargo clippy -p bevy_android --no-default-features --target="{{TARGET}}"
    cargo clippy -p bevy_android --target="{{TARGET}}"
    cargo clippy -p bevy_android --all-features --target="{{TARGET}}"

# Runs clippy on `bevy_app`, with all this permutations of features
# * --no-default-features
# * --no-default-features --features="*each feature in Cargo.toml individually*"
# * "default features"
# * --all-features
# 
# Some of the features require that either `bevy_reflect/auto_register_inventory` or
# `bevy_reflect/auto_register_static` be enabled.
[doc("Runs clippy for `bevy_app`")]
[group("clippy")]
bevy_app:
    cargo clippy -p bevy_app --no-default-features
    cargo clippy -p bevy_app --no-default-features --features="bevy_reflect"
    cargo clippy -p bevy_app --no-default-features --features="reflect_functions"
    cargo clippy -p bevy_app --no-default-features --features="reflect_auto_register bevy_reflect/auto_register_inventory"
    cargo clippy -p bevy_app --no-default-features --features="reflect_auto_register bevy_reflect/auto_register_static"
    cargo clippy -p bevy_app --no-default-features --features="trace"
    cargo clippy -p bevy_app --no-default-features --features="bevy_debug_stepping"
    cargo clippy -p bevy_app --no-default-features --features="error_panic_hook"
    cargo clippy -p bevy_app --no-default-features --features="std"
    cargo clippy -p bevy_app --no-default-features --features="critical-section"
    cargo clippy -p bevy_app --no-default-features --features="web"
    cargo clippy -p bevy_app --no-default-features --features="hotpatching"
    cargo clippy -p bevy_app
    cargo clippy -p bevy_app --all-features --features="bevy_reflect/auto_register_inventory"
    cargo clippy -p bevy_app --all-features --features="bevy_reflect/auto_register_static"

# Runs clippy on `bevy_ecs`, with all this permutations of features
# * --no-default-features
# * --no-default-features --features="*each feature in Cargo.toml individually*"
# * "default features"
# * --all-features
# 
# Some of the features require that either `bevy_reflect/auto_register_inventory` or
# `bevy_reflect/auto_register_static` be enabled.
[doc("Runs clippy for `bevy_ecs`")]
[group("clippy")]
bevy_ecs:
    cargo clippy -p bevy_ecs --no-default-features
    cargo clippy -p bevy_ecs --no-default-features --features="multi_threaded"
    cargo clippy -p bevy_ecs --no-default-features --features="serialize"
    cargo clippy -p bevy_ecs --no-default-features --features="bevy_reflect"
    cargo clippy -p bevy_ecs --no-default-features --features="reflect_functions"
    cargo clippy -p bevy_ecs --no-default-features --features="reflect_auto_register bevy_reflect/auto_register_inventory"
    cargo clippy -p bevy_ecs --no-default-features --features="reflect_auto_register bevy_reflect/auto_register_static"
    cargo clippy -p bevy_ecs --no-default-features --features="backtrace"
    cargo clippy -p bevy_ecs --no-default-features --features="trace"
    cargo clippy -p bevy_ecs --no-default-features --features="detailed_trace"
    cargo clippy -p bevy_ecs --no-default-features --features="track_location"
    cargo clippy -p bevy_ecs --no-default-features --features="async_executor"
    cargo clippy -p bevy_ecs --no-default-features --features="std"
    cargo clippy -p bevy_ecs --no-default-features --features="critical-section"
    cargo clippy -p bevy_ecs --no-default-features --features="hotpatching"
    cargo clippy -p bevy_ecs
    cargo clippy -p bevy_ecs --all-features --features="bevy_reflect/auto_register_inventory"
    cargo clippy -p bevy_ecs --all-features --features="bevy_reflect/auto_register_static"
