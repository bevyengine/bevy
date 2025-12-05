# Docs.rs Extensions

This directory includes some templates and styling to extend and modify [rustdoc]'s output
for Bevy's documentation on [docs.rs]. Currently this consists of tags indicating core
`bevy_ecs` traits.

## 3rd Party Crates

To use in your own crate, first copy this folder into your project,
then add the following to your Cargo.toml:

```toml
[package.metadata.docs.rs]
rustc-args = ["--cfg", "docsrs_dep"]
rustdoc-args = [
    "--cfg", "docsrs_dep",
    "--html-after-content", "docs-rs/trait-tags.html",
]

[lints.rust]
unexpected_cfgs = { check-cfg = ['cfg(docsrs_dep)'] }
```

## Local Testing

Build the documentation with the extension enabled like this:

```bash
RUSTDOCFLAGS="--html-after-content docs-rs/trait-tags.html --cfg docsrs_dep" RUSTFLAGS="--cfg docsrs_dep" cargo doc --no-deps --package <package_name>
```

[rustdoc]: https://doc.rust-lang.org/rustdoc/what-is-rustdoc.html
[docs.rs]: https://docs.rs
