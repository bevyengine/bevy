---
title: Async ECS Access
authors: ["@MalekiRe", "@andriyDev", "@dlom"]
pull_requests: [21744]
---

Asynchronous functions are a popular feature of Rust. They allow you to write code that appears
imperative (a sequence of instructions), but under the hood this generates a "state machine" which
can effectively be paused and resumed as operations complete. This also allows Rust to track the
usage of borrows within an async function entirely in safe Rust.

Unfortunately, accessing the ECS through an `async` function has not been easy. We've had some
"workarounds" for this historically (see the `async_channel_pattern` or `async_compute` examples).
While these approaches are still valid, they required users to write a lot of boilerplate.

As a result, we've created `bevy_async`! This crate provides a "bridge" that allows you to send arbitrary requests to the ECS, with the same functionality as systems (meaning queries, commands,
etc). More importantly (and unlike the two approaches above), it is easy to pass references into
this bridge, allowing for much simpler code without sacrificing any safety.

To use this, first add the `AsyncPlugin`:

```rust
App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(AsyncPlugin)
    .run();
```

Next, define a "sync point" type. This type is used as a marker to indicate which sync point will be
used by a given async bridge. This type should always be a unit struct:

```rust
struct MySyncPoint;
```

Next, add the sync point system to our app:

```rust
App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(AsyncPlugin)
    // Since we are adding this system to `Update`, bridged operations will occur in the `Update`
    // schedule. Adding multiple of this sync point system can allow bridge operations to be
    // resolved at many different points in the schedule (although consider whether you should have
    // multiple separate sync points instead).
    .add_systems(Update, async_world_sync_point::<MySyncPoint>)
    .run();
```

Finally, create a `SystemState` from `AsyncWorld`, and then use its `bridge` method to access the
ECS:

```rust
fn my_task_spawning_system(async_world: Res<AsyncWorld>) {
    // Note: If you're spawning several tasks with the same required access, clone an instance of
    // this and move it into each task! Sharing this state allows `Local`s to be reused, and change
    // detection to work properly.
    let system_state = async_world.system_state();

    AsyncComputeTaskPool::get().spawn(async move {
        // This might take a while. Putting it in a task avoids our game from being blocked on
        // loading this file.
        let my_scene_format = read_a_file_and_parse_it().await;
        system_state.bridge(MySyncPoint,
            |mut commands: Commands,
            query: Query<Entity, With<SceneMarker>>| {
            // Despawn any pre-existing scenes.
            for entity in query.iter() {
                commands.entity(entity).despawn();
            }
            // Bridge functions run in the context of the async task, so we can take references to
            // its variables, like `my_scene_format` here!
            commands.spawn((MyScene(my_scene_format.clone()), SceneMarker));
        })
        .await
        .unwrap();
    });
}
```
