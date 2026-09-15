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

fn resource_system(query: Query<FilteredEntityRef>) {
    let entity: FilteredEntityRef = query.single().unwrap(); // Or use `Single<FilteredEntityRef>` as a parameter!
    let resource_a: &A = entity.get::<A>().unwrap();
    // Or with change tracking
    let resource_a: Ref<A> = entity.get_ref::<A>().unwrap();
    // Or by ID
    let resource: Ptr = entity.get_by_id(component_id).unwrap();
    let change_ticks: ComponentTicks = entity.get_change_ticks_by_id(component_id).unwrap();
}
```

So instead of a `FilteredResourcesParamBuilder` that provides a `FilteredResourcesBuilder`, which resolves to `FilteredResources`, we have a `QueryParamBuilder` that provides a `QueryBuilder` that resolves to a `Query`. The `Mut` variants also turn into `Query`, `QueryParam`, and `QueryParamBuilder`.
Most of the migration should be rather straightforward, but there are some specifics we need to clear up.
Firstly, when is adding `.with::<IsResource>` necessary? In general, `.with::<IsResource>` is used to stop system conflicts. Take a look at the following example:

```rust
// 0.20
fn resource_system(resource_query: Query<FilteredEntityRef>, broad_query: Query<EntityMut>) {}

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
fn resource_system(resource_query: Query<FilteredEntityRef>, broad_query: Query<EntityMut, Without<IsResource>>) {}

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

Secondly, there's the issue of dealing with multiple resources. Given a `Query<FilteredEntityRef>` with multiple resources, how do you extract the desired resource. For this, you'd have to know what `Entity` the resource is stored on. For this purpose, we provide the `ResourceEntities` system parameter. Querying multiple resources ends up looking as follows:

```rust
// 0.20
#[test]
let system = (
    QueryParamBuilder::new(|builder| {
        builder.data::<EntityRef>();
        builder.with::<IsResource>();
        builder.or(|builder| {
            builder.with::<ResA>();
            builder.with::<ResB>();
        });
    }),
    ParamBuilder,
    ParamBuilder,
)
    .build_state(&mut world)
    .build_system(resource_system);

fn resource_system(
    query: Query<FilteredEntityRef>,
    resource_entities: &ResourceEntities,
    components: &Components,
) {
    // this can be done for every resource separately. 
    let component_id = components.get_id(TypeId::of::<ResA>()).unwrap();
    let entity = resource_entities.get(component_id).unwrap();
    let entity_ref: FilteredEntityRef = query.get(entity).unwrap();
    let value = entity_ref_a.get::<ResA>().unwrap();
}
```
