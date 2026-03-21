---
title: Asset sources no longer contain "processed" versions of its fields.
pull_requests: []
---

In previous versions of Bevy, asset sources contained two "parts". The "regular" part (i.e., the
unprocessed reader, writer, watcher, etc) and the "processed" part (i.e., the processed reader,
writer, watcher, etc). For many sources, these were actually duplicated. For example, the "web"
asset source set its `reader` and `processed_reader` to the same value.

Now, asset sources contain only a single part. There's now only one reader, writer, watcher, etc.
Processed sources now need to explicitly specify that they intend to be processed. Processed sources
also create their processed reader, writer, watcher, etc, automatically!

**For users who don't use asset processing:** nothing changes! Presumably your asset sources already
don't include any of the "processed" versions, and registering unprocessed sources remain the same.
You should be able to ignore this migration guide (though we do recommend users try using asset
processing).

**For users who do use asset processing:** do with the following.

1. For asset sources that should be processed, replace `app.register_asset_source(...)` with
   `app.register_processed_asset_source(...)`.
2. Remove any instances of:
   1. `source_builder.with_processed_reader(...)`
   2. `source_builder.with_processed_writer(...)`
   3. `source_builder.with_processed_watcher(...)`
   4. `source_builder.with_processed_watch_warning(...)`

   The processing system now automatically creates these readers for you (as regular file readers
   into the `imported_assets` folder, or whatever folder is set as the processed path).

   **For advanced users:** you may override the automatically created processed source using
   `set_processed_asset_source_for_unprocessed_source`. Consider whether you actually need this
   though. It may be more efficient to use the automatic source during dev, and then use a
   completely different "bundled" source when publishing (where you would just use
   `register_asset_source` directly). Come talk to us in the Bevy Discord `#assets-dev` to help us
   understand your use case!

3. If you do not need the default asset source to be processed, set `AssetPlugin::mode` to
   `AssetMode::Unprocessed` in the `AssetPlugin`.

So for example, if your app looks like the following:

```rust
fn main() {
    App::new()
        .register_asset_source(
            AssetSourceBuilder::new(|| Box::new(FileAssetReader::new("some/unprocessed/path")))
                .with_watcher(|sender| Box::new(FileWatcher::new(
                        Path::new("some/unprocessed/path").into(),
                        sender,
                        Duration::from_secs(1.0),
                    ).unwrap()))
                .with_processed_reader(|| Box::new(FileAssetReader::new("the/processed/path")))
                .with_processed_writer(|create_root| Box::new(FileAssetWriter::new(
                        "the/processed/path",
                        create_root,
                    )))
                .with_processed_watcher(|sender| Box::new(FileWatcher::new(
                        Path::new("the/processed/path").into(),
                        sender,
                        Duration::from_secs(1.0),
                    ).unwrap()))
        )
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            mode: AssetMode::Processed,
            ..Default::default()
        }))
        .run();
}
```

Now, it would look like:

```rust
fn main() {
    App::new()
        .register_processed_asset_source(
            AssetSourceBuilder::new(|| Box::new(FileAssetReader::new("some/unprocessed/path")))
                .with_watcher(|sender| Box::new(FileWatcher::new(
                        Path::new("some/unprocessed/path").into(),
                        sender,
                        Duration::from_secs(1.0),
                    ).unwrap()))
        )
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            // If you don't want the default source to be processed, set this to
            // `AssetMode::Unprocessed` (which is the default for `AssetPlugin`).
            mode: AssetMode::Processed,
            ..Default::default()
        }))
        .run();
}
```

As an added consequence of this, `AssetServer::write_default_loader_meta_file_for_path` no longer
works for processed asset sources. Instead, you can use
`AssetProcessor::write_default_meta_file_for_path` for these asset sources.
