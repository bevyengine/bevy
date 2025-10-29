---
title: Tilemap Chunk Layout
pull_requests: [21684]
---

Tiles in a chunk are stored in a `Vec`, meaning the 2d coordinates of a tile need to be mapped into an index.

Previously the mapping would put the origin point for a chunks tile layout in the top left of the chunk.  This doesn't align 
align with bevy's world coordinate system, so when mapping to and from chunk space (to map a world coordinate to a tile) you
would have to account for the chunk y coordinate being inverted.

The origin for a chunk has now been changed to the bottom left of the chunk, meaning you can simply mod world coordinates to get chunk coordinates.

Some other tiling tools have the convention of the origin being at the top left, but it's more important for Bevy's features
to be internally consistant as it allows for better ease of use.