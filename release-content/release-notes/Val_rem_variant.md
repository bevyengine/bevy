---
title: "`rem` units support"
authors: ["@Ickshonpe"]
pull_requests: [21187]
---
The UI now supports `rem` units. `Val::Rem` values are relative to the root UI Node's fontsize, or the default font size, if none is set.

Includes a helper function `rem(val: f32)`.
