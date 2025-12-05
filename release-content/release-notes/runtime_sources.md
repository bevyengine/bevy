---
title: Adding and removing asset sources at runtime
authors: ["@andriyDev"]
pull_requests: [21890]
---

Custom asset sources are a great way to extend the asset system to access data from all sorts of
sources, whether that be a file system, or a webserver, or a compressed package. Unfortunately, in
previous versions, asset sources could **only** be added before the app starts! This prevents users
from choosing their sources at runtime.

For a concrete example, consider the case of an application which allows you to pick a `zip` file to
open. Internally, a `zip` is its own little filesystem. Representing this as an asset source is
quite natural and allows loading just the parts you need. However, since we couldn't previously add
asset sources at runtime, this wasn't possible!

Now you can add asset sources quite easily!

```rust
fn add_source_and_load(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) -> Result<(), BevyError> {
    let user_selected_file_path: String = todo!();

    asset_server.add_source(
        "user_directory",
        &mut AssetSourceBuilder::platform_default(&user_selected_file_path, None)
    )?;

    let wallpaper = asset_server.load("user_directory://wallpaper.png");
    commands.spawn(Sprite { image: wallpaper, ..Default::default() });

    Ok(())
}
```

Asset sources can also be removed at runtime, allowing you to load and unload asset sources as
necessary.

We've also changed the behavior of registering asset sources. Previously, you needed to register
asset sources **before** `DefaultPlugins` (more accurately, the `AssetPlugin`). This was uninuitive,
and resulted in limitations, like effectively preventing crate authors from registering their own
asset sources (since crate plugins often need to come after `DefaultPlugins`). Now, asset sources
need to be registered after `AssetPlugin` (and so, `DefaultPlugins`).

## Limitations

A limitation is that asset sources added after `Startup` cannot be **processed** asset sources. Attempting to add
such a source will return an error. Similarly, removing a processed source returns an error. In the
future, we hope to lift this limitation and allow runtime asset sources to be processed.
