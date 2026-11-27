---
title: "Val calc"
pull_requests: []
---

`Val`, `MinTrackSizingFunction` and `MaxTrackSizingFunction` have a new `Calc` variant.

`Val::resolve` is no longer `const`.

`MeasureArgs` is now `#[non_exhaustive]` and its `style` field is private. Use `MeasureArgs::style()` to read the style, and `MeasureArgs::resolve_padding` and `MeasureArgs::resolve_border` to resolve its padding and border.

`TaffyStyle` is no longer a tuple struct and no longer implements `Clone`, `Deref` and `DerefMut`.
