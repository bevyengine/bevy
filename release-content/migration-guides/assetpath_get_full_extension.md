---
title: `AssetPath::get_full_extension` returns `&str`
pull_requests: [19974]
---

`AssetPath::get_full_extension` now works on non-UTF-8 file names and returns `&str` instead of `String`.
