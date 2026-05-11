---
title: Asset Saving
authors: ["@andriyDev"]
pull_requests: [22622]
---

Bevy has had an `AssetSaver` trait since 0.12.
However, it was only ever intended for use inside asset processing pipelines, not for saving assets at runtime.
This left a frustrating gap: if you wanted to save a procedurally generated mesh, a baked lightmap, or the output of an in-editor workflow, there was no supported path to do it.

Now there is. `save_using_saver` lets you save any asset to disk using an `AssetSaver` implementation of your choice.

## 1. Building the `SavedAsset`

For simple assets with no sub-assets, use `SavedAsset::from_asset`:

```rust
let main_asset = InlinedBook {
    lines: vec!["Save me!".to_string(), "Please!".to_string()],
};
let saved_asset = SavedAsset::from_asset(&main_asset);
```

For assets that reference other assets (sub-assets), use `SavedAssetBuilder`:

```rust
let asset_path: AssetPath<'static> = "my/file/path.whatever".into();
let mut builder = SavedAssetBuilder::new(asset_server.clone(), asset_path.clone());

let subasset_1 = Line("howdy".into());
let subasset_2 = Line("goodbye".into());
let handle_1 = builder.add_labeled_asset_with_new_handle(
    "TheFirstLabel", SavedAsset::from_asset(&subasset_1));
let handle_2 = builder.add_labeled_asset_with_new_handle(
    "AnotherOne", SavedAsset::from_asset(&subasset_2));

let main_asset = Book {
    lines: vec![handle_1, handle_2],
};
let saved_asset = builder.build(&main_asset);
```

`SavedAsset` borrows rather than owns its assets.
That means you can build and save in the same async block — no need to transfer ownership first.

## 2. Calling `save_using_saver`

```rust
save_using_saver(
    asset_server.clone(),
    &MyAssetSaver::default(),
    &asset_path,
    saved_asset,
    &MySettings::default(),
).await.unwrap();
```

`save_using_saver` is async.
Generally, you'll want to spawn it with `IoTaskPool::get().spawn(...)`.
You'll also need to implement `AssetSaver` for `MyAssetSaver` to define the serialization format.
