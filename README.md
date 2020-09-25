# [![Bevy](assets/branding/bevy_logo_light_small.svg)](https://bevyengine.org)
[![Crates.io](https://img.shields.io/crates/v/bevy.svg)](https://crates.io/crates/bevy)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/bevyengine/bevy/blob/master/LICENSE)
[![Crates.io](https://img.shields.io/crates/d/bevy.svg)](https://crates.io/crates/bevy)
[![Rust](https://github.com/bevyengine/bevy/workflows/CI/badge.svg)](https://github.com/bevyengine/bevy/actions)

## What is Bevy?

Bevy is a refreshingly simple data-driven game engine built in Rust. It is free and open-source forever!

## WARNING

Bevy is still in the _very_ early stages of development. APIs can and will change (now is the time to make suggestions!). Important features are missing. Documentation is sparse. Please don't build any serious projects in Bevy unless you are prepared to be broken by api changes constantly.

## Design Goals

* **Capable**: Offer a complete 2D and 3D feature set 
* **Simple**: Easy for newbies to pick up, but infinitely flexible for power users
* **Data Focused**: Data-oriented architecture using the Entity Component System paradigm 
* **Modular**: Use only what you need. Replace what you don't like
* **Fast**: App logic should run quickly, and when possible, in parallel
* **Productive**: Changes should compile quickly ... waiting isn't fun

## About

* **[Features](https://bevyengine.org):** A quick overview of Bevy's features.
* **[Roadmap](https://github.com/bevyengine/bevy/projects/1):** The Bevy team's development plan.
* **[Introducing Bevy](https://bevyengine.org/news/introducing-bevy/)**: A blog post covering some of Bevy's features

## Docs

* **[The Bevy Book](https://bevyengine.org/learn/book/introduction):** Bevy's official documentation. The best place to start learning Bevy. 
* **[Bevy Rust API Docs](https://docs.rs/bevy):** Bevy's Rust API docs, which are automatically generated from the doc comments in this repo.

## Community
Before contributing or participating in discussions with the community, you should familiarize yourself with our **[Code of Conduct](https://github.com/bevyengine/bevy/blob/master/CODE_OF_CONDUCT.md)**

* **[Discord](https://discord.gg/gMUk5Ph):** Bevy's official discord server.
* **[Reddit](https://reddit.com/r/bevy):** Bevy's official subreddit.
* **[Stack Overflow](https://stackoverflow.com/questions/tagged/bevy):** Questions tagged Bevy on Stack Overflow.
* **[Awesome Bevy](https://github.com/bevyengine/awesome-bevy):** A collection of awesome Bevy projects.

## Getting Started

We recommend checking out [The Bevy Book](https://bevyengine.org/learn/book/introduction) for a full tutorial.

Follow the [Setup guide](https://bevyengine.org/learn/book/getting-started/setup/) to ensure your development environment is setup correctly.
Once setup, you can quickly try out the [examples](/examples) by cloning this repo and running the following command:

```sh
# Runs the "breakout" example
cargo run --example breakout
```

### Fast Compiles

Bevy can be built just fine using default configuration on stable Rust. However for really fast iterative compiles, you should enable the "fast compiles" setup by [following the instructions here](http://bevyengine.org/learn/book/getting-started/setup/).

## Focus Areas

Bevy has the following [Focus Areas](https://github.com/bevyengine/bevy/labels/focus-area). We are currently focusing our development efforts in these areas and they will receive priority for Bevy developers' time. If you would like to contribute to Bevy, you are heavily encouraged to join in on these efforts:

### [Editor-Ready UI](https://github.com/bevyengine/bevy/issues/254)
### [PBR / Clustered Forward Rendering](https://github.com/bevyengine/bevy/issues/179)
### [Scenes](https://github.com/bevyengine/bevy/issues/255)

## Libraries Used

Bevy is only possible because of the hard work put into these foundational technologies:

* [wgpu-rs](https://github.com/gfx-rs/wgpu-rs): modern / low-level / cross platform graphics library inspired by Vulkan
* [glam-rs](https://github.com/bitshifter/glam-rs): a simple and fast 3D math library for games and graphics
* [winit](https://github.com/rust-windowing/winit): cross platform window creation and management in Rust
* [spirv-reflect](https://github.com/gwihlidal/spirv-reflect-rs): Reflection API in rust for SPIR-V shader byte code

## [Bevy Cargo Features][cargo_features]

This [list][cargo_features] outlines the different cargo features supported by Bevy. These allow you to customize the Bevy feature set for your use-case.

[cargo_features]: docs/cargo_features.md

## Thanks and Alternatives

Additionally, we would like to thank the [Amethyst](https://github.com/amethyst/amethyst), [macroquad](https://github.com/not-fl3/macroquad), [coffee](https://github.com/hecrj/coffee), [ggez](https://github.com/ggez/ggez), [rg3d](https://github.com/mrDIMAS/rg3d), and [Piston](https://github.com/PistonDevelopers/piston) projects for providing solid examples of game engine development in Rust. If you are looking for a Rust game engine, it is worth considering all of your options. Each engine has different design goals and some will likely resonate with you more than others. 
