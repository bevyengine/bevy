---
title: "`FromType` replaced by `CreateTypeData`"
pull_requests: [13723]
---

`FromType<T>` has been replaced by `CreateTypeData<T, Input = ()>`.
This was done to better communicate what the trait was for (i.e. creating type data),
as well as make it possible to pass in additional input when registering type data.

Implementors of `FromType<T>` will need to update their implementation:

```rust
// BEFORE
impl<T> FromType<T> for ReflectMyTrait {
  fn from_type() -> Self {
    // ...
  }
}

// AFTER
impl<T> CreateTypeData<T> for ReflectMyTrait {
  fn create_type_data(input: ()) -> Self {
    // ...
  }
}
```

Additionally, any calls made to `FromType::from_type` will need to be updated as well:

```rust
// BEFORE
<ReflectMyTrait as FromType<Foo>>::from_type()

// AFTER
<ReflectMyTrait as CreateTypeData<Foo>>::create_type_data(())
```
