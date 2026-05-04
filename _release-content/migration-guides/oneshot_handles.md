---
title: "One-shot system functions now return `SystemHandle`s"
pull_requests: [24114]
---

One-shot system functions like `World::register_system`, `World::register_boxed_system`,
`App::register_system`, or `SubApp::register_system` now return `SystemHandle`s
instead of `SystemId`s. Consider storing these rather than `SystemId`s in order
to support automatic cleanup of one-shot systems.

```rust
fn my_system() {}

// Bevy 0.18
#[derive(Component)]
pub struct MyCallback(SystemId);

let id = world.register_system(my_system);
let entity = world.spawn(MyCallback(id)).id();
world.despawn(entity); // Doesn't automatically cleanup the registered system!

// Bevy 0.19
#[derive(Component)]
pub struct MyCallback(SystemHandle);

let handle = world.register_system(my_system);
let entity = world.spawn(MyCallback(handle)).id();
world.despawn(entity);
// The registered system entity will be despawned at the next call to the
// `despawn_unused_registered_systems` system, which is in the `Last` schedule
// by default.
```

Calling `World::run_system` or `World::run_system_with` with handles may attempt
to take ownership of the handle, which wasn't a problem previously because
`SystemId`s are `Copy`. To avoid this issue, pass the handle by reference:

```rust
// Bevy 0.18
let id = world.register_system(my_system);
world.run_system(id);
world.run_system(id);
world.run_system(id);

// Bevy 0.19
let handle = world.register_system(my_system);
world.run_system(&handle);
world.run_system(&handle);
world.run_system(&handle);
```

`bevy_remote` was migrated from `SystemId`s to `SystemHandle`s in the following ways:

- `bevy_remote::RemoteMethods` functions now accept and return `RemoteMethodSystemHandle`,
  which holds `SystemHandle`s rather than `SystemId`s. If necessary, you may convert
  `SystemId`s into weak `SystemHandle`s. Most likely, the changes to the one-shot
  system registration functions mean you've probably been switched to `SystemHandle`s
  automatically.
