---
title: Call-style component hooks can use `Self` and generic parameters
pull_requests: [26608]
---

`#[derive(Component)]` accepts two kinds of hook in `#[component(on_add = ...)]` and its `on_insert`, `on_discard`, `on_remove` and `on_despawn` siblings: a path to a function, or a call to a function that returns the hook as a closure.

Previously, the call form could not mention `Self` or the generic parameters of the component, failing with "can't use generic parameters from outer item". It now can:

```rust
fn log_type<T: 'static>() -> impl Fn(DeferredWorld, HookContext) {
    |_world, _ctx| println!("{}", core::any::type_name::<T>())
}

#[derive(Component)]
#[component(on_add = log_type::<T>(), on_insert = log_type::<Self>())]
struct Wrapper<T: Send + Sync + 'static>(T);
```

Call-style `on_insert`, `on_discard` and `on_despawn` hooks can now also be combined with `#[relationship]` and `#[relationship_target]`, which previously failed to compile.

No existing code needs to change. If you worked around these limitations, for example by implementing `Component` by hand, by registering the hook at runtime, or by writing a separate non-generic hook function for each concrete type, you can now use the attribute directly.
