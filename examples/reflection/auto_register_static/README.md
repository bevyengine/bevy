# Automatic registration example for platforms without inventory support

This example illustrates how to use automatic type registration of `bevy_reflect` on platforms that don't support `inventory`.

To run the example, use the provided `Makefile` with `make run` or run manually by setting env var and enabling the required feature:

```sh
BEVY_REFLECT_AUTO_REGISTER_STATIC="$(cargo metadata --no-deps --format-version 1 | sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p')" cargo run --features bevy/reflect_auto_register_static
```

This approach should generally work on all platforms, however it is less convenient and slows down linking. It's recommended to use it only as a fallback.

Here's a list of caveats of this approach:

1. `load_type_registrations!` macro must be called before constructing `App` or using `TypeRegistry::register_derived_types`.
2. All of the types to be automatically registered must be declared in a separate from `load_type_registrations!` crate. This is why this example uses separate `lib` and `bin` setup.
3. Registration function names are cached in `target/type_registrations`. Due to incremental compilation the only way to rebuild this cache is to build with `bevy/reflect_auto_register_static` (or `auto_register_static` if just using `bevy_reflect`) feature disabled, then delete `target/type_registrations` and rebuild again with this feature enabled and `BEVY_REFLECT_AUTO_REGISTER_STATIC` environment variable set. Running `cargo clean` before recompiling is also an option, but it is even slower to do.
4. `BEVY_REFLECT_AUTO_REGISTER_STATIC` must be set to workspace `target` dir and be an absolute path for this to work.

If you're experiencing linking issues try running `cargo clean` before rebuilding.
