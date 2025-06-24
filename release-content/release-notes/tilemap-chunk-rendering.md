---
title: Tilemap Chunk Rendering
authors: ["@ConnerPetzold", "@grind086", "@IceSentry"]
pull_requests: [18866]
---

A performant way to render tilemap chunks has been added as the first building block to Bevy's tilemap support. You can render a chunk by supplying a tileset texture to the `TilemapChunk` component and the indices into that tileset for each tile to `TilemapChunkIndices`.

```rust
let chunk_size = UVec2::splat(64);
let tile_size = UVec2::splat(16);
let indices: Vec<Option<u32>> = (0..chunk_size.x * chunk_size.y)
    .map(|_| rng.gen_range(0..5))
    .map(|i| if i == 0 { None } else { Some(i - 1) })
    .collect();

commands.spawn((
    TilemapChunk {
        chunk_size,
        tile_size,
        tileset,
    },
    TilemapChunkIndices(indices),
));
```
