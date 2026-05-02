---
title: "One-shot systems are now registered as `SystemArc`s rather than `BoxedSystem`s"
pull_requests: [24072]
---

In order to support `bsn!` templating of `SystemId`s, one-shot systems are now
stored as `SystemArc`s rather than `BoxedSystem`s.

- `World::register_boxed_system` was replaced with `World::register_system_arc`.
- Rather than storing `Box<dyn System>`s and passing them to `register_boxed_system`,
  store `SystemArc<dyn System>`s and pass them to `register_system_arc`.

```rust
struct Foo;
struct Bar;

// 0.18
let my_boxed_system: Box<dyn System<In = In<Foo>, Out = Bar>> =
    Box::new(IntoSystem::into_system(
        |foo| { Bar }
    ));

world.register_boxed_system(my_boxed_system);

// 0.19
let my_system_arc: SystemArc<dyn System<In = In<Foo>, Out = Bar>> =
    SystemArc::new_dyn(|foo| { Bar });

world.register_system_arc(my_system_arc);
```
