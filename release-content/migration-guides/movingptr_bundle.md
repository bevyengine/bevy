---
title: Implement `Bundle` for `MovingPtr`
pull_requests: [21454]
---

`MovingPtr<'_, B: Bundle>` now implements the `Bundle` trait.
`MovingPtr<'_, B: Bundle>` can also be part of another bundle, such as a tuple bundle:

```rust
fn assert_bundle<B: Bundle>() {}
assert_bundle::<Transform>();
assert_bundle::<MovingPtr<'_, Transform>>();
assert_bundle::<(Name, MovingPtr<'_, Transform>)>();
```

Implementers of traits such as `SpawnableList`, which work with `MovingPtr`s, can now spawn bundles directly from a `MovingPtr<'_, B: Bundle>` rather than having to copy the bundle to the stack first.

```rust
struct MyList<B: Bundle> {
    bundle: B
}

// Previously
fn spawn(this: MovingPtr<'_, Self>, world: &mut World, entity: Entity) {
    deconstruct_moving_ptr!({
        let MyList { bundle } = this;
    });
    let bundle = bundle.read();
    world.spawn((bundle, R::from(entity)));
}

// Now
fn spawn(this: MovingPtr<'_, Self>, world: &mut World, entity: Entity) {
    deconstruct_moving_ptr!({
        let MyList { bundle } = this;
    });
    world.spawn((bundle, R::from(entity)));
}
```
