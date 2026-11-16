---
title: "Exposed light written to the HDR main texture is capped"
pull_requests: [26211]
---

Bevy's shaders that write exposed light to a view's HDR main texture now scale their output down,
keeping its hue, so that no channel exceeds `HDR_CEILING` from `bevy_render::view`. Light many
stops brighter than the view's exposure is capped even at ordinary exposures. Layers blended on top
of each other can still add up past it. Custom shaders that write exposed light to the main texture
should pass it through `bevy_render::view::apply_hdr_ceiling`.
