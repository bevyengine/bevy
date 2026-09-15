---
title: Load Radiance HDR files as cubemaps
authors: ["@stuartparmenter"]
pull_requests: [25802]
---

Environment maps often come as lat-long panoramas in Radiance HDR (`.hdr`) files, while Bevy's skyboxes and environment lighting use cubemaps. Bevy can now convert these panoramas during loading, removing the need for an external conversion tool.

For example, with asset processing and `compressed_image_saver` enabled, add an `environment.hdr.meta` file beside your panorama to convert it to a compressed cubemap for rendering:

```ron
(
    meta_format_version: "1.0",
    asset: Process(
        processor: "LoadTransformAndSave<HdrTextureLoader, IdentityAssetTransformer<Image>, CompressedImageSaver>",
        settings: (
            loader_settings: (
                asset_usage: RenderAssetUsages("RENDER_WORLD"),
                // Convert the panorama to six 2048×2048 faces.
                cubemap_face_size: Some(2048),
            ),
            transformer_settings: (),
            // Use default compression and generate mipmaps.
            saver_settings: (),
        ),
    ),
)
```

Then `asset_server.load("environment.hdr")` loads the compressed cubemap, ready to use as a skybox or to filter for environment lighting. Conversion is also available during ordinary asset loading, without compression.
