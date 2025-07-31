---
title: `BorderColor::all` now accepts any `impl Into<Color>` type
pull_requests: [20311]
---

`BorderColor`'s `all` constructor function is no longer const and it's `color` parameter now accepts any `impl Into<Color>` type, not only `Color`.
