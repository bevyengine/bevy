---
title: Reflect auto registration
authors: ["@eugineerd"]
pull_requests: [15030]
---

## Automatic [`Reflect`] registration

Deriving [`Reflect`] on types opts into **Bevy's** runtime reflection infrastructure, which is used to power systems like component runtime inspection and serialization. Before **Bevy 0.17**, any top-level
types that derive [`Reflect`] (not used as a field in some other [`Reflect`]-ed type) had to be manually registered using [`register_type`] for the runtime reflection to work with them. With this release,
all types that [`#[derive(Reflect)]`] are now automatically registered! This works for any types without generic type parameters and should reduce the boilerplate needed when adding functionality that depends on [`Reflect`].

```rs
fn main() {
  // No need to manually call .register_type::<Foo>()
  App::new()
    .add_plugins(DefaultPlugins)
    .add_systems(Startup, setup)
    .run();
}

#[derive(Reflect)]
pub struct Foo {
  a: usize,
}

fn setup(type_registry: Res<AppTypeRegistry>) {
  let type_registry = type_registry.read();
  assert!(type_registry.contains(TypeId::of::<Foo>()));
}
```

In cases where automatic registration is undesirable, it can be opted-out of by adding #[reflect(no_auto_register)] reflect attribute to a type:

```rs
#[derive(Reflect)]
#[reflect(no_auto_register)]
pub struct Foo {
  a: usize,
}
```

## Unsupported platforms

This feature relies on the [`inventory`] crate to collect all type registrations at compile-time. However, some niche platforms are not supported by [`inventory`], and while it would be best for
any unsupported platforms to be supported upstream, sometimes it might not be possible. For this reason, there is a different implementation of this feature that works on all platforms.
It comes with some caveats with regards to project structure and might increase compile time, so it is better used as a backup solution. The detailed instructions on how to use this feature
can be found in this [`example`]. Types can also still be manually registered using `app.register_type::<T>()`.

[`Reflect`]: https://docs.rs/bevy/0.17.0/bevy/prelude/trait.Reflect.html
[`inventory`]: https://github.com/dtolnay/inventory
[`example`]: https://github.com/bevyengine/bevy/tree/release-0.17.0/examples/reflection/auto_register_static
[`register_type`]: https://docs.rs/bevy/0.17.0/bevy/prelude/struct.App.html#method.register_type
