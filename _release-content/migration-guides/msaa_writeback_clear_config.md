---
title: "`MsaaWriteback` no longer runs if `Camera::clear_color` is not `ClearColorConfig::None`"
pull_requests: [25634]
---

Previously `MsaaWriteback` would run when the clear color was `ClearColorConfig::Default` or `ClearColorConfig::Custom`, which caused the camera's clear color to have no effect. Now `MsaaWriteback` never runs when the clear color is not `ClearColorConfig::None` because the texture is cleared so the writeback is useless. This brings the behavior in line with when MSAA is disabled.

If your code requires higher‑order cameras to overlay its result on top of lower‑order cameras and relied on the buggy behavior of `MsaaWriteback`, please use `ClearColorConfig::None` make cameras not clear previous rendering results.
