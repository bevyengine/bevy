---
title: `SpawnableList`
pull_requests: [20772, 20877]
---

In order to reduce the stack size taken up by spawning and inserting large bundles, `SpawnableList` now takes a `MovingPtr<T>` as the self-type for the `spawn` function:

```rust
// 0.16
fn spawn(self, world: &mut World, entity: Entity);
// 0.17
fn spawn(self: MovingPtr<'_, Self>, world: &mut World, entity: Entity);
```

This change also means that `SpawnableList` must now also be `Sized`!

`MovingPtr<T>` is a safe, typed, box-like pointer that owns the data it points to, but not the underlying memory: that means the owner of a `MovingPtr<T>` can freely move parts of the data out and doesn't have to worry about de-allocating memory.
Much like `Box<T>`, `MovingPtr<T>` implements `Deref` and `DerefMut` for easy access to the stored type, when it's safe to do so.
To decompose the value inside of the `MovingPtr<T>` into its fields without copying them to the stack, you can use the `deconstruct_moving_ptr!` macro to give you `MovingPtr<U>`s to each field specified:

```rust
struct MySpawnableList<A: Bundle, B: Bundle> {
    a: A,
    b: B,
}
let my_ptr: MovingPtr<'_, MySpawnableList<u32, String>> = ...;
deconstruct_moving_ptr!(my_ptr => { a, b, });
let a_ptr: MovingPtr<'_, u32> = a;
let b_ptr: MovingPtr<'_, String> = b;
```

Similar to `Box::into_inner`, `MovingPtr<T>` also has a method `MovingPtr::read` for moving the whole value out of the pointer onto the stack:

```rust
let a: u32 = a_ptr.read();
let b: String = b_ptr.read();
```

To migrate your implementations of `SpawnableList` to the new API, you will want to read the `this` parameter to spawn or insert any bundles stored within:

```rust

impl<R: Relationship> SpawnableList<R> for MySpawnableList<A: Bundle, B: Bundle> {
    // 0.16
    fn spawn(self, world: &mut World, entity: Entity) {
        let MySpawnableList { a, b } = self;
        world.spawn((R::from(entity), a, b));
    }

    // 0.17
    fn spawn(this: MovingPtr<'_, Self>, world: &mut World, entity: Entity) {
        let MySpawnableList { a, b } = this.read();
        world.spawn((R::from(entity), a, b));
    }
}
```

or only read the fields you need with `deconstruct_moving_ptr!`:

```rust
    fn spawn(this: MovingPtr<'_, Self>, world: &mut World, entity: Entity) {
        unsafe {
            // Only `a` is kept, `b` will be forgotten without being dropped!
            deconstruct_moving_ptr!(this => { a, });
            let a = a.read();
            world.spawn((R::from(entity), a));
        }
    }
```
