---
title: Different defaults for `Tonemapping` based on `tonemapping_luts` feature
pull_requests: [20924]
---

`Tonemapping` component now has a different defaults based on `tonemappint_luts` feature.
When `tonemapping_luts` is present the default remains `TonyMcMapface`, but when it is off
the default is now `None`.
