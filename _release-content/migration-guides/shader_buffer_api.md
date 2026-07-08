---
title: "`ShaderBuffer` API changes"
pull_requests: [24915]
---

1. `ShaderBuffer::new` now accepts `Vec<T>` where `T: bytemuck::NoUninit` instead of `&[u8]`. Use `[u8]::to_vec` to create a `Vec` if you want.
2. `ShaderBuffer::from` now accepts `Vec<T>` where `T: bytemuck::NoUninit`. `From<T: ShaderType + WriteInto>` is replaced by `ShaderBuffer::from_value`.
3. `ShaderBuffer::set_data` now accepts `impl Iterator<Item = T>` instead of a single `T` value. Use `core::iter::once` for single value. For iterators that were previously collected into a `Vec`, pass the iterator directly.

```rust
// BEFORE
ShaderBuffer::new(my_slice, RenderAssetUsages::default());
ShaderBuffer::from(my_shader_type_value);
buffer.set_data(my_value);
buffer.set_data(my_iterator.collect::<Vec<_>>());

// AFTER
ShaderBuffer::new(my_slice.to_vec(), RenderAssetUsages::default());
ShaderBuffer::from_value(my_shader_type_value);
buffer.set_data(core::iter::once(my_value));
buffer.set_data(my_iterator);
```

There is also a new `ShaderBuffer::set_data_raw` method which can be used for types implement `bytemuck::NoUninit`.
