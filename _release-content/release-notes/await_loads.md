---
title: Waiting for ~~Godot~~ assets
authors: ["@andriyDev"]
pull_requests: [26546]
---

(The title references the book "Waiting for Godot", not the game engine)

In Bevy, assets are naturally asynchronous. You call `asset_server.load("my/file/path")`, and after
a few frames, the asset can be accessed in the ECS. This avoids stuttering from assets being loaded,
but means users need to handle the case where their assets are loading.

In most games, this is exactly where a loading screen would show up. Historically, Bevy hasn't had
tools for this. Now, we do! Most importantly, these tools don't have to be used for traditional
loading screens. For example, a common trope in games is to have your character "squeeze" through a
narrow gap (with a fixed animation) to enter a new area; in actuality these animations are a sort of
loading screen! Our tools are just as applicable for these cases.

You can use the `AwaitLoadsBuilder` as a replacement to `AssetServer`. This internally tracks all
executed loads. Once you've loaded all your assets, you can spawn the resulting `AwaitLoads` and
observe it to react to the assets being loaded. For example:

```rust
fn my_startup_system(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut load_builder = AwaitLoadsBuilder::from(asset_server);

    let house_world_asset = load_builder.load("house.gltf#Scene0");
    let tree_world_asset = load_builder.load("tree.gltf#Scene0");
    let player_world_asset = load_builder.load("player.gltf#Scene0");

    commands.spawn(load_builder.build()).observe(
        move |_: On<Insert<LoadFinished>>, mut commands: Commands| {
            commands.spawn((
                WorldAssetRoot(house_world_asset.clone()),
                Transform::from_xyz(0.0, 0.0, 10.0),
            ));

            commands.spawn((
                WorldAssetRoot(tree_world_asset.clone()),
                Transform::from_xyz(3.0, 0.0, 3.0),
            ));

            commands.spawn(WorldAssetRoot(player_world_asset.clone()));
        }
    );
}
```

There are many ways to use this API, and we've documented several patterns for using these tools [in
the module docs](https://docs.rs/bevy/0.21.0/bevy/asset/await_loads/index.html). You can also check
out the [loading screen](https://bevy.org/examples/games/loading-screen/) example. Play around with
these tools, and let us know about any other useful patterns you end up using!
