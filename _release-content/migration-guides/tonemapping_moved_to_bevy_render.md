---
title: "`Tonemapping` and `DebandDither` moved to `bevy_render::view`"
pull_requests: [25480]
---

`Tonemapping` and `DebandDither` moved from `bevy_core_pipeline::tonemapping` to
`bevy_render::view`. Rust imports still work through re-exports, but the reflected type
paths changed: update scene files and Bevy Remote Protocol component keys that name them.

- `bevy_core_pipeline::tonemapping::Tonemapping` is now `bevy_render::view::Tonemapping`
- `bevy_core_pipeline::tonemapping::DebandDither` is now `bevy_render::view::DebandDither`
