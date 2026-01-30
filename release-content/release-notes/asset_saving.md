---
title: Asset Saving
authors: ["@andriyDev"]
pull_requests: []
---

Since Bevy 0.12, we've had the `AssetSaver` trait. Unfortunately, this trait was not really usable
for asset saving: it was only intended for use with asset processing! This was a common stumbling
block for users, and pointed to a gap in our API.

Now, users can save their assets using `save_using_saver`. To use this involves two steps.

## 1. Building the `SavedAsset`

To build the `SavedAsset`, either use `SavedAsset::from_asset`, or `SavedAssetBuilder`. For example:

```rust
let asset_path: AssetPath<'static> = "my/file/path.whatever";
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

Note that since these assets are borrowed, building the `SavedAsset` should happen in the same async
task as the next step.

## 2. Calling `save_using_saver`

Now, with a `SavedAsset`, we can just call `save_using_saver` and fill in any arguments:

```rust
save_using_saver(
    asset_server.clone(),
    &MyAssetSaver::default(),
    &asset_path,
    saved_asset,
    &MySettings::default(),
).await.unwrap();
```

Part of this includes implementing the `AssetSaver` trait on `MyAssetSaver`. In addition, this is an
async function, so it is likely you will want to spawn this using `IoTaskPool::get().spawn(...)`.
