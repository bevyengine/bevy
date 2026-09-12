---
title: ReflectFromPtr::from_ptr and ReflectFromPtr::from_ptr_mut replaced with ReflectFromPtr::raw_pointer_cast
pull_requests: [25754]
---

Previously, `ReflectFromPtr` had to methods `from_ptr` and `from_ptr_mut` which returned the
function pointers for casting from a `Ptr` to a `&dyn Reflect`, and from a `PtrMut` to a `&mut dyn Reflect`,
respectively. These were very constrained to these two particular casts and required going into a
reference (which can have soundness implications).

Now, we have one `ReflectFromPtr::raw_pointer_cast`. This simply does a cast from an arbitrary
pointer, into the same pointer as a trait object for the type used to construct the
`ReflectFromPtr`.

It is not possible to maintain the previous type, however at the callsite of the function pointer,
you can do:

```rust
// Before

let from_ptr = reflect_from_ptr.from_ptr();
let from_ptr_mut = reflect_from_ptr.from_ptr_mut();

let my_ptr: Ptr = todo!();
// SAFETY: Because I said so!
let reflect: &dyn Reflect = unsafe { (from_ptr)(my_ptr) };

let my_ptr_mut: PtrMut = todo!();
// SAFETY: I'm doubling down!
let reflect_mut: &mut dyn Reflect = unsafe { (from_ptr_mut)(my_ptr_mut) };

// After

let raw_pointer_cast = reflect_from_ptr.raw_pointer_cast();

let my_ptr: Ptr = todo!();
let reflect_ptr: *const dyn Reflect = raw_pointer_cast(my_ptr.as_ptr().cast::<()>().cast_mut()).cast_const();
// SAFETY: Same reasoning as before.
let reflect: &dyn Reflect = unsafe { &*reflect_ptr };

let my_ptr_mut: PtrMut = todo!();
let reflect_ptr_mut: *mut dyn Reflect = raw_pointer_cast(my_ptr.as_ptr().cast());
// SAFETY: Same reasoning as before.
let reflect_mut: &mut dyn Reflect = unsafe { &mut *reflect_ptr };

```

If you would like to "robustify" your safety comments, it may be useful to note that
`ReflectFromPtr::raw_pointer_cast` promises not to modify the passed-in pointer.
