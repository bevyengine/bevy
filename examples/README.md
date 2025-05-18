<!-- MD024 - The Headers from the Platform-Specific Examples should be identical  -->
<!-- Use 'cargo run -p build-templated-pages -- build-example-page' to generate the final example README.md -->
<!-- markdownlint-disable-file MD024 -->

# Examples

To run an example, use the command `cargo run --example <Example>` with the short name (without any subdirectory) of the example.

On Linux/BSD/etc, add the option `--features wayland` to build with native support for Wayland graphical environments. Bevy only enables X11 support by default.

```sh
cargo run --features wayland --example 3d_scene
```

(runs the example in `api/3d/3d_scene`)

**⚠️ Note: for users of releases on crates.io!**

There are often large differences and incompatible API changes between the latest [crates.io](https://crates.io/crates/bevy) release and the development version of Bevy in the git main branch!

If you are using a released version of bevy, you need to make sure you are viewing the correct version of the examples!

- Latest release (`latest` branch): [https://github.com/bevyengine/bevy/tree/latest/examples](https://github.com/bevyengine/bevy/tree/latest/examples)
- Specific version (such as `v0.16.0`): [https://github.com/bevyengine/bevy/tree/v0.16.0/examples](https://github.com/bevyengine/bevy/tree/v0.16.0/examples)

When you clone the repo locally to run the examples, use `git checkout` to get the correct version:

```bash
# `latest` always points to the newest release
git checkout latest
# or use a specific version
git checkout v0.16.0
```

---

There are several categories of examples:

- [API Examples](./api/): Showing various features of Bevy and how to use them.
- [Usage Examples](./usage/): Showing how to accomplish various common gamedev tasks using Bevy.
- [Game Examples](./games/): Various simple games, showing how to make a more complete project in Bevy.

We also have additional example categories, which are not necessarily intended to teach you how to use Bevy, but rather useful to have in the repo:
- [Tools](./tools/): Things that might be useful when developing with Bevy.
- [Stress Tests](../stress_tests/): Doing absurdly computationally-intensive things in Bevy, to benchmark performance.
- Testbeds: designed to test features and scenes that rely on graphical rendering or interactivity, in both an automated and manual fashion.

Click on the above links to explore our rich collection of examples!

## Platform Support

The above examples are cross-platform. They should run on desktop platforms out of the box, can be adapted to mobile without code changes (see below), and most should work on Web (WASM).

Mobile and Web have some special considerations when building and do not support all features of the desktop platforms. Some examples that use advanced functionality might not run, or run poorly.

See the [Mobile](./mobile/) example to learn what is needed to run Bevy on Android and iOS.

See the [WASM](./wasm/) README to learn what is needed to run Bevy in a web browser.

Some parts of Bevy can also run on embedded and various other constrained or legacy platforms. See the [`no_std`](./no_std/) folder.

## Tests

We have some examples to teach you how to write tests for your Bevy code.

Example | Description
--- | ---
[How to Test Apps](../tests/how_to_test_apps.rs) | How to test apps (simple integration testing)
[How to Test Systems](../tests/how_to_test_systems.rs) | How to test systems with commands, queries or resources
