---
title: Assets-as-entities
pull_requests: [22939]
---

Previously assets were stored in a resource named `Assets`. Assets are now stored as components on
entities in the ECS world. This has required us to redesign some of our APIs. Here are some common
cases and their replacements:

## 1. Reading asset data

- Replace `Res<Assets<A>>` with `Assets<A>`.

Before:

```rust
fn my_system(meshes: Res<Assets<Mesh>>) {
    let handle = ...;
    let data: &Mesh = meshes.get(&handle).unwrap();
}
```

After:

```rust
fn my_system(meshes: Assets<Mesh>) {
    let handle = ...;
    let data: &Mesh = meshes.get(&handle).unwrap();
}
```

## 2. Mutating asset data

- Replace `ResMut<Assets<A>>` with `AssetMut<A>`.
- When assigning `get_mut` to a variable, make the variable mutable (since we return `AssetMut` now).

Before:

```rust
fn my_system(mut meshes: ResMut<Assets<Mesh>>) {
    let handle = ...;
    let data: &mut Mesh = meshes.get_mut(&handle).unwrap();
}
```

After:

```rust
fn my_system(mut meshes: AssetsMut<Mesh>) {
    let handle = ...;
    let mut data: AssetMut<Mesh> = meshes.get(&handle).unwrap();
}
```

## 3. Adding new assets

- Replace `ResMut<Assets<T>>` with `AssetCommands`.
  - For multiple asset types, you only need one `AssetCommands`.
- Replace `.add()` with `.spawn_asset()`.
- For types that previously implicitly converted to your type (e.g., `Cuboid` implements
  `Into<Mesh>`), you must surround the value in `MyType::from`.
  - If the asset type is "constrained" (e.g., you store the handle into `Handle<Mesh>`), you can
    use `.into()` to convert your value instead.

Before:

```rust
fn my_system(mut meshes: ResMut<Assets<Mesh>>) {
    // Cuboid gets implicitly converted to Mesh.
    let handle = meshes.add(Cuboid::new(1.0, 2.0, 3.0));
}
```

After:

```rust
fn my_system(mut asset_commands: AssetCommands) {
    let handle = asset_commands.spawn_asset(Mesh::from(Cuboid::new(1.0, 2.0, 3.0)));
}
```

Or:

```rust
fn my_system(mut asset_commands: AssetCommands) {
    let handle: Handle<Mesh> = asset_commands.spawn_asset(Cuboid::new(1.0, 2.0, 3.0).into());
}
```

## 4. Removing assets

- Replace `ResMut<Assets<A>>` with `AssetCommands`.
  - For multiple asset types, you only need one `AssetCommands`.
- Replace `.remove()` with `.remove_asset()`.
  - **This does not return the asset**. To get the asset back, you can enqueue a command in
    `Commands`, then use `world.remove_asset()`.

Before:

```rust
fn my_system(mut meshes: ResMut<Assets<Mesh>>) {
    let handle = ...;
    // We get back the data here.
    let mesh = meshes.remove(&handle).unwrap();
}
```

After:

```rust
fn my_system(mut asset_commands: AssetCommands) {
    let handle = ...;
    // We don't get the data here, since this action is deferred. You need exclusive world access.
    meshes.remove(&handle);
}
```

## 5. Spawning materials

- Since we can no longer deduce the asset type (since we have an untyped `AssetCommands` and
  `MeshMaterial3d` is generic), we need to explicitly convert colors to a material. Wrap your value
  in `StandardMaterial::from` or `ColorMaterial::from` for 3D or 2D respectively.

Before:

```rust
fn my_system(mut commands: Commands, mut materials: ResMut<Assets<StandardMaterial>>) {
    commands.spawn((
        Mesh3d(...),
        MeshMaterial3d(materials.add(Color::BLACK)),
    ));
}
```

After:

```rust
fn my_system(mut commands: Commands, mut asset_commands: AssetCommands) {
    commands.spawn((
        Mesh3d(...),
        MeshMaterial3d(asset_commands.spawn_asset(StandardMaterial::from(Color::BLACK))),
    ));
}
```

## 6. UUID assets

- Instead of accessing the `Assets<A>` resource and inserting the asset into the handle, call
  `world.spawn_uuid_asset()` or `app.world_mut().spawn_uuid_asset()`.

Before:

```rust
const IMAGE: Handle<Image> = uuid_handle!("1347c9b7-c46a-48e7-b7b8-023a354b7cac");

fn my_plugin(app: &mut App) {
    app.world_mut().resource_mut::<Assets<Image>>().insert(&IMAGE, create_some_image());
}
```

After:

```rust
const IMAGE: Handle<Image> = uuid_handle!("1347c9b7-c46a-48e7-b7b8-023a354b7cac");

fn my_plugin(app: &mut App) {
    app.world_mut().spawn_uuid_asset(IMAGE.uuid().unwrap(), create_some_image());
}
```

## Dealing with "deferred" assets

Some existing code may assume that adding an asset is instant - in other words, you can call
`Assets::add` and then `Assets::get_mut` to mutate that added asset in the same system. Now that
assets need to be spawned, calling `AssetCommands::spawn_asset` followed by `AssetsMut::get_mut`
does not allow you to mutably access the asset. This acts the same way how calling `Commands::spawn`
followed by `Query::get_mut` does not allow you to mutably access a component.

There are several ways to deal with this. One possibility is to change your system to be exclusive,
spawning assets with `DirectAssetAccessExt::spawn_asset`, and mutating the asset with
`DirectAssetAccessExt::get_asset_mut`. Since these operate on a world, spawning the asset happens
immediately.

Another possibility is to defer the spawning of assets until the end of your system: allow accessing
an asset either through `AssetsMut` or through a local "pending" collection. Create assets in this
pending collection instead of spawning them, and then at the end of your system, spawn all pending
assets. This approach won't work in all cases, but can be straight forward when possible!

## Misc changes

- `Assets::len` -> `Assets::count`
- `Assets::reserve_handle` -> `AssetCommands::reserve_handle` / `DirectAssetAccessExt::reserve_asset_handle`.
- `AssetServer::get_id_handle` -> `AssetServer::get_entity_handle`
- `ReflectAsset::assets_resource_type_id` -> `ReflectAsset::asset_data_type_id`
- `ReflectAsset::add` -> `ReflectAsset::spawn`
- `ReflectAsset::len` -> `ReflectAsset::count`
