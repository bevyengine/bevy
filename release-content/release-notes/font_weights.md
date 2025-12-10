---
title: "Font weight support"
authors: ["@ickshonpe"]
pull_requests: [22038]
---

Adds support for font weights.

`TextFont` now has a `weight: FontWeight` field. `FontWeight` newtypes a `u16`, values inside the range 1 and 1000 are valid. Values outside the range are clamped.
