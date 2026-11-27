---
title: "Val calc"
pull_requests: []
---

`Val`, `MinTrackSizingFunction` and `MaxTrackSizingFunction` have a new `Calc` variant.

`Val::resolve` is no longer `const`.

`MeasureArgs`'s `style` field is now private, so it can no longer be constructed outside of `bevy_ui`. Use `MeasureArgs::style()` to read the style, and `MeasureArgs::resolve_padding` and `MeasureArgs::resolve_border` to resolve its padding and border.

`TaffyStyle` is no longer a tuple struct and no longer implements `Clone`, `Deref` and `DerefMut`.
