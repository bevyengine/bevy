# ![Bevy](assets/temp_bevy_logo.png)
[![Crates.io](https://img.shields.io/crates/v/bevy.svg)](https://crates.io/crates/bevy)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/bevyengine/bevy/LICENSE)
[![Crates.io](https://img.shields.io/crates/d/bevy.svg)](https://crates.io/crates/bevy)

## What is Bevy?

Bevy is a modern data-driven game engine built in Rust

## WARNING

Bevy is still in the _very_ stages of development. APIs can and will change. Important features are missing. Please don't build any serious projects in Bevy unless you are prepared to be broken by api changes constantly.

## Design Goals

* Provide a first class user-experience for both 2D and 3D games.
* Easy for newbies to pick up, but infinitely flexible for power users.
* Fast iterative compile times. Ideally less than 1 second for small to medium sized projects.
* Data-first game development using ECS (Entity Component Systems)
* High performance and parallel architecture
* Use the latest and greatest rendering technologies and techniques

## Features

* Cross Platform: Windows, MacOS, Linux
* Modern Renderer
    * Multiple Backends: Vulkan, DirectX 11/12, Metal
    * Modern and flexible low level "Render Graph" api
        * Easy to use high level defaults for beginners 
        * Experts can create their own Render Graphs or modify the defaults
* Expressive UI System
* Fast iterative compile times
    * the example projects have less than 1 second iterative compiles
* Dynamically load plugins at runtime
    * "script" your game in Rust or extend the engine with new features

## Planned Features

* Physically Based Rendering
* Load scenes from files
* Editor (built using Bevy)
* Live Plugin Reloading
* GLTF model loading
* Gamepad support
* Networking
* More Platforms: Android, iOS, Web

## Getting Started

### Examples:

```
cargo run --example simple
```

### Fast Compiles

* Bevy can be built using stable rust with default configuration (ex: ```cargo build```), but for optimal build times we recommend using a nightly compiler with the following settings:
    * LLD Linker: ```-Clink-arg=-fuse-ld=lld```
        * LLD will significantly speed up compile times in Bevy, but it doesn't work out of the box on some platforms / os-es.
        * See [this issue](https://github.com/rust-lang/rust/issues/39915) and [this issue](https://github.com/rust-gamedev/wg/issues/50) for more information.
    * Generic Sharing (nightly only): ```-Zshare-generics=y```
        * Most of the generics you will use in Bevy apps are also used by the Bevy engine code. Generic sharing is a nightly feature that lets your Bevy app re-use generics used in Bevy engine.
    * Oddly in some cases compiling in release mode can actually reduce iterative compile times with the settings above. In our experience this is true for most of the examples in this project.
    * You can set these flags in one of two ways:
        * [.cargo/config](https://doc.rust-lang.org/cargo/reference/config.html)
            * We have included an example configuration in ```.cargo/config_fast_builds```. Try renaming it to ```.cargo/config``` and see if it works!
        * [environment variables](https://doc.rust-lang.org/cargo/reference/environment-variables.html):
            * For example, in Bash you would run this command: ```RUSTFLAGS="-Clink-arg=-fuse-ld=lld -Zshare-generics=y" cargo build --release```

## Libraries Used

Bevy is only possible because of the hard work put into these foundational technologies:

* [wgpu-rs](https://github.com/gfx-rs/wgpu-rs): modern / low-level / cross platform graphics library inspired by Vulkan
* [legion](https://github.com/TomGillen/legion): a feature rich high performance ECS library
* [glam-rs](https://github.com/bitshifter/glam-rs): a simple and fast 3D math library for games and graphics
* [winit](https://github.com/rust-windowing/winit): cross platform window creation and management in Rust
* [legion_transform](https://github.com/AThilenius/legion_transform): A hierarchical space transform system, implemented using Legion ECS

Additionally, we would like to thank the [Amethyst](https://github.com/amethyst/amethyst), [coffee](https://github.com/hecrj/coffee), [ggez](https://github.com/ggez/ggez), and [Piston](https://github.com/PistonDevelopers/piston) projects for providing solid examples of game engine development in Rust. If you are looking for a game engine to build Rust games in, it is worth considering all of your options. Each engine has different design goals and some will likely resonate with you more than others. 

## F.A.Q.

#### Why build Bevy instead of using INSERT_GAME_ENGINE_HERE?

@cart (original creator of Bevy) speaking: I decided to build Bevy after years of contributing code to other engines (ex: Godot). I spent over four years building a game in Godot and I have experience with Unity, Unreal, Three.js, Armory, and Game Maker. I built multiple custom engines using Rust, Go, HTML5, and Java. I have also closely followed the other major players in the Rust gamedev ecosystem, namely [Amethyst](https://github.com/amethyst/amethyst), [coffee](https://github.com/hecrj/coffee), and [Piston](https://github.com/PistonDevelopers/piston). I am currently a senior software engineer at Microsoft which has also affected my opinions of software and what it should be.

These experiences led me to want the following from a game engine:

* It needs to be free and open source with no strings attached. Games are a huge part of our culture and humanity is investing _millions_ of hours into the development of games. Why are we (as game developers / engine developers) continuing to build up the ecosystems of closed-source monopolies that take cuts of our sales and deny us visibilty into the tech we use daily? As a community I believe we can do so much better.
* It needs to have fast build/run/test loops, which translates to either scripting languages or fast compile times in native languages. But scripting languages introduce runtime overhead and cognitive load. 
* Ideally the engine is written in the same lanugage as games are. I found myself loving engines where game code is written in the same language as engine code. Being able to run an IDE "go to definition" command on a symbol in your game and hop directly into the engine source is an extremely powerful concept. And you don't need to worry about translation layers or lossy abstractions. Also, if an engine's community builds games in the same language as the engine, they are more likely (and able) to contribute back to the engine.
* It needs to be easy to use for common tasks, but it also can't hide the details from you. Many engines are either "easy to use but too high level" or "very low level but difficult to do common tasks in".
* It needs to have an editor. Scene creation is a large part of game development and in many cases visual editors beat code. As a bonus, the editor should be built _in the engine_. Godot uses this approach and it is _so smart_. Doing so [dogfoods](https://en.wikipedia.org/wiki/Eating_your_own_dog_food) the engine's UI system. Improvements to the editor are also often improvements to the engine. And it makes sure your engine is flexible enough to build tooling (and not just games).
* It needs to be data-driven/data-oriented/data-first. ECS is a common way of doing this, but it definitely isn't the only way. These paradigms can make your game faster (cache friendly, easier to parallelize), but they also make common tasks like game state serialization and synchronization delightfully straightforward.

None of the engines on the market _quite_ line up with what I'm looking for. And the changes required to make them meet my requirements are either massive in scope, impossible (closed source), or unwelcome (the things I want aren't what the developers or customers want). On top of that, making new game engines is fun!

Bevy is not trying to out-compete other open-source game engines. As much as possible we should be collaborating and building common foundations. If you are an open source game engine developer and you think a Bevy component would make your engine better, one of your engine's components could make Bevy better, or both, please reach out! Bevy is already benefitting massively from the efforts of the Rust gamedev ecosystem and we would love to pay it forward in whatever way we can.