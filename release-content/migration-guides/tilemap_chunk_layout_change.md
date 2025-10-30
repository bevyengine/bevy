---
title: Tilemap Chunk Layout
pull_requests: [21684]
---

`TilemapChunk` and `TilemapChunkTileData`'s default layout has been changed from the origin being in the top left to the origin being in the bottom left.

The previous layout origin didn't align with Bevy's world coordinate system, so when mapping to and from chunk space (to map a world coordinate to a tile) you would have to account for the chunk y coordinate being inverted.

With the origin of the chunk being in the bottom left, you can simply mod world coordinates to get chunk coordinates.

Some other tiling tools have the convention of the origin being at the top left, but it's more important for Bevy's features
to be internally consistent as it allows for better ease of use.
