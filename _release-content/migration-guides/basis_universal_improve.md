---
title: "Basis Universal update and improvement"
pull_requests: [23672]
---

Previously bevy used [basis-universal-rs](https://github.com/aclysma/basis-universal-rs) for basis universal support, including `.basis` and ktx2 UASTC texture
loading and `CompressedImageSaver`. However it doesn't support web and uses relatively outdated Basis Universal v1.16.

Now bevy uses [`basisu_c_sys`](https://docs.rs/basisu_c_sys/latest/basisu_c_sys) which is basis universal v2.10 and supports all the basis universal formats (ETC1S, UASTC, ASTC and XUASTC) and `wasm32-unknown-unknown` on web.

`ImageFormat::Basis` is removed. `CompressedImageSaver` is replaced by `BasisuSaver`/`BasisuProcessor` which is not added by `ImagePlugin` automatically. Also the `basis-universal` cargo feature is renamed to `basis_universal`, `compressed_image_saver` is replaced by `basis_universal_saver`.

If you are using `.basis` files, it's recommanded to re-compress your textures to `.ktx2` format with basisu tool. Basis universal textures will be handled as `ImageFormat::Ktx2` if `basis_universal` feature is enabled.

To use the `BasisuProcessor`, enable `basis_universal_saver` feature and add `BasisUniversalProcessorPlugin`:

```rs
use bevy::image::BasisUniversalProcessorPlugin;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(bevy::log::LogPlugin {
                    filter: "bevy_image=debug,bevy_asset=debug,wgpu=warn".to_string(),
                    ..Default::default()
                })
                .set(AssetPlugin {
                    mode: AssetMode::Processed,
                    ..Default::default()
                }),
            BasisUniversalProcessorPlugin::default(),
        ))
        .run();
}
```
