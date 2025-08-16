---
title: "`Val` helper functions"
authors: ["@Ickshonpe"]
pull_requests: [20518]
---

To make `Val`s easier to construct the following helper functions have been added: `px`, `percent`, `vw`, `vh`, `vmin` and `vmax`. Each function takes an `f32` value and returns value wrapped by its corresponding `Val` variant. There is also an `auto` helper function that maps to `Val::Auto`.
