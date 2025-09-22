---
title: Data-Driven Materials
authors: [ "@tychedelia" ]
pull_requests: [ 19667 ]
---

Bevy's material system has historically relied on the `Material` and `AsBindGroup` traits in order to provide a
type-safe way to define data that is passed to the shader that renders your material. While this approach has
many advantages, recent improvements to the renderer like GPU-driven rendering in Bevy `0.16` have made the 3D renderer
more siloed and less modular than we would like. Additionally, the type-level split between `Material` and `Material2d`
has meant that every feature implemented for 3D needs a mostly copy-pasted version for 2D, which has caused the 2D
renderer to lag behind in terms of features.

In Bevy `0.17`, we've started the process of refactoring the renderer's mid and low-level APIs to be *data driven*. More
specifically, we've removed the `M: Material` bound from every rendering system in the render world. Rather than being
described statically by a type, the renderer now understands materials in terms of plain data that can be modified at
runtime. Consequently, it is now possible to implement a custom material that doesn't rely on the `Material` trait at
all, for example in the
new [manual material example](https://github.com/bevyengine/bevy/blob/8b36cca28c4ea00425e1414fd88c8b82297e2b96/examples/3d/manual_material.rs).
While this API isn't exactly ergonomic yet, it represents a first step in decoupling the renderer from a specific
high-level material API.

Importantly, for users of the `Material` trait, nothing changes. Our `AsBindGroup` driven API is now just one possible
consumer of the renderer. But adopting a more dynamic, data-first approach creates many opportunities for the renderer
we are hoping to explore in `0.18` and beyond, including:

- Unifying the 2D and 3D rendering implementations. While we'll continue to present an opinionated 2D API that benefits
  users building 2D games, we want every new rendering improvement to the 3D renderer to be at least potentially
  available
  to 2D users.
- Exploring new material representations for a future material editor. While type-safety is great for writing code, it
  poses real problems for being able to dynamically edit a material in a UI like a shader graph or load a material at
  runtime from a serialized format.
- Modularizing more of the mid-level rendering APIs to allow user's writing advanced rendering code access to
  complicated pieces of rendering infrastructure like mesh and bind group allocation, GPU pre-processing, retained
  rendering caches, and custom draw functions.

With this foundation in place, we're actively evolving the renderer to embrace the flexibility and composability
that defines Bevy's ECS. If you'd like to help us explore the possibilities of ECS-driven rendering, please join us on
[Discord](https://discord.gg/bevy) or [GitHub Discussions](https://github.com/bevyengine/bevy/discussions)!
