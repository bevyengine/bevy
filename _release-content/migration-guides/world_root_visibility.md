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

Alternatively, applications can preserve the previous automatic insertion
behavior by manually registering `Visibility` as a required component:

```rust
app.register_required_components::<WorldAssetRoot, Visibility>();
app.register_required_components::<DynamicWorldRoot, Visibility>();
```

This registration must occur before either root component is first inserted
into the world.
