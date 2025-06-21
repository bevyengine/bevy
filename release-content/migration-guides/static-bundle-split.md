---
title: StaticBundle split off from Bundle
pull_requests: [19491]
---

The `StaticBundle` trait has been split off from the `Bundle` trait to avoid conflating the concept of a type whose values can be inserted into an entity (`Bundle`) with the concept of a statically known set of components (`StaticBundle`). This required the update of existing APIs that were using `Bundle` as a statically known set of components to use `StaticBundle` instead.

Changes for most users will be zero or pretty minimal, since `#[derive(Bundle)]` will automatically derive `StaticBundle` and most types that implemented `Bundle` will now also implement `StaticBundle`. The main exception will be generic APIs or types, which now will need to update or add a bound on `StaticBundle`. For example:

```rs
// 0.16
#[derive(Bundle)]
struct MyBundleWrapper<T: Bundle> {
    inner: T
}

fn my_register_bundle<T: Bundle>(world: &mut World) {
    world.register_bundle::<T>();
}


// 0.17
#[derive(Bundle)]
struct MyBundleWrapper<T: Bundle + StaticBundle> { // Add a StaticBundle bound
    inner: T
}

fn my_register_bundle<T: StaticBundle>(world: &mut World) { // Replace Bundle with StaticBundle
    world.register_bundle::<T>();
}
```

The following APIs now require the `StaticBundle` trait instead of the `Bundle` trait:

- `World::register_bundle`, which has been renamed to `World::register_static_bundle`
- the `B` type parameter of `EntityRefExcept` and `EntityMutExcept`
- `EntityClonerBuilder::allow` and `EntityClonerBuilder::deny`
- `EntityCommands::clone_components` and `EntityCommands::move_components`
- `EntityWorldMut::clone_components` and `EntityWorldMut::move_components`
- the `B` type parameter of `IntoObserverSystem`, `Trigger`, `App::add_observer`, `World::add_observer`, `Observer::new`, `Commands::add_observer`, `EntityCommands::observe` and `EntityWorldMut::observe`
- `EntityWorldMut::remove_recursive` and `Commands::remove_recursive`
- `EntityCommands::remove`, `EntityCommands::remove_if`, `EntityCommands::try_remove_if`, `EntityCommands::try_remove`, `EntityCommands::remove_with_requires`, `EntityWorldMut::remove` and `EntityWorldMut::remove_with_requires`
- `EntityWorldMut::take`
- `EntityWorldMut::retain` and `EntityCommands::retain`

The following APIs now require the `StaticBundle` trait in addition to the `Bundle` trait:

- `Commands::spawn_batch`, `Commands::insert_batch`, `Commands::insert_batch_if_new`, `Commands::try_insert_batch`, `Commands::try_insert_batch_if_new`, `bevy::ecs::command::spawn_batch`, `bevy::ecs::command::insert_batch`, `World::spawn_batch`, `World::insert_batch`, `World::insert_batch_if_new`, `World::try_insert_batch` and `World::try_insert_batch_if_new`
- `ReflectBundle::new`, `impl FromType<B>` for `ReflectBundle` and `#[reflect(Bundle)]`
- `ExtractComponent::Out`

Moreover, some APIs have been renamed:

- `World::register_bundle` has been renamed to `World::register_static_bundle`
- the `DynamicBundle` trait has been renamed to `ComponentsFromBundle`
