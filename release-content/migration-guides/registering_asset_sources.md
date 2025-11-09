---
title: Registering asset sources
pull_requests: []
---

In previous versions, asset sources had to be registered **before** adding the `AssetPlugin` (which
usually meant before adding the `DefaultPlugins`). Now, asset sources must be registered **after**
`AssetPlugin` (and in effect, after `DefaultPlugins`). So if you had:

```rust
App::new()
    .register_asset_source("my_source",
        AssetSource::build()
            .with_reader(move || Box::new(todo!()))
    )
    .add_plugins(DefaultPlugins)
    .run();
```

Now, it will be:

```rust
App::new()
    .add_plugins(DefaultPlugins)
    .register_asset_source("my_source",
        AssetSourceBuilder::new(move || Box::new(todo!()))
    )
    .run();
```

Note: See also "Custom asset sources now require a reader" for changes to builders.

In addition, default asset sources **can no longer be registered like custom sources**. There are
two cases here:

## 1. File paths

The `AssetPlugin` will create the default asset source for a pair of file paths. Previously, this
was written as:

```rust
App::new()
    .add_plugins(DefaultPlugins.set(
        AssetPlugin {
            file_path: "some/path".to_string(),
            processed_file_path: "some/processed_path".to_string(),
            ..Default::default()
        }
    ));
```

Now, this is written as:

```rust
App::new()
    .add_plugins(DefaultPlugins.set(
        AssetPlugin {
            default_source: DefaultAssetSource::FromPaths {
                file_path: "some/path".to_string(),
                // Note: Setting this to None will just use the default path.
                processed_file_path: Some("some/processed_path".to_string()),
            },
            ..Default::default()
        }
    ));
```

## 2. Custom default source

Users can also completely replace the default asset source to provide their own implementation.
Previously, this was written as:

```rust
App::new()
    .register_asset_source(
        AssetSourceId::Default,
        AssetSource::build()
            .with_reader(move || Box::new(todo!()))
    )
    .add_plugins(DefaultPlugins);
```

Now, this is written as:

```rust
App::new()
    .add_plugins(DefaultPlugins.set(
        AssetPlugin {
            default_source: DefaultAssetSource::FromBuilder(Mutex::new(
                AssetSourceBuilder::new(move || Box::new(todo!()))
            )),
            ..Default::default()
        }
    ));
```
