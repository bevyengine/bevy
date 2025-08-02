---
title: Non-generic `Access`
pull_requests: [TODO]
---

Now that `archetype_component_id` has been removed,
`Access`, `AccessFilters`, `FilteredAccess`, and `FilteredAccessSet`
were only ever parameterized by `ComponentId`.
To simplify use of those types, the generic parameter has been removed.
Remove the `<Component>` generic from any use of those types.

```rust
// 0.16
fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>) {}
// 0.17
fn update_component_access(state: &Self::State, access: &mut FilteredAccess) {}
```
