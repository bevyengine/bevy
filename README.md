# [![Bevy](assets/branding/bevy_logo_light_dark_and_dimmed.svg)](https://bevyengine.org)

[![Crates.io](https://img.shields.io/crates/v/bevy.svg)](https://crates.io/crates/bevy)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](./LICENSE)
[![Crates.io](https://img.shields.io/crates/d/bevy.svg)](https://crates.io/crates/bevy)
[![Rust](https://github.com/bevyengine/bevy/workflows/CI/badge.svg)](https://github.com/bevyengine/bevy/actions)
![iOS cron CI](https://github.com/bevyengine/bevy/workflows/iOS%20cron%20CI/badge.svg)
[![Discord](https://img.shields.io/discord/691052431525675048.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/bevy)

## What is Bevy?

Bevy is a refreshingly simple data-driven game engine built in Rust. It is free and open-source forever!

## WARNING

Bevy is still in the _very_ early stages of development. APIs can and will change (now is the time to make suggestions!). Important features are missing. Documentation is sparse. Please don't build any serious projects in Bevy unless you are prepared to be broken by API changes constantly.

**MSRV:** Bevy relies heavily on improvements in the Rust language and compiler.
As a result, the Minimum Supported Rust Version (MSRV) is "the latest stable release" of Rust.

## Design Goals

* **Capable**: Offer a complete 2D and 3D feature set
* **Simple**: Easy for newbies to pick up, but infinitely flexible for power users
* **Data Focused**: Data-oriented architecture using the Entity Component System paradigm
* **Modular**: Use only what you need. Replace what you don't like
* **Fast**: App logic should run quickly, and when possible, in parallel
* **Productive**: Changes should compile quickly ... waiting isn't fun

## About

* **[Features](https://bevyengine.org):** A quick overview of Bevy's features.
* **[News](https://bevyengine.org/news/)**: A development blog that covers our progress, plans and shiny new features.

## Docs

* **[The Bevy Book](https://bevyengine.org/learn/book/introduction):** Bevy's official documentation. The best place to start learning Bevy.
* **[Bevy Rust API Docs](https://docs.rs/bevy):** Bevy's Rust API docs, which are automatically generated from the doc comments in this repo.
* **[Official Examples](https://github.com/bevyengine/bevy/tree/latest/examples):** Bevy's dedicated, runnable examples, which are great for digging into specific concepts.
* **[Community-Made Learning Resources](https://bevyengine.org/assets/#learning)**: More tutorials, documentation, and examples made by the Bevy community.

## Community

Before contributing or participating in discussions with the community, you should familiarize yourself with our [**Code of Conduct**](./CODE_OF_CONDUCT.md).

* **[Discord](https://discord.gg/bevy):** Bevy's official discord server.
* **[Reddit](https://reddit.com/r/bevy):** Bevy's official subreddit.
* **[GitHub Discussions](https://github.com/bevyengine/bevy/discussions):** The best place for questions about Bevy, answered right here!
* **[Bevy Assets](https://bevyengine.org/assets/):** A collection of awesome Bevy projects, tools, plugins and learning materials.

If you'd like to help build Bevy, check out the **[Contributor's Guide](https://github.com/bevyengine/bevy/blob/main/CONTRIBUTING.md)**.
For simple problems, feel free to open an issue or PR and tackle it yourself!

For more complex architecture decisions and experimental mad science, please open an [RFC](https://github.com/bevyengine/rfcs) (Request For Comments) so we can brainstorm together effectively!

## Getting Started

We recommend checking out [The Bevy Book](https://bevyengine.org/learn/book/introduction) for a full tutorial.

Follow the [Setup guide](https://bevyengine.org/learn/book/getting-started/setup/) to ensure your development environment is set up correctly.
Once set up, you can quickly try out the [examples](https://github.com/bevyengine/bevy/tree/latest/examples) by cloning this repo and running the following commands:

```sh
# Switch to the correct version (latest release, default is main development branch)
git checkout latest
# Runs the "breakout" example
cargo run --example breakout
```

To draw a window with standard functionality enabled, use:

```rust
use bevy::prelude::*;

fn main(){
  App::new()
    .add_plugins(DefaultPlugins)
    .run();
}
```

### Fast Compiles

Bevy can be built just fine using default configuration on stable Rust. However for really fast iterative compiles, you should enable the "fast compiles" setup by [following the instructions here](http://bevyengine.org/learn/book/getting-started/setup/).

## Libraries Used

Bevy is only possible because of the hard work put into these foundational technologies:

* [wgpu](https://wgpu.rs/): modern / low-level / cross-platform graphics library inspired by Vulkan
* [glam-rs](https://github.com/bitshifter/glam-rs): a simple and fast 3D math library for games and graphics
* [winit](https://github.com/rust-windowing/winit): cross-platform window creation and management in Rust
* [spirv-reflect](https://github.com/gwihlidal/spirv-reflect-rs): Reflection API in rust for SPIR-V shader byte code

## [Bevy Cargo Features][cargo_features]

This [list][cargo_features] outlines the different cargo features supported by Bevy. These allow you to customize the Bevy feature set for your use-case.

[cargo_features]: docs/cargo_features.md

## [Third Party Plugins][plugin_guidelines]

Plugins are very welcome to extend Bevy's features. [Guidelines][plugin_guidelines] are available to help integration and usage.

[plugin_guidelines]: docs/plugins_guidelines.md

## Thanks and Alternatives

Additionally, we would like to thank the [Amethyst](https://github.com/amethyst/amethyst), [macroquad](https://github.com/not-fl3/macroquad), [coffee](https://github.com/hecrj/coffee), [ggez](https://github.com/ggez/ggez), [Fyrox](https://github.com/FyroxEngine/Fyrox), and [Piston](https://github.com/PistonDevelopers/piston) projects for providing solid examples of game engine development in Rust. If you are looking for a Rust game engine, it is worth considering all of your options. Each engine has different design goals, and some will likely resonate with you more than others.

## License

Bevy is free and open source! All code in this repository is dual-licensed under either:

* MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

at your option. This means you can select the license you prefer! This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
