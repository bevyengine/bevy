---
title: Tilemap Chunk Rendering
authors: ["@ConnerPetzold", "@grind086", "@IceSentry"]
pull_requests: [18866]
---

A performant way to render tilemap chunks has been added as the first building block to Bevy's tilemap support. You can render a chunk by supplying a tileset texture to the `TilemapChunk` component and tile data to `TilemapChunkTileData`. For each tile, `TileData` allows you to specify the index into the tileset, the visibility, and the color tint.

```rust
let chunk_size = UVec2::splat(64);
let tile_display_size = UVec2::splat(16);
let tile_data: Vec<Option<TileData>> = (0..chunk_size.element_product())
    .map(|_| rng.gen_range(0..5))
    .map(|i| {
        if i == 0 {
            None
        } else {
            Some(TileData::from_index(i - 1))
        }
    })
    .collect();

commands.spawn((
    TilemapChunk {
        chunk_size,
        tile_display_size,
        tileset,
    },
    TilemapChunkTileData(tile_data),
));
```
