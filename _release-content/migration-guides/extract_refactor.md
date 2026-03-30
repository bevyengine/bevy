---
title: "`ExtractComponent` refactor"
pull_requests: [22766, 23334]
---

Previously, `SyncComponentPlugin`/`ExtractComponentPlugin` would despawn the render entity thus removing all the derived components if the component was removed. Now the render entity is no longer despawned and only the `Target` components of `SyncComponent` trait are removed.

`SyncComponent` is a subtrait of `ExtractComponent` and you must implement it to clean up extracted and derived components.

```rust,ignore
impl SyncComponent for MyComponent {
    type Target = (Self, OtherDerivedComponents);
}

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

You can also specify the sync target (default to `Self`) using `extract_component_sync_target` attribute in derive macros.

```rust,ignore
#[derive(Component, ExtractComponent)]
#[extract_component_sync_target((Self, OtherDerivedComponents))]
struct MyComponent;
```

Both `SyncComponent` and `ExtractComponent` have also gotten an optional marker type that can be used to bypass orphan rules, see the docs for details.
