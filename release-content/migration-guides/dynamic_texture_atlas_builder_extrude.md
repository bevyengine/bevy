---
title: "`DynamicTextureAtlasBuilder::new` extrude parameter"
pull_requests: [23132]
---

`DynamicTextureAtlasBuilder::new` has a new parameter `extrude: bool`. Set it to `false` for past behavior (transparent padding). If true, the source image's border pixels are extruded into its padding.
