---
title: "`Text2d` window independent scale factor support"
authors: ["@Icksonpe"]
pull_requests: [20656]
---

In previous versions of bevy, text rendered using `Text2d` would always use the scale factor of the primary window regardless of its render target. In 0.17, `Text2d` glyphs are rasterized to match the scale factor of the render target where the text is to be drawn.

`Text2d` is still limited to generating only one text layout per `Text2d` entity. If a `Text2d` entity is simultaneously rendered to multiple targets with different scale factors then the maximum of the target scale factors is used.
