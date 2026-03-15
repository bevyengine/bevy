---
title: "texture atlas builder padding improvements"
authors: ["@Wuketuke", "@andriyDev", "@ickshonpe"]
pull_requests: [23091, 23056, 23074, 23132]
---

`DynamicTextureAtlasBuilder` now supports edge extrusion. With extrusion enabled, border pixels of the each texture in the atlas are duplicated (extruded) outwards into the padding area. This helps prevent artifacts when colors are blended across the edge of a texture in an atlas.

To enable edge extrusion, call `DynamicTextureAtlasBuilder::new` with its `extrude` parameter set to true.

`DynamicTextureAtlasBuilder` and `TextureAtlasBuilder` now include padding on the top left of each texture atlas to mitigate rendering artifacts with textures placed into the atlas against the top or left edges.

`bevy_text`'s font atlases now use a two pixel padding around each glyph with the texture extruded.

Two new scenes have been added to `testbed_2d` that demonstrate these changes:

```bash
cargo run --example testbed_2d -- DynamicTextureAtlasBuilder
```

```bash
cargo run --example testbed_2d -- TextureAtlasBuilder
```
