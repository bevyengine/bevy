---
title: Entity Reference Counting
authors: ["@andriyDev"]
pull_requests: []
---

`Arc` is a common tool in a Rust programmer's toolbelt. It allows you to allocate data and then
reference that data in multiple places. Most importantly, it drops the data once all references have
been removed.

We've recreated this tool for entities! With `EntityRc`, users can now reference an entity and have
it automatically despawned when all `EntityRc`s have been dropped.

To do this, first create an `EntityRcSource`, and store it somewhere (like a resource).

```rust
#[derive(Resource)]
struct OrderedReferenceCount {
    order: u32,
    rc_source: EntityRcSource,
}

fn setup(mut commands: Commands) {
    commands.insert_resource(OrderedReferenceCount {
        order: 0,
        rc_source: EntityRcSource::new(),
    });
}
```

Next, create a system to regularly handle any drops:

```rust
fn handle_drops(ordered_internal: Res<OrderedReferenceCount>, mut commands: Commands) {
    ordered_internal.handle_dropped_rcs(&mut commands);
}
```

Lastly, provide an interface for users to create `EntityRc`s:

```rust
#[derive(SystemParam)]
pub struct CreateReferences<'w, 's> {
    ordered_internal: ResMut<'w, OrderedReferenceCount>,
    commands: Commands<'w, 's>,
}

// We expect most uses will wrap this reference-count in a wrapper that provides a more strict API.
pub struct MyHandle(EntityRc<u32>);

impl MyHandle {
    pub fn get_order(&self) -> u32 {
        *self.0
    }
}

impl CreateReferences {
    pub fn create_reference(&mut self) -> MyHandle {
        // Spawn an entity to be reference-counted.
        let entity = self.commands.spawn((Transform::from_xyz(10.0, 20.0, 30.0))).id();
        self.ordered_internal.order += 1;
        // Store the order number in the `EntityRc` so it can be accessed from any handle. This can
        // store whatever you want!
        self.ordered_internal.rc_source.create_rc(entity, self.ordered_internal.order);
    }
}
```

This provides users with an API like:

```rust
fn user_system(mut refs: CreateReferences, mut commands: Commands) {
    let new_handle = refs.create_reference();
    commands.spawn(HoldsAReference(new_handle));
}
```
