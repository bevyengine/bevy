---
title: Short-type-path asset processors
authors: ["@andriyDev"]
pull_requests: [21339]
---

Asset processors allow manipulating assets at "publish-time" to convert them into a more optimal
form when loading the data at runtime. This can either be done using a default processor, which
processes all assets with a particular file extension, or by specifying the processor in the asset's
meta file.

In previous versions of Bevy, the processor had to be **fully** specified in the asset's meta file.
For example:

```ron
(
    meta_format_version: "1.0",
    asset: Process(
        processor: "bevy_asset::processor::process::LoadTransformAndSave<asset_processing::CoolTextLoader, asset_processing::CoolTextTransformer, asset_processing::CoolTextSaver>",
        settings: (
            loader_settings: (),
            transformer_settings: (),
            saver_settings: (),
        ),
    ),
)
```

As you can see, processor types can be very verbose! In order to make these meta files easier to
manipulate, we now also support using the "short type path" of the asset. This would look like:

```ron
(
    meta_format_version: "1.0",
    asset: Process(
        processor: "LoadTransformAndSave<CoolTextLoader, CoolTextTransformer, CoolTextSaver>",
        settings: (
            loader_settings: (),
            transformer_settings: (),
            saver_settings: (),
        ),
    ),
)
```
