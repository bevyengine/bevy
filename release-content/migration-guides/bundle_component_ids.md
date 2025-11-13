---
title: Change  `Bundle::component_ids` and `Bundle::get_component_ids` to return an iterator.
pull_requests: [14791, 15458, 15269]
---

`Bundle::component_ids` and `Bundle::get_component_ids` were changed to return an iterator over
`ComponentId` and `Option<ComponentId>` respectively. In some cases this can avoid allocating.

```rust
// For implementors
// Before
unsafe impl<C: Component> Bundle for C {
    fn component_ids(components: &mut ComponentsRegistrator, ids: &mut impl FnMut(ComponentId)) {
        ids(components.register_component::<C>());
    }

    fn get_component_ids(components: &Components, ids: &mut impl FnMut(Option<ComponentId>)) {
        ids(components.get_id(TypeId::of::<C>()));
    }
}

// After
unsafe impl<C: Component> Bundle for C {
    fn component_ids<(
        components: &mut ComponentsRegistrator,
    // we use a `use` here to explicitly not capture the lifetime of `components`
    ) -> impl Iterator<Item = ComponentId> + use<C> {
        iter::once(components.register_component::<C>())
    }

    fn get_component_ids(components: &Components) -> impl Iterator<Item = Option<ComponentId>> {
        iter::once(components.get_id(TypeId::of::<C>()))
    }
}
```

```rust
// For Consumers
// Before
let mut components = vec![];
MyBundle::component_ids(&mut world.components_registrator(), &mut |id| {
    components.push(id);
});

// After
let components: Vec<_> = B::component_ids(&mut world.components_registrator()).collect();
```
