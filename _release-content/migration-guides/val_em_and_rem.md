---
title: "Val::Em and Val::Rem"
pull_requests: [25231]
---

`Val` has two new variants, `Val::Em` and `Val::Rem`, which size a length relative to a font size.
`Val::Em` resolves against the font size of the node it is set on, `Val::Rem` against the
`RemSize` resource. The `em` and `rem` helper functions construct them, alongside the existing `px`,
`percent`, `vw` and `vh`.

Resolving a `Val` now needs both of those font sizes, so the following methods take two additional
arguments, `em_size: EmSize` and `rem_size: RemSize`:

- `Val::resolve`
- `Val2::resolve`
- `UiPosition::resolve`
- `CornerRadius::resolve`
- `RadialGradientShape::resolve`
- `UiTransform::compute_affine`
- `BorderRadius::resolve`

```rust
// 0.19
let physical = val.resolve(scale_factor, physical_base_value, physical_target_size)?;

// 0.20
let physical = val.resolve(
    scale_factor,
    physical_base_value,
    physical_target_size,
    em_size,
    rem_size,
)?;
```

`ComputedNode` has new `em_size` and `rem_size` fields holding the values that were used to lay the
node out, so when resolving a `Val` against an existing node you can take them from there (`box_shadow`
for example).

`Node` now requires `EmSize` (from `bevy_text`, re-exported in `bevy_ui::prelude`), the per-node
font size that `Val::Em` resolves against. If the node has a `TextFont`, `EmSize` is recomputed when `TextFont`,
`RemSize`, or render-target info changes; values you set persist until then. If the node does not
have a `TextFont` component then the value is yours to set and is left alone; it defaults to
`DEFAULT_REM_SIZE_PX`, which matches the default `RemSize` but does not track changes to it. Propagating
`EmSize` is the responsibility of an app, not `bevy_ui`.

Use `GridTrack::em`, `GridTrack::rem`, `RepeatedGridTrack::em` and `RepeatedGridTrack::rem` to construct
grid tracks sized in these units using new `MinTrackSizingFunction::Em`, `MinTrackSizingFunction::Rem`,
`MaxTrackSizingFunction::Em` and `MaxTrackSizingFunction::Rem` variants.

`FontSize::eval` now takes a `RemSize` rather than an `f32`:

```rust
// 0.19
let size = font_size.eval(logical_viewport_size, rem_size_px);

// 0.20
let size = font_size.eval(logical_viewport_size, RemSize(rem_size_px));
```
