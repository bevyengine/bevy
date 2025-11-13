---
title: Cargo Feature Collections
authors: ["@cart"]
pull_requests: [21472]
---

Historically, Bevy developers have lived one of two lifestyles:

1. Use all of Bevy's default features, potentially compiling many unwanted or unneeded features.
2. Disable Bevy's default features and manually define the complete list of features.

Living in the world of (2) was an exercise in frustration, as the list of bevy features is _massive_ and the features required to accomplish a given task changes regularly across releases. This was an _expert level_ task that required intimate knowledge of engine internals to get right.

**Bevy 0.18** introduces high-level "cargo feature collections" to the `bevy` crate: `2d`, `3d`, and `ui`. This enables developers to easily select the kind of app they want to build, and only compile the pieces of Bevy needed for that app.

This means scenarios like using Bevy as a UI framework, without pulling in the rest of the engine, is now as easy as:

```toml
bevy = { version = "0.18", default-features = false, features = ["ui"] }
```

We've also added mid-level feature collections like `2d_api`, which is Bevy's 2D API _without the default Bevy renderer_. This makes it much easier to swap out the default Bevy renderer for a custom one.

For example, the `2d` profile looks like this:

```toml
2d = [
  "default_app",
  "default_platform",
  "2d_api",
  "2d_bevy_render",
  "ui",
  "scene",
  "audio",
  "picking",
]
```

Someone building a custom 2D renderer now just needs to remove `2d_bevy_render` and provide their own.

Developers can now define their own high-level cargo feature profiles from these mid-level pieces, making it _much_ easier to define the subset of Bevy you want to build into your app.
