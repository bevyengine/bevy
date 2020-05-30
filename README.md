# [![Bevy](assets/branding/bevy_logo_light_small.svg)](https://bevyengine.org)
[![Crates.io](https://img.shields.io/crates/v/bevy.svg)](https://crates.io/crates/bevy)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/bevyengine/bevy/LICENSE)
[![Crates.io](https://img.shields.io/crates/d/bevy.svg)](https://crates.io/crates/bevy)

## What is Bevy?

Bevy is an open-source modular game engine built in Rust, with a focus on developer productivity and performance.

## WARNING

Bevy is still in the _very_ early stages of development. APIs can and will change. Important features are missing. Documentation is non-existent. Please don't build any serious projects in Bevy unless you are prepared to be broken by api changes constantly.

## Design Goals

* Provide a first class developer experience for both 2D and 3D games.
* Easy for newbies to pick up, but infinitely flexible for power users.
* Fast iterative compile times. Ideally less than 1 second for small to medium sized projects.
* Data-first game development using ECS (Entity Component System)
* Modular design: use only what you need ... replace what you don't like
* High performance and parallel architecture
* Use the latest and greatest rendering technologies and techniques

## About

* **[Features](https://bevyengine.org/learn/book/introduction/features):** A quick overview of Bevy's features.
* **[Roadmap](https://bevyengine.org/learn/book/contributing/roadmap):** The Bevy team's development plan.

## Docs

* **[The Bevy Book](https://bevyengine.org/learn/book/introduction):** Bevy's official documentation. The best place to start learning Bevy. 
* **[Bevy Rust API Docs](https://docs.rs/bevy):** Bevy's Rust API docs, which are automatically generated from the doc comments in this repo.

## Getting Started

We recommend checking out [The Bevy Book](https://bevyengine.org/learn/book/introduction) for a full tutorial. You can quickly try out the [examples](/examples) by cloning this repo and running the following command:

```sh
# Runs the "scene" example
cargo run --example scene
```

### Nightly Compiler

Bevy requires nightly rust right now. It currently uses [specialization](https://github.com/rust-lang/rfcs/blob/master/text/1210-impl-specialization.md) features, which are unstable. If specialization goes stable soon then we can go back to a stable compiler. [There is actually good forward progress happening here](https://github.com/rust-lang/rust/pull/68970). In the meantime, we will try our best to remove specialization usage so we can go back to stable.


## Libraries Used

Bevy is only possible because of the hard work put into these foundational technologies:

* [wgpu-rs](https://github.com/gfx-rs/wgpu-rs): modern / low-level / cross platform graphics library inspired by Vulkan
* [legion](https://github.com/TomGillen/legion): a feature rich high performance ECS library
* [glam-rs](https://github.com/bitshifter/glam-rs): a simple and fast 3D math library for games and graphics
* [winit](https://github.com/rust-windowing/winit): cross platform window creation and management in Rust
* [legion_transform](https://github.com/AThilenius/legion_transform): A hierarchical space transform system, implemented using Legion ECS
* [shaderc](https://github.com/google/shaderc-rs): compiles GLSL and HLSL shaders to SPIR-V binaries
* [spirv-reflect](https://github.com/gwihlidal/spirv-reflect-rs): Reflection API in rust for SPIR-V shader byte code


Additionally, we would like to thank the [Amethyst](https://github.com/amethyst/amethyst), [coffee](https://github.com/hecrj/coffee), [ggez](https://github.com/ggez/ggez), and [Piston](https://github.com/PistonDevelopers/piston) projects for providing solid examples of game engine development in Rust. If you are looking for a Rust game engine, it is worth considering all of your options. Each engine has different design goals and some will likely resonate with you more than others. 