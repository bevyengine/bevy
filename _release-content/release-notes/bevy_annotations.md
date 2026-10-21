---
title: Reflection annotations
authors: ["@jbuehler23"]
pull_requests: [26164]
---

`bevy_reflect` already lets you attach custom attributes to types, fields and variants with `#[reflect(@...)]`.
The new `bevy_annotations` crate adds a shared vocabulary of those attributes, so tools like inspectors and editors know what to look for.

```rust
use bevy::annotations::{DisplayName, ValueRange, ReadOnly, Step, Tooltip, Unit};

#[derive(Component, Reflect)]
#[reflect(Component, @Tooltip("Movement settings for a character"))]
struct Movement {
    #[reflect(@ValueRange(0.0..=20.0), @Step(0.5), @Unit("m/s"))]
    speed: f32,
    #[reflect(@DisplayName("Jump Height"))]
    jump: f32,
    #[reflect(@ReadOnly)]
    grounded: bool,
}
```

Tools read them back with `get_attribute::<T>()` on the type, field or variant info.
`ValueRange::from_attributes` also accepts a plain `RangeInclusive` of any numeric type.
