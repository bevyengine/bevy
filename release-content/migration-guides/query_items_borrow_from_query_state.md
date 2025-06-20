---
title: Query items can borrow from query state
pull_requests: [15396, 19720]
---

The `QueryData::Item` associated type and the `QueryItem` and `ROQueryItem` type aliases now have an additional lifetime parameter corresponding to the `'s` lifetime in `Query`.
The `QueryData::fetch()` and `QueryFilter::filter_fetch()` methods have a new parameter taking a `&'s WorldQuery::State`.
Manual implementations of `WorldQuery` and `QueryData` will need to update the method signatures to include the new lifetimes.
Other uses of the types will need to be updated to include a lifetime parameter, although it can usually be passed as `'_`.
In particular, `ROQueryItem` is used when implementing `RenderCommand`.

Before:

```rust
// 0.16
fn render<'w>(
    item: &P,
    view: ROQueryItem<'w, Self::ViewQuery>,
    entity: Option<ROQueryItem<'w, Self::ItemQuery>>,
    param: SystemParamItem<'w, '_, Self::Param>,
    pass: &mut TrackedRenderPass<'w>,
) -> RenderCommandResult;

// 0.17
fn render<'w>(
    item: &P,
    view: ROQueryItem<'w, '_, Self::ViewQuery>,
    entity: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
    param: SystemParamItem<'w, '_, Self::Param>,
    pass: &mut TrackedRenderPass<'w>,
) -> RenderCommandResult;
```

---

Methods on `QueryState` that take `&mut self` may now result in conflicting borrows if the query items capture the lifetime of the mutable reference.
This affects `get()`, `iter()`, and others.
To fix the errors, first call `QueryState::update_archetypes()`, and then replace a call `state.foo(world, param)` with `state.query_manual(world).foo_inner(param)`.
Alternately, you may be able to restructure the code to call `state.query(world)` once and then make multiple calls using the `Query`.

```rust
let mut state: QueryState<_, _> = ...;

// 0.16
let d1 = state.get(world, e1);
let d2 = state.get(world, e2); // Error: cannot borrow `state` as mutable more than once at a time

println!("{d1:?}");
println!("{d2:?}");

// 0.17
state.update_archetypes(world);
let d1 = state.get_manual(world, e1);
let d2 = state.get_manual(world, e2);
// OR
state.update_archetypes(world);
let d1 = state.query_manual(world).get_inner(e1);
let d2 = state.query_manual(world).get_inner(e2);
// OR
let query = state.query(world);
let d1 = query.get_inner(e1);
let d1 = query.get_inner(e2);

println!("{d1:?}");
println!("{d2:?}");
```
