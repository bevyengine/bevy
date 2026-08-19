---
title: "`ShaderBuffer` stores typed data in an `AlignedVec` with an explicit buffer size"
pull_requests: [24938, 25426]
---

`ShaderBuffer` CPU data is now stored in an `AlignedVec` whose alignment can be
configured at runtime. The allocation is guaranteed to be aligned to the element
type used to fill it, which allows typed reads and writes through
`bytemuck::cast_slice` / `cast_slice_mut` without extra copies or unaligned
access.

The GPU buffer size is now explicit and decoupled from the CPU data length:
`ShaderBufferData::Initialized` carries a `buffer_size` that can differ from the
length of `data`, so a larger GPU allocation can be kept while CPU data is
cleared or resized, avoiding reallocations.

When creating the GPU buffer, `buffer_size` always wins over the data length:
the data is truncated when it is longer than `buffer_size`, and the buffer is
zero-filled when it is shorter.

The public fields of `ShaderBuffer` have changed:

- `data: Option<Vec<u8>>` is now `data: ShaderBufferData`
  (`Initialized { data, buffer_size }` or `Uninitialized(size)`).
- `buffer_description: wgpu::BufferDescriptor<'static>` has been replaced by the
  `label: Cow<'static, str>` and `buffer_usage: BufferUsages` fields.

The following methods changed:

- `ShaderBuffer::new` now takes a `Vec<T>` of `bytemuck::NoUninit` elements
  instead of a byte slice:

  ```rust
  // Bevy 0.19
  let buffer = ShaderBuffer::new(&bytes, RenderAssetUsages::default());

  // Bevy 0.20
  let buffer = ShaderBuffer::new(bytes, RenderAssetUsages::default());
  ```

- `ShaderBuffer::set_data` which uses `encase::ShaderType`, has been removed. Build the buffer with `new` or fill
  an existing one with `extend` / `extend_from_slice` instead:

  ```rust
  // Bevy 0.19
  let mut buffer = ShaderBuffer::default();
  buffer.set_data(my_struct);

  // Bevy 0.20
  let mut buffer = ShaderBuffer::default();
  buffer.extend([my_struct]);
  ```

- `From<T>` which uses `encase::ShaderType`, is replaced by `From<Vec<T>>` which reuses the memory:

  ```rust
  // Bevy 0.19
  let buffer = ShaderBuffer::from(my_struct);

  // Bevy 0.20
  let buffer = ShaderBuffer::from(vec![my_struct]);
  ```

- `ShaderBuffer::with_size` now takes a `u64` instead of a `usize`.
- `ShaderBuffer::resize_in_place` has been removed. Use `ShaderBuffer::resize_buffer`
  to resize the GPU buffer without touching the CPU data, or `resize` for both.
- `ShaderBuffer::cast_slice` / `cast_slice_mut` provide typed access to the data
  (new in 0.20). In 0.19 the data was only reachable as raw bytes through the
  `data` field; casting that `Vec<u8>` was not guaranteed to be aligned.
- `ShaderBuffer::buffer_size()` returns the GPU buffer size; use
  `buffer_size() == 0` to check for a zero-sized buffer.
