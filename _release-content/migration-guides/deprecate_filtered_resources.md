---
title: "`FilteredResources` and similar structs have been deprecated"
pull_requests: [25331]
---

`FilteredResources`, `FilteredResourcesMut`, `FilteredResourcesBuilder`, `FilteredResourcesMutBuilder`, `FilteredResourcesParamBuilder`, and `FilteredResourcesMutParamBuilder`, have been deprecated in favor of `QueryBuilder` and `QueryParamBuilder`.

The API has changed somewhat, below we provide an example.

```rust
// 0.19
let system = 
    FilteredResourcesParamBuilder::new(|builder| {
        builder.add_read::<ResA>();
    })
    .build_state(&mut world)
    .build_system(resource_system);

fn resource_system(filtered: FilteredResources) {
   let resource_a: Ref<ResA> = filtered.get::<ResA>().unwrap();
}

// 0.20
let system =
    QueryParamBuilder::new(|builder| {
        builder.data::<Ref<ResA>>().with::<IsResource>();
    })
    .build_state(&mut world)
    .build_system(resource_system);

fn resource_system(query: Query<()>) {
    let resource_a: Ref<ResA> = query.single().unwrap();
}
```

So instead of a `FilteredResourcesParamBuilder` that provides a `FilteredResourcesBuilder`, which resolves to `FilteredResources`, we have a `QueryParamBuilder` that provides a `QueryBuilder` that resolves to a `Query`. The `Mut` variants also turn into `Query`, `QueryParam`, and `QueryParamBuilder`.
Most of the migration should be rather straightforward, but there are some specifics we need to clear up.
First, change detection was automatically included for `FilteredResources` and `FilteredResourcesMut`, which is now opt-in. You have to specify `Ref` and `Mut` in `QueryBuilder::data` if you want change detection.
Secondly, when is adding `.with::<IsResource>` necessary? In general, `.with::<IsResource>` is used to stop system conflicts. Take a look at the following example:

```rust
// 0.20
fn resource_system(resource_query: Query<()>, broad_query: Query<EntityMut>) {}

let system = (
    QueryParamBuilder::new(|builder| {
        builder.data::<&mut ResA>();
    }),
    ParamBuilder,
)
    .build_state(&mut world)
    .build_system(resource_system); // panic!
```

Here, `.build_system` panics, because `broad_query` also has mutable access to `ResA`, just as `resource_query` does.
In order to avoid conflicts, you can add an `IsResource` filter, like so:

```rust
// 0.20
fn resource_system(resource_query: Query<()>, broad_query: Query<EntityMut, Without<IsResource>>) {}

let system = (
    QueryParamBuilder::new(|builder| {
        builder.data::<&mut ResA>().with::<IsResource>();
    }),
    ParamBuilder,
)
    .build_state(&mut world)
    .build_system(resource_system); // works!
```

Adding `IsResource` is therefor only occasionally necessary, as these conflicts arise. Still, since a resource entity always has an `IsResource` marker attached, it can't hurt.
