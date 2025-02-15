# Rustdoc Postprocessor

We want to adjust rustdoc's html output to make it more obvious
which types are `Component`s, `Plugin`s etc. To do so, this
tool wraps rustdoc and modifies its output by adding relevant tags
to the top of a type's doc page.

On docs.rs and dev-docs.bevyengine.org the wrapper is invoked via

```bash
cargo doc --config "build.rustdoc = \"tools/rustdoc-wrapper/rustdoc.sh\""
```

If you want to build Bevy's documentation with these customizations
applied yourself and are not on Unix or want to run the wrapper in release mode,
first compile it:

```bash
cargo build --release --package rustdoc-wrapper
```

and then point `build.rustdoc` at it.

If you want to be able to run cargo doc without passing the right rustdoc path every time, you can set it in your `.cargo/config.toml`:

```toml
[build]
rustdoc = "target/release/rustdoc-wrapper"
```

## 3rd-Party Crates

The above also works with other crates that use Bevy.
