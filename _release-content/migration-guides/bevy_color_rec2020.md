---
title: "New `Color::LinearRec2020` variant in `bevy_color`"
pull_requests: [25373]
---

`Color` has a new variant, `Color::LinearRec2020(LinearRec2020)` which represents linear RGB
with wide-gamut Rec.2020 primaries. Make sure to add it to your exhaustive `match`
statements. Conversions such as `From`/`Into`, `Color::to_linear`, `Color::to_srgba` keep working
like every other space.
