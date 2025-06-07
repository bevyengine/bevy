---
title: Option and dynamic bundles
authors: ["@SkiFire13"]
pull_requests: [19491]
---

Until now bundles have always represented a static set of components, meaning you couldn't create a `Bundle` that sometimes inserts a component and sometimes doesn't, or a totally dynamic bundle like you would expect from a `Box<dyn Bundle>`.

The `Bundle` trait has now been reworked to support these usecases, in particular:

- `Option<B: Bundle` now implements `Bundle`
- `Bundle` is now a dyn-compatible trait and `Box<dyn Bundle>` implements `Bundle`
- `Vec<Box<dyn Bundle>>` also implements `Bundle`

```rs
fn enemy_bundle(name: String, attack_power: u32, is_boss: bool) -> impl Bundle {
    (
        EnemyMarker,
        Name::new(name),
        AttackPower(attack_power),
        if is_boss { Some(BossMarker) } else { None }
    )
}

fn bundle_from_reflected_data(components: Vec<Box<dyn Reflect>>, registry: &TypeRegistry) -> impl Bundle {
    components
        .into_iter()
        .map(|data| {
            let Some(reflect_bundle) = registry.get_type_data::<ReflectBundle>(data.type_id()) else { todo!() };
            let Ok(bundle_box) = reflect_bundle.get_boxed(data) else { todo!() };
            bundle_box
        })
        .collect::<Vec<_>>()
}
```

## `StaticBundle`

In order to support these changes to `Bundle` we had to introduce a new trait, `StaticBundle`, for the cases where statically knowing the set of components is required, like for example `World::remove`.

The `Bundle` derive macro will automatically implement this trait for you, so most things should continue working like before, but in any case check out the migration guide!

In case however that your bundle contains some kind of dynamic bundle then this won't be possible, and you'll have to opt-out of automatically implementing `StaticBundle` and `BundleFromComponents` by adding the `#[reflect(dynamic)]` attribute.

```rs
#[derive(Bundle)]
#[derive(dynamic)] // This is required if any field is like the ones below!
struct MyBundle {
    foo: Option<FooComponent>,
    extra: Box<dyn Bundle>,
    extras: Vec<Box<dyn Bundle>>,
}
```
