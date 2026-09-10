---
title: "`define_label!` no longer defines an `Interner`"
pull_requests: [24445]
---

The macro `define_label!()` no longer takes a parameter for the name of an `Interner`,
and that interner is no longer a public `static` item. Calls like

```rust
bevy::ecs::define_label!(
    /// Documentation
    ThingLabel,
    THING_LABEL_INTERNER
);
```

must be changed to

```rust
bevy::ecs::define_label!(
    /// Documentation
    ThingLabel,
);
```

If you were calling `Interner::intern()` on the defined interner, then replace those calls
with calls to the `.intern()` method of the defined label trait.
