---
title: "Type data creation and registration changes"
pull_requests: [24518]
---

Type data definition and registration have been reworked to enable `on_insert` and `on_register` registration callbacks.

Type data is no longer automatically implemented on types implementing `Clone`.
Instead, it must be manually implemented or derived:

```rust
// BEFORE
#[derive(Clone)]
struct ReflectSomeTypeData;

// AFTER
#[derive(TypeData)]
struct ReflectSomeTypeData;

// or manually:
// impl TypeData for ReflectSomeTypeData {}
```

Additionally, type data can no longer be inserted directly onto a `&mut TypeRegistration`.
Instead, it must be inserted during construction, with an owned `TypeRegistration`.
This was done to ensure that registration callbacks weren't accidentally missed.

```rust
// BEFORE
let mut registration = TypeRegistration::of::<MyType>();
registration.register_type_data::<SomeTypeData, _>();

// AFTER
let registration = TypeRegistration::of::<MyType>()
  .register_type_data::<SomeTypeData, _>();
});
```

Methods for registering type data on the `TypeRegistry` remain unchanged.
However, because of the new restrictions around inserting type data on `TypeRegistration`,
instances of `TypeRegistry::get_mut` can be replaced with the new `TypeRegistry::registration_scope`
in order to insert multiple type data without additional lookups.

```rust
// BEFORE
let registration = registry.get_mut(TypeId::of::<MyType>());
registration.register_type_data::<SomeTypeData, _>();

// AFTER
let registration = registry.registration_scope(TypeId::of::<MyType>(), |mut registration| {
  registration.register_type_data::<SomeTypeData, _>();
});
```
