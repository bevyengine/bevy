---
title: "`CompressedImageSaver` improvements"
pull_requests: [24223, 24904]
---

The `compressed_image_saver` Cargo feature has been reworked. The old behavior (Basis Universal UASTC compression) has been moved to a new feature called `compressed_image_saver_universal`, and the `compressed_image_saver` feature now uses the `ctt` library to compress textures into BCn (desktop) or ASTC (mobile) formats instead.

If you were using the `compressed_image_saver` feature and want to keep the previous Basis Universal behavior, rename the feature in your `Cargo.toml`:

```toml
# Before
bevy = { version = "0.19", features = ["compressed_image_saver"] }

# After (keeps old Basis Universal behavior)
bevy = { version = "0.20", features = ["compressed_image_saver_universal"] }
```

Alternatively, keep using `compressed_image_saver` to get the new BCn/ASTC compression backend. This produces higher-quality output and supports a wider range of input formats, but does not support all platforms in a single file like UASTC does. We recommend sticking to `compressed_image_saver_universal` when targeting the web.

`CompressedImageSaverError` has a new variant `CompressionFailed`. If you were matching exhaustively on this enum, add a branch for it.

In Bevy 0.19, `ImagePlugin` registered a default compressed image processor for PNG files. This meant PNG files were automatically compressed if asset processing was enabled, and the processor wasn't overridden by a `.meta` file. In Bevy 0.20, JPEG files have been added to the default processor. The extensions that the default processor uses can also be overridden by `ImagePlugin::default_compressed_image_processor_extensions` -  to revert back to the Bevy 0.19 PNG-only behavior:

```rust
App::new().add_plugins(
    DefaultPlugins.set(ImagePlugin {
        default_compressed_image_processor_extensions: ["png".into()].into(),
        ..Default::default()
    }),
)
```
