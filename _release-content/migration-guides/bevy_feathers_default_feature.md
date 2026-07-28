---
title: "FeathersPlugins and the `bevy_feathers` feature are now included/on by default"
pull_requests: [13723]
---

`FeathersPlugins` are now part of `DefaultPlugins`. If you were adding the `FeathersPlugins` explicitly in your app, you can now rely on `DefaultPlugins` to include it, provided the `bevy_feathers` feature is enabled.

`bevy_feathers` is now a default feature. If you use default features and would not like to use `bevy_feathers`, you should only enable the `2d`, `3d`, `ui`, and/or `audio` feature groups.
