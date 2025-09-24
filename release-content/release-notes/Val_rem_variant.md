---
title: "`rem` units support"
authors: ["@Ickshonpe"]
pull_requests: [21187]
---
Adds a `Rem` variant to `Val`. `Val::Rem` values are based on the root UI Node's `TextFont` component's `font_size` value, or, if not present, the size of the default font. `Val::Rem` is a scalar value, not a percentage.

Includes a helper constructor function `rem` that takes a numerical value and returns a `Val::Rem`.