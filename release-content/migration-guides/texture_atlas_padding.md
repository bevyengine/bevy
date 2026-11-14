---
title: Texture Atlas Padding
pull_requests: [23056]
---

The `TextureAtlasBuilder` now has some padding along its left and top edge.
The `padding` function has been changed, so it not only sets the padding between textures, but also this initial padding.

In case the padding should be changed separately, the `initial_padding` only changes the padding on the top and left edge. the padding on the top and left edge.
