---
title: LoadTransformAndSave has been replaced by make_load_transform_and_save_processor.
pull_requests: []
---

In previous versions of Bevy, the recommended approach for creating asset processors was using
`LoadTransformAndSave` (as opposed to the lower-level `Process` trait). Unfortunately, due to how
many generics were included in `LoadTransformAndSave`, it was quite cumbersome to refer to
processors from meta files (for example, meta files needed to reference types like
`"LoadTransformAndSave<ImageLoader, DoSomethingToImageTransformer, CompressedImageSaver>"`). Even
registering asset processors was cumbersome, since it involved repeating the type name twice: once
to create the processor to register, and once to set that processor as the default for a file
extension.

`LoadTransformAndSave` is now deprecated in favor of the `make_load_transform_and_save_processor`
macro. This creates a *brand-new type* that performs the requested process. This means it's easier
to reference processors from meta files, and it's easier to register those processors.

To migrate an existing `LoadTransformAndSave` like:

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            mode: AssetMode::Processed,
            ..Default::default()
        }))
        .register_asset_processor::<LoadTransformAndSave<
            ImageLoader,
            DoSomethingToImageTransformer,
            CompressedImageSaver,
        >>(
            LoadTransformAndSave::new(
                DoSomethingToImageTransformer,
                CompressedImageSaver,
            )
        )
        .set_default_processor::<LoadTransformAndSave<
            ImageLoader,
            DoSomethingToImageTransformer,
            CompressedImageSaver,
        >>("png")
        .run();
}
```

This can be replaced with:

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            mode: AssetMode::Processed,
            ..Default::default()
        }))
        .register_asset_processor(
            DoSomethingToImageProcessor::new(
                DoSomethingToImageTransformer,
                CompressedImageSaver,
            )
        )
        .set_default_processor::<DoSomethingToImageProcessor>("png")
        .run();
}

make_load_transform_and_save_processor!(
    struct DoSomethingToImageProcessor {
        loader: ImageLoader,
        transformer: DoSomethingToImageTransformer,
        saver: CompressedImageSaver,
    }

    struct DoSomethingToImageProcessorSettings { .. }
)
```

For the special case where you don't need a transformer, this field can be omitted.

Keep in mind any existing meta files (in your `assets` directory) may still refer to the old
`LoadTransformAndSave<...>` processor. These meta files can be edited manually to replace
`LoadTransformAndSave<...> with the name of your new processor. **Do not edit the meta files in your
`imported_assets` directory** - these should be automatically recomputed once you update the meta
files in your `assets` directory.
