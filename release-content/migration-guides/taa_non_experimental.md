---
title: TAA is no longer experimental
pull_requests: [18349]
---

TAA is no longer experimental.

`TemporalAntiAliasPlugin` no longer needs to be added to your app to use TAA. It is now part of `DefaultPlugins`, via `AntiAliasPlugin`.

As part of this change, the import paths for `TemporalAntiAliasNode`, `TemporalAntiAliasing` and `TemporalAntiAliasPlugin` have changed from `bevy::anti_alias::experimental::taa` to `bevy::anti_alias::taa`: if you want to add `TemporalAntiAliasing` to a Camera, you can now find it at `bevy::anti_alias::taa::TemporalAntiAliasing`.

`TemporalAntiAliasing` now uses `MipBias` as a required component in the main world, instead of overriding it manually in the render world.
