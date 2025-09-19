---
title: Reflect auto registration
authors: ["@eugineerd"]
pull_requests: [15030]
---

Deriving [`Reflect`] on types opts into **Bevy's** runtime reflection infrastructure, which is used to power systems like runtime component inspection and serialization:

```rust
#[derive(Reflect)]
pub struct Foo {
  a: usize,
}
```

In previous Bevy versions, any top-level
types that derived [`Reflect`] had to be manually registered using [`register_type`]:

```rust
// This would make Foo visible to Bevy
app.register_type::<Foo>()
```

In **Bevy 0.17**, all types that [`#[derive(Reflect)]`] are now automatically registered! This significantly reduces the boilerplate required to use Bevy's reflection features, which will be increasingly important as we build out Bevy's new scene system, entity inspector, and visual editor.

Note that generic types still require manual registration, as these types don't (yet) exist when [`Reflect`] is derived:

```rust
app.register_type::<Container<Item>>()
```

In cases where automatic registration is undesirable, it can be opted-out of by adding the `#[reflect(no_auto_register)]` attribute to the type.

## Supporting unsupported platforms

This feature relies on the [`inventory`] crate to collect all type registrations at compile-time. This is supported on Bevy's most popular platforms: Windows, macOS, iOS, Android, and WebAssembly. However, some niche platforms are not supported by [`inventory`], and while it would be best for
any unsupported platforms to be supported upstream, sometimes it might not be possible. For this reason, there is a different implementation of this feature that works on all platforms.
It comes with some caveats with regards to project structure and might increase compile time, so it is better used as a backup solution. The detailed instructions on how to use this feature
can be found in this [`example`]. Types can also still be manually registered using `app.register_type::<T>()`.

[`Reflect`]: https://docs.rs/bevy/0.17.0/bevy/prelude/trait.Reflect.html
[`inventory`]: https://github.com/dtolnay/inventory
[`example`]: https://github.com/bevyengine/bevy/tree/release-0.17.0/examples/reflection/auto_register_static
[`register_type`]: https://docs.rs/bevy/0.17.0/bevy/prelude/struct.App.html#method.register_type
