---
title: "`ShaderBuffer::set_data` now takes an iterator"
pull_requests: [24915]
---

`ShaderBuffer::set_data` now accepts `impl Iterator<Item = T>` instead of a single `T` value. Use `std::iter::once` for single value:

```rust
// BEFORE
buffer.set_data(my_value);

// AFTER
buffer.set_data(core::iter::once(my_value));
```

For iterators that were previously collected into a `Vec`, pass the iterator directly.

```rust
// BEFORE
buffer.set_data(my_iterator.collect::<Vec<_>>());

// AFTER
buffer.set_data(my_iterator);
```

There is also a new `ShaderBuffer::set_data_raw` method which can be used for types implement `bytemuck::NoUninit`.
