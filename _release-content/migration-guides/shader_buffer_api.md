---
title: "`ShaderBuffer` API changes"
pull_requests: [24915]
---

`ShaderBuffer::new` now accepts `Vec<u8>` instead of `&[u8]`. Use `[u8]::to_vec` to explicitly create a `Vec` if you want:

```rust
// BEFORE
ShaderBuffer::new(my_slice, RenderAssetUsages::default());

// AFTER
ShaderBuffer::new(my_slice.to_vec(), RenderAssetUsages::default());
```

`ShaderBuffer::set_data` now accepts `impl Iterator<Item = T>` instead of a single `T` value. Use `core::iter::once` for single value:

```rust
// BEFORE
buffer.set_data(my_value);

// AFTER
buffer.set_data(core::iter::once(my_value));
```

For iterators that were previously collected into a `Vec`, pass the iterator directly:

```rust
// BEFORE
buffer.set_data(my_iterator.collect::<Vec<_>>());

// AFTER
buffer.set_data(my_iterator);
```

Note that `ShaderBuffer::set_data` won't push any data if the iterator is empty (previously it will push at least one element with zeros).

There is also a new `ShaderBuffer::set_data_raw` method which can be used for types implement `bytemuck::NoUninit`.
