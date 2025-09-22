---
title: `DynamicBundle`
pull_requests: [20772, 20877]
---

In order to reduce the stack size taken up by spawning and inserting large bundles, the way the (mostly internal) trait `DynamicBundle` gets called has changed significantly:

```rust
// 0.16
trait DynamicBundle {
    type Effect;
    
    // hidden in the docs
    fn get_components(self, func: &mut impl FnMut(StorageType, OwningPtr<'_>)) -> Self::Effect;
}

// 0.17
trait DynamicBundle {
    type Effect;
    
    unsafe fn get_components(ptr: MovingPtr<'_, Self>, func: &mut impl FnMut(StorageType, MovingPtr<'_>));
    unsafe fn apply_effect(ptr: MovingPtr<'_, MaybeUninit<Self>>, entity: &mut EntityWorldMut);
}
```

To prevent unnecessary copies to the stack, `get_components` now takes a `MovingPtr<'_, Self>` instead of `self` by value.

`MovingPtr<T>` is a safe, typed, box-like pointer that owns the data it points to, but not the underlying memory: that means the owner of a `MovingPtr<T>` can freely move parts of the data out and doesn't have to worry about de-allocating memory.
Much like `Box<T>`, `MovingPtr<T>` implements `Deref` and `DerefMut` for easy access to the stored type, when it's safe to do so.
To decompose the value inside of the `MovingPtr<T>` into its fields without copying them to the stack, you can use the `deconstruct_moving_ptr!` macro to give you `MovingPtr<U>`s to each field specified:

```rust
struct MySpecialBundle<A: Bundle, B: Bundle> {
    a: A,
    b: B,
}
let my_ptr: MovingPtr<'_, MySpecialBundle<u32, String>> = ...;
deconstruct_moving_ptr!(my_ptr => { a, b, });
let a_ptr: MovingPtr<'_, u32> = a;
let b_ptr: MovingPtr<'_, String> = b;
```

Similar to `Box::into_inner`, `MovingPtr<T>` also has a method `MovingPtr::read` for moving the whole value out of the pointer onto the stack:

```rust
let a: u32 = a_ptr.read();
let b: String = b_ptr.read();
```

`apply_effect` is a new method that takes the job of the old `BundleEffect` trait, and gets called once after `get_components` for any `B::Effect: !NoBundleEffect`.
Since `get_components` might have already partially moved out some of the fields of the bundle, `apply_effect` takes a `MovingPtr<'_, MaybeUninit<Self>>` and implementers must make sure not to create any references to fields that are no longer initialized.
Likewise, implementers of `get_components` must take care not to move out fields that will be needed in `apply_effect`. `deconstruct_moving_ptr!` can be used to selectively move out fields while ensuring the rest are forgotten, and remain valid for the subsequent call to `apply_effect`.
The associated type `Effect` remains as a vestigial marker to keep track of whether `apply_effect` needs to be called for any `B::Effect: !NoBundleEffect`.
