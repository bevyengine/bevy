---
title: World root components no longer require Visibility
pull_requests: [24963]
---

`WorldAssetRoot` and `DynamicWorldRoot` no longer automatically insert
`Visibility`. This removes the `bevy_world_serialization` dependency on
`bevy_camera`.

Users relying on this behavior should add `Visibility` explicitly:

```rust
commands.spawn((
    WorldAssetRoot(handle),
    Visibility::default(),
));
```
