---
title: "`System::initialize` takes a `SystemInitContext`"
pull_requests: []
---

`System::initialize` has a new `context: &SystemInitContext` parameter.
A `Schedule` uses it to tell each system which system sets it belongs to, which system params can read from `SystemMeta::system_sets` once the system is initialized.

If you call `System::initialize` yourself, pass `&SystemInitContext::default()`:

```rust
// Before
system.initialize(&mut world);

// After
system.initialize(&mut world, &SystemInitContext::default());
```

If you implement `System`, add the parameter and forward it to any systems you wrap:

```rust
// Before
fn initialize(&mut self, world: &mut World) -> SystemAccess {
    self.inner.initialize(world)
}

// After
fn initialize(&mut self, world: &mut World, context: &SystemInitContext) -> SystemAccess {
    self.inner.initialize(world, context)
}
```
