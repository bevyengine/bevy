---
title: Deprecate `UnsafeCellDeref`
pull_requests: []
---

The `UnsafeCellDeref` trait has been deprecated,
as the functionality is now available directly on `UnsafeCell`.

```rust
let x: UnsafeCell<T> = ...;
// 0.20
use bevy_ptr::UnsafeCellDeref;
let v: &T = unsafe { x.deref() };
let v: &mut T = unsafe { x.deref_mut() };
let v: T = unsafe { x.read() };
// 0.21
let v: &T = unsafe { x.as_ref_unchecked() };
let v: &mut T = unsafe { x.as_mut_unchecked() };
let v: T = unsafe { *x.as_ref_unchecked() };
```
