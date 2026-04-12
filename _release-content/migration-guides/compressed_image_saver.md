---
title: "`CompressedImageSaver` improvements"
pull_requests: [23567]
---

The `compressed_image_saver` Cargo feature has been reworked. The old behavior (Basis Universal UASTC compression) has been moved to a new feature called `compressed_image_saver_universal`, and the `compressed_image_saver` feature now uses the `ctt` library to compress textures into BCn (desktop) or ASTC (mobile) formats instead.

If you were using the `compressed_image_saver` feature and want to keep the previous Basis Universal behavior, rename the feature in your `Cargo.toml`:

```toml
# Before
bevy = { version = "0.18", features = ["compressed_image_saver"] }

# After (keeps old Basis Universal behavior)
bevy = { version = "0.19", features = ["compressed_image_saver_universal"] }
```

Alternatively, keep using `compressed_image_saver` to get the new BCn/ASTC compression backend. This produces higher-quality output and supports a wider range of input formats, but does not support all platforms in a single file like UASTC does. We recommend sticking to `compressed_image_saver_universal` when targeting the web.

`CompressedImageSaverError` has a new variant `CompressionFailed`. If you were matching exhaustively on this enum, add a branch for it.
