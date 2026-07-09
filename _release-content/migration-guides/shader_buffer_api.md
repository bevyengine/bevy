---
title: "`ShaderBuffer` API changes"
pull_requests: [24915]
---

1. `ShaderBuffer::new` now accepts `Vec<u8>` instead of `&[u8]`. Use `[u8]::to_vec` to create a `Vec` if you want.
2. `ShaderBuffer::from` now accepts `Vec<T>` where `T: bytemuck::NoUninit`. `From<T: ShaderType + WriteInto>` is replaced by `ShaderBuffer::from_value`.
3. `ShaderBuffer::set_data` now accepts `impl Iterator<Item = T>` instead of a single `T` value. Use `core::iter::once` for single value. For iterators that were previously collected into a `Vec`, pass the iterator directly.
4. Previously `buffer.set_data(empty_vec)` would insert one zeroed `T` element. Now `set_data` with an empty iterator will make the data empty. You can check the iterator by `peekable` and insert zeros to ensure the buffer isn't zero-sized.

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
// If your iterator is never empty, the behavior is the same as before.
buffer.set_data(my_iterator);
// Note that if your iterator may be empty, previously `set_data(empty_vec)` would write one zeroed element.
// Now you should check and write zeros explicitly.
let mut my_peekable_iterator = my_iterator.peekable();
if my_peekable_iterator.peek().is_none() {
    // Ensure the buffer is not zero-sized
    buffer.set_data_raw(core::iter::once(zeroed_values));
} else {
    buffer.set_data_raw(my_peekable_iterator);
}
```

There is also a new `ShaderBuffer::set_data_raw` method which can be used for types implement `bytemuck::NoUninit`.
