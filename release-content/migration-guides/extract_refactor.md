---
title: "`ExtractComponent` refactor"
pull_requests: [22766]
---

The `Out` type from `ExtractComponent` has been split into a separate `SyncComponent` trait.

Both traits have also gotten an optional marker type that can be used to bypass orphan rules, see the docs for details.

```rust,ignore
impl ExtractComponent for MyComponent {
    type QueryData = ();
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(
        item: QueryItem<'_, '_, Self::QueryData>,
    ) -> Option<Self::Out> {
        Some(*item)
    }
}
```

After:

```rust,ignore
impl SyncComponent for MyComponent {
    type Out = Self;
}

impl ExtractComponent for MyComponent {
    type QueryData = ();
    type QueryFilter = ();

    fn extract_component(
        item: QueryItem<'_, '_, Self::QueryData>,
    ) -> Option<Self::Out> {
        Some(*item)
    }
}
```

All above has moved to new crate `bevy_extract`.

`ExtractPlugin` is now generic on `AppLabel`

Currently a component can only be extracted to a single world.

Most extraction parts are re-exported by `bevy_render` , but the following migrations are needed:

- When using traits, specify the `AppLabel`, e.g. `SyncComponent`, `ExtractComponent`

Before:

```rust,ignore
impl SyncComponent for TemporalAntiAliasing {
```

After:

```rust,ignore
impl SyncComponent<RenderApp> for TemporalAntiAliasing {
```

- Use `TemporarySubEntity` instead of `TemporaryRenderEntity`
