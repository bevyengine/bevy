---
title: Some traits no longer provide downcasting helpers
pull_requests: [26505]
---

Several dyn-compatible traits provided downcasting helpers through `downcast_rs::Downcast` and a macro. The `downcast-rs` dependency has been removed, so these helpers are gone. They can be easily replaced with either standard coercion for concrete types, or trait upcasting, a language feature since Rust 1.86.

The removed helpers are:

- Conversions to `Any`: `as_any(&self) -> &dyn Any`, `as_any_mut(&mut self) -> &mut dyn Any`, `into_any(self: Box<Self>) -> Box<dyn Any>`, and `into_any_rc(self: Rc<Self>) -> Rc<dyn Any>`.
- Downcasts to a concrete type: `is::<T>()`, `downcast_ref::<T>()`, `downcast_mut::<T>()`, and `downcast::<T>()`.

The affected public traits are:

- `Plugin` (`bevy_app`)
- `Settings` (`bevy_asset`)
- `AssetMetaDyn` (`bevy_asset`)
- `AnimationCurveEvaluator` (`bevy_animation`)
- `TypeData` (`bevy_reflect`)

The `Any` conversion helpers (`as_any`, `as_any_mut`, `into_any`) should be replaced with direct coercion to `dyn Any`.

Given:

```rust
trait Trait: Any {}
struct Foo;
impl Trait for Foo {}

let x = Foo;
let bx: Box<dyn Trait> = Box::new(Foo);
```

Then:

```rust
// old
let y: &dyn Any = x.as_any();
let by: Box<dyn Any> = bx.into_any();

// new
let y = &x as &dyn Any;
let by = bx as Box<dyn Any>;
```

The downcasting helpers (`downcast_ref`, `downcast_mut`, `is`) should be replaced with upcasting the trait object to `dyn Any` first, then calling the corresponding `Any` method.

Given:

```rust
trait Trait: Any {}
struct Foo;
impl Trait for Foo {}

let x: &dyn Trait = &Foo;
```

Then:

```rust
// old
let y = x.downcast_ref::<Foo>().unwrap();
// new
let y = (x as &dyn Any).downcast_ref::<Foo>().unwrap();
```

For wrapped trait objects (`Box` and `Rc`), dereferencing first is necessary to not downcast the wrapper:

```rust
let x: Box<dyn Trait> = Box::new(Foo);

// old
let y = x.downcast_ref::<Foo>();
// new, but probably not what you want:
let y = (&x as &dyn Any).downcast_ref::<Foo>();
// new, correct:
let y = (&*x as &dyn Any).downcast_ref::<Foo>();
```

The owned `downcast::<T>()` helper is replaced similarly (coercing to `Any` first):

```rust
let bx: Box<dyn Trait> = Box::new(Foo);

// old                  note vvvvvvvvvvvvvvv
let result: Result<Box<Foo>, Box<dyn Trait>> = bx.downcast::<Foo>();
// new                  note vvvvvvvvvvvvv
let result: Result<Box<Foo>, Box<dyn Any>> = (bx as Box<dyn Any>).downcast::<Foo>();
```

If you need to recover the original type on failure as before, check the type before downcasting:

```rust
let value = if (&*bx as &dyn Any).is::<Foo>() {
    Ok(*(bx as Box<dyn Any>).downcast::<Foo>().unwrap())
} else {
    Err(bx)
};
```
