---
title: Rename `ThinSlicePtr::get()` to `ThinSlicePtr::get_unchecked()`
pull_requests: [21823]
---

`ThinSlicePtr::get()` has been deprecated in favor of the new `ThinSlicePtr::get_unchecked()`
method in order to more clearly signal that bounds checking is not performed. Beyond the name
change, the only difference between these two methods is that `get_unchecked()` takes `&self` while
`get()` takes `self`. In order to migrate, simply rename all usages of `get()` with
`get_unchecked()`:

```rust
let slice: &[u32] = &[2, 4, 8];
let thin_slice = ThinSlicePtr::from(slice);

// 0.17
let x = unsafe { thin_slice.get(0) };

// 0.18
let x = unsafe { thin_slice.get_unchecked(0) };
```
