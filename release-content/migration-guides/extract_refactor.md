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
