# [![Bevy](assets/branding/bevy_logo_light_dark_and_dimmed.svg)](https://bevyengine.org)

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/bevyengine/bevy#license)
[![Crates.io](https://img.shields.io/crates/v/bevy.svg)](https://crates.io/crates/bevy)
[![Downloads](https://img.shields.io/crates/d/bevy.svg)](https://crates.io/crates/bevy)
[![Docs](https://docs.rs/bevy/badge.svg)](https://docs.rs/bevy/latest/bevy/)
[![CI](https://github.com/bevyengine/bevy/workflows/CI/badge.svg)](https://github.com/bevyengine/bevy/actions)
[![Discord](https://img.shields.io/discord/691052431525675048.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/bevy)

## What is Bevy?

Bevy is a refreshingly simple data-driven game engine built in Rust. 
It is designed to be **fast, modular, and easy to use**, offering a modern approach to game development. It is free and open-source forever!

- **No Garbage Collection**: Bevy uses Rust's **ownership model** for memory safety.  
- **Entity Component System (ECS)**: Efficient data-driven architecture for handling complex game logic.  
- **Parallel Execution**: Bevy **automatically runs systems in parallel**, improving performance.  
- **Cross-Platform**: Runs on **Windows, macOS, Linux, and WebAssembly** (WASM).

## WARNING

Bevy is still in the early stages of development and is actively evolving. Which means:
- Some **important features are missing**.  
- **Documentation is still being improved**.  
- A **new version is released [approximately once every 3 months](https://bevyengine.org/news/bevy-0-6/#the-train-release-schedule)**, often with **breaking API changes**
- **[Migration guides](https://bevyengine.org/learn/migration-guides/)** are available, but updating to new versions may require manual adjustments.

Despite these challenges, Bevy provides a solid framework for Rust-based game development, and the community is actively improving the engine with each release.

**MSRV:** Bevy relies heavily on improvements in the Rust language and compiler.
As a result, the Minimum Supported Rust Version (MSRV) is generally close to "the latest stable release" of Rust.

## Design Goals
Bevy is built for **performance, flexibility, and ease of use**, following these principles:

### **Capable: Full 2D and 3D Support**
Bevy provides a **modern rendering engine**, **physics integration**, and **asset management** for both 2D and 3D development.

### **Simple: Easy to Use, Powerful When Needed**
Minimal setup with a **clear API**, **no garbage collection**, and **Rust’s strong type system** to ensure safety and efficiency.

### **Data-Oriented: Optimized with ECS**
Uses the **Entity Component System (ECS)** for **scalability, parallel execution, and performance**.

### **Modular: Use Only What You Need**
Developers can **enable or replace components** as needed, keeping projects lightweight and customizable.

### **Fast: High-Performance Execution**
Bevy optimizes **parallel processing, cache efficiency, and system execution** for smooth gameplay.

### **Productive: Faster Iteration**
Supports **hot-reloading, rapid prototyping, and reduced compile times** for better development workflow.

These principles ensure Bevy remains a **powerful yet user-friendly** engine for both small and large projects.

## About
Bevy provides a variety of resources to help users get started and stay up to date:

* **[Features](https://bevyengine.org):** A quick overview of Bevy's features.
* **[News](https://bevyengine.org/news/)**: A development blog that covers our progress, plans and shiny new features.
* **[Bevy Showcase](https://bevyengine.org/showcase/)**: A collection of games and projects built using Bevy.  
* **[Bevy Plugins](https://bevyengine.org/assets/)**: A curated list of community-contributed plugins and tools.  
* **[Rust Game Development Resources](https://arewegameyet.rs/)**: External resources for learning Rust game development.

Bevy is actively developed by **a passionate open-source community**, and new contributors are always welcome!

## Docs
Bevy offers a range of official and community-maintained documentation:

* **[Quick Start Guide](https://bevyengine.org/learn/quick-start/introduction):** Bevy's official Quick Start Guide. The best place to start learning Bevy.
* **[Bevy Rust API Docs](https://docs.rs/bevy):** Bevy's Rust API docs, which are automatically generated from the doc comments in this repo.
* **[Official Examples](https://github.com/bevyengine/bevy/tree/latest/examples):** Bevy's dedicated, runnable examples, which are great for digging into specific concepts.
* **[Community-Made Learning Resources](https://bevyengine.org/assets/#learning)**: More tutorials, documentation, and examples made by the Bevy community.

## Community
Before contributing or participating in discussions with the community, you should familiarize yourself with our [**Code of Conduct**](./CODE_OF_CONDUCT.md).

Ways to get involved:

- **[Discord](https://discord.gg/bevy)** – The official Bevy Discord server for real-time discussions.  
- **[Reddit](https://www.reddit.com/r/bevy/)** – A space for Bevy-related discussions and news.  
- **[GitHub Discussions](https://github.com/bevyengine/bevy/discussions)** – A place for questions, ideas, and feature discussions.  
- **[Bevy Assets](https://bevyengine.org/assets/)** – A curated list of Bevy projects, tools, plugins, and learning materials.

### Contributing

If you’d like to help improve Bevy, follow these steps:

- **Check out the [Contributor's Guide](https://github.com/bevyengine/bevy/blob/main/CONTRIBUTING.md)** for an overview of how to contribute.  
- **Browse the [issue tracker](https://github.com/bevyengine/bevy/issues)** for open tasks.  
- **For simple fixes**, feel free to open a **pull request (PR)**.  
- **For major changes**, submit a **Request For Comments (RFC)** so the community can discuss the best approach.  

Your contributions help make Bevy a better engine for everyone!

## Getting Started

We recommend checking out the [Quick Start Guide](https://bevyengine.org/learn/quick-start/introduction) for a brief introduction.

### **Setting Up Bevy**
Follow the [Setup guide](https://bevyengine.org/learn/quick-start/getting-started/setup) to ensure your development environment is set up correctly.
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

Bevy can be built just fine using default configuration on stable Rust. However for really fast iterative compiles, you should enable the "fast compiles" setup by [following the instructions here](https://bevyengine.org/learn/quick-start/getting-started/setup).

## [Bevy Cargo Features][cargo_features]
Bevy supports various **Cargo features** that allow customization based on project needs.

This [list][cargo_features] outlines the different cargo features supported by Bevy. These allow you to customize the Bevy feature set for your use-case.

[cargo_features]: docs/cargo_features.md

## Thanks

Bevy is the result of the hard work of many people. A huge thanks to all Bevy contributors, the many open source projects that have come before us, the [Rust gamedev ecosystem](https://arewegameyet.rs/), and the many libraries we build on.

A huge thanks to Bevy's [generous sponsors](https://bevyengine.org). Bevy will always be free and open source, but it isn't free to make. Please consider [sponsoring our work](https://bevyengine.org/donate/) if you like what we're building.

<!-- This next line need to stay exactly as is. It is required for BrowserStack sponsorship. -->
This project is tested with BrowserStack.

## License

Bevy is free, open source and permissively licensed!
Except where noted (below and/or in individual files), all code in this repository is dual-licensed under either:

* MIT License ([LICENSE-MIT](LICENSE-MIT) or [http://opensource.org/licenses/MIT](http://opensource.org/licenses/MIT))
* Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0))

This means you can select the license you prefer!
This dual-licensing approach is the de-facto standard in the Rust ecosystem and there are [very good reasons](https://github.com/bevyengine/bevy/issues/2373) to include both.

Some of the engine's code carries additional copyright notices and license terms due to their external origins.
These are generally BSD-like, but exact details vary by crate:
If the README of a crate contains a 'License' header (or similar), the additional copyright notices and license terms applicable to that crate will be listed.
The above licensing requirement still applies to contributions to those crates, and sections of those crates will carry those license terms.
The [license](https://doc.rust-lang.org/cargo/reference/manifest.html#the-license-and-license-file-fields) field of each crate will also reflect this.
For example, [`bevy_mikktspace`](./crates/bevy_mikktspace/README.md#license-agreement) has code under the Zlib license (as well as a copyright notice when choosing the MIT license).

The [assets](assets) included in this repository (for our [examples](./examples/README.md)) typically fall under different open licenses.
These will not be included in your game (unless copied in by you), and they are not distributed in the published bevy crates.
See [CREDITS.md](CREDITS.md) for the details of the licenses of those files.

### Your contributions

Unless you explicitly state otherwise,
any contribution intentionally submitted for inclusion in the work by you,
as defined in the Apache-2.0 license,
shall be dual licensed as above,
without any additional terms or conditions.
