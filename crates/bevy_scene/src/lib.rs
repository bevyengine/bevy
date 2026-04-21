#![expect(unsafe_code, reason = "Unsafe code is used to improve performance.")]
//! Composable scene authoring for Bevy, defined using the Bevy Scene Notation (BSN) format.
//!
//! Game entities rarely exist in isolation.
//! A 3D level might be made up of walls, floors, props and enemies.
//! A 2D character might need a distinct sprite entity for weapon, hat and boots.
//! A UI popup might need text and multiple buttons for accept, cancel, minimize and close actions.
//! Spawning these collections as individual, disjointed entities is tedious, error-prone, and hard to reuse.
//! A **scene** lets you describe a conceptual **object** — an entity, its components, children, and assets — once
//! and spawn it wherever you need it.
//!
//! Any scene system must overcome three challenges:
//!
//! - **Composability**: combining smaller scenes into larger ones without
//!   duplicating shared constants and setup code.
//! - **Granular overrides**: when reusing a scene, overriding *individual fields*
//!   on a component (like changing just a button's width) without having to respecify
//!   every other field on that component.
//! - **Asset integration**: referencing assets (meshes, textures, sounds) from within
//!   scenes without manually wiring up asset handles.
//!
//! This crate tackles all three, via [`Scene`] composition, [`Template`]-based
//! field-level patching, and automatic string-to-asset-handle resolution.
//!
//! The [`bsn!`] macro exposes these ideas, and makes the process of scene-authoring pleasant
//! by providing a terse syntax for defining [`Scene`]s inline.
//! This brevity is essential: making it easier to review and understand scenes at a glance,
//! resolve merge conflicts and keep file sizes under control.
//! The macro includes full Rust Analyzer support (autocomplete, go-to-definition, hover docs)!
//!
//! ## Quick Start
//!
//! Spawn entities in a [`Scene`] by calling [`World::spawn_scene`],
//! wrapping a call to the [`bsn!`] macro.
//!
//! ```
//! # use bevy_app::App;
//! # use bevy_scene::{prelude::*, ScenePlugin};
//! # use bevy_ecs::prelude::*;
//! # use bevy_asset::AssetPlugin;
//! # use bevy_app::TaskPoolPlugin;
//! # let mut app = App::new();
//! # app.add_plugins((
//! #     TaskPoolPlugin::default(),
//! #     AssetPlugin::default(),
//! #     ScenePlugin::default(),
//! # ));
//! # let world = app.world_mut();
//! #[derive(Component, Default, Clone)]
//! struct Score(usize);
//!
//! #[derive(Component, Default, Clone)]
//! struct Sword;
//!
//! #[derive(Component, Default, Clone)]
//! struct Shield;
//!
//! // #Player adds a `Name("Player")` component to the root entity.
//! // Children spawns two child entities: one with Sword, one with Shield.
//! world.spawn_scene(bsn! {
//!     // This names the entity "Player"
//!     #Player
//!     Score(0)
//!     Children [
//!         Sword,
//!         Shield,
//!     ]
//! });
//! ```
//!
//! ## Core Concepts
//!
//! - **[`Scene`]**: the main authoring type. A [`Scene`] is made up of [`Template`]s (one
//!   per component, plus any related entities such as children). Use the [`bsn!`] macro to
//!   create a scene.
//! - **[`SceneList`]**: a list of scenes, each producing a separate root entity.
//!   Think of it as the `Vec<Entity>` analogue to [`Scene`]'s single `Entity`.
//!   Use the [`bsn_list!`] macro to create a scene list.
//! - **[`Template`]**: a data description that produces a component value at spawn time,
//!   given access to the entity and world (e.g. resolving asset paths into handles, or
//!   named entity references into [`Entity`] ids). Each component in a [`Scene`] is
//!   represented by a [`Template`] rather than a concrete value.
//! - **[`FromTemplate`]**: associates a type with its canonical [`Template`]. Derive
//!   [`FromTemplate`] on your components to generate a companion template type where each
//!   field is independently set-or-unset, enabling per-field overrides.
//! - **[`ScenePatch`]**: an [`Asset`] that wraps a [`Scene`] together with its dependencies
//!   and its [`ResolvedScene`] (once loaded). You'll encounter this when using asset
//!   inheritance (`:"enemy.bsn"`), or when you want to treat a scene as a loadable,
//!   hot-reloadable prefab. See also [`ScenePatchInstance`] for applying one to an entity.
//! - **[`ResolvedScene`]**: the fully-resolved, ready-to-spawn result produced by resolving
//!   one or more [`Scene`]s. User code rarely interacts with this directly.
//!
//! ## Spawning Scenes
//!
//! There are two approaches to spawning scenes:
//!
//! - **Immediate**: [`World::spawn_scene`] and [`Commands::spawn_scene`]
//!   resolve and spawn in one step. Returns an error if any asset dependencies are not yet loaded.
//!   Use this when your scene has no asset dependencies.
//! - **Queued**: [`World::queue_spawn_scene`] and [`Commands::queue_spawn_scene`]
//!   register the scene's dependencies and wait for them to load before resolving and spawning.
//!   Use this when inheriting from asset-based scenes.
//!
//! In all cases, your `*_spawn_scene` method call should wrap an invocation of the [`bsn!`] macro,
//! or call a function which returns a [`Scene`].
//!
//! See the [`WorldSceneExt`], [`CommandsSceneExt`], [`EntityWorldMutSceneExt`], and
//! [`EntityCommandsSceneExt`] extension traits for the full set of scene-spawning APIs.
//!
//! ## Entity Hierarchies and Relationships
//!
//! Use `Children [scene1, scene2]` inside [`bsn!`] to spawn child entities.
//! Children (and entities within [`bsn_list!`]) are separated by commas;
//! add multiple components to the same entity by listing them without a comma:
//!
//! ```ignore
//! // Spawns one child entity
//! bsn! { #Parent Children [ComponentA ComponentB ComponentC] }
//!
//! // Spawns two child entities due to the added comma
//! bsn! { #Parent Children [ComponentA ComponentB, ComponentC] }
//!
//! // Spawns two child entities, but more clearly
//! bsn! { #Parent Children [(ComponentA ComponentB), ComponentC] }
//! ```
//!
//! These invocations can be nested to build deeper hierarchies.
//!
//! ```ignore
//! bsn! {
//!   #Parent,
//!   Children [
//!     #Child1
//!     ComponentA
//!     ComponentB,
//!     #Child2
//!     ComponentA
//!     Children [
//!        #GrandChild1
//!        ComponentA,
//!        #GrandChild2
//!        ComponentB
//!     ]
//!   ]
//! }
//! ```
//!
//! We can improve clarity at the cost of compactness through the careful use of newlines, parentheses and indentation:
//!
//! ```ignore
//! bsn! {
//!   #Parent,
//!   Children [
//!      (
//!        #Child1
//!        ComponentA
//!        ComponentB
//!      ),
//!      (
//!        #Child2
//!        ComponentA
//!        Children [
//!           #GrandChild1
//!           ComponentA,
//!           #GrandChild2
//!           ComponentB
//!        ]
//!      ),
//!   ]
//! }
//! ```
//!
//! This is fundamentally a stylistic choice: white space, Rust comments (`//` and `/* */`), and parentheses used in this way are ignored.
//!
//! The tools discussed here are not limited to [`Children`]: any [`RelationshipTarget`] type can be used the same way.
//!
//! ## Named Entity References
//!
//! The `#Name` syntax assigns a [`Name`] to an entity and registers it for cross-referencing.
//! Other entities in the same **scope** can refer to a named entity by its `#Name`,
//! receiving the resolved [`Entity`] id at spawn time.
//!
//! ### Scope rules
//!
//! Each [`bsn!`] invocation creates its own name scope. A name is visible to the root
//! entity, its children, and any deeper descendants — as long as the reference is written
//! in the same [`bsn!`] call. Composed or inherited scenes (via `my_scene()` or `:my_scene`)
//! each bring their own separate scope, so names do not leak across scene boundaries.
//!
//! If both a parent and a composed child define the same name (e.g. both use `#X`),
//! each scope's `#X` resolves to its own entity — there is no conflict or shadowing.
//!
//! In a [`bsn_list!`], all root entities share a single name scope, so sibling scenes
//! can reference each other by name. This is useful for wiring up relationships between
//! entities that are spawned together — for example, a group of UI panels where each
//! panel needs a relationship to its neighbor:
//!
//! ```ignore
//! fn linked_pair() -> impl SceneList {
//!     bsn_list![
//!         (#Left  Link(#Right)),
//!         (#Right Link(#Left)),
//!     ]
//! }
//! ```
//!
//! ## Composition and Patching
//!
//! When you insert a component in normal ECS code, the entire pre-existing value is replaced.
//! If a base scene sets `Button { width: 100, height: 300 }` and a caller wants to
//! change just `width`, ordinary component insertion would force them to respecify `height` too.
//!
//! **Patching** avoids this. When you write `Button { width: 200 }` in [`bsn!`], it creates
//! a *patch* that sets only the `width` field. Unmentioned fields keep their existing values
//! (from a parent scene, an earlier patch, or the type's defaults). Multiple patches to the
//! same component merge together rather than overwriting each other.
//!
//! To make a component patchable, derive [`FromTemplate`]. This generates a companion
//! [`Template`] type where every field is independently set-or-unset, which is what makes
//! partial patches possible.
//!
//! **Watch out:** types that implement `Clone + Default` (like the Quick Start example's
//! `Score`, `Sword`, and `Shield`) get a blanket [`Template`] impl automatically, so they
//! work inside [`bsn!`] without deriving [`FromTemplate`]. However, this blanket impl
//! always replaces the *entire* value — there is no field-level merging. If you want
//! per-field patching, you must derive [`FromTemplate`] explicitly.
//!
//! Deriving [`FromTemplate`] and [`Default`] on the same type is not allowed —
//! both would supply a [`FromTemplate`] impl and conflict.
//! You still have access to a default constructor of sorts though: the derive generates a companion
//! `YourTypeTemplate` struct that implements `Default`, so `YourTypeTemplate::default()` serves the same purpose.
//!
//! You compose scenes by writing functions that return `impl Scene` and calling them
//! inside [`bsn!`]:
//!
//! ```
//! # use bevy_app::App;
//! # use bevy_scene::{prelude::*, ScenePlugin};
//! # use bevy_ecs::prelude::*;
//! # use bevy_asset::AssetPlugin;
//! # use bevy_app::TaskPoolPlugin;
//! # let mut app = App::new();
//! # app.add_plugins((
//! #     TaskPoolPlugin::default(),
//! #     AssetPlugin::default(),
//! #     ScenePlugin::default(),
//! # ));
//! # let world = app.world_mut();
//! #[derive(Component, FromTemplate)]
//! struct Health {
//!  current: u32,
//!  max: u32
//! }
//!
//! fn enemy() -> impl Scene {
//!     bsn! { Health { current: 100, max: 100 } }
//! }
//!
//! // Compose `enemy()` and patch just the `max` field:
//! world.spawn_scene(bsn! {
//!     enemy()
//!     Health { max: 200 }
//! });
//! ```
//!
//! The spawned entity has `Health { current: 100, max: 200 }`: the `max` field is overridden
//! while `current` retains the value from `enemy()`. Tuples of [`Scene`]s also implement
//! [`Scene`], so patches from multiple sources merge into a single [`ResolvedScene`].
//!
//! For programmatic patching outside of [`bsn!`], see the [`PatchFromTemplate`] and
//! [`PatchTemplate`] traits.
//!
//! ## Scene Inheritance
//!
//! There are two ways to build on an existing scene: **inline composition** and **inheritance**.
//! Both let you patch fields on top of the parent, and both merge children from parent
//! and child (parent's children appear first). They differ in *when* the parent is resolved
//! and what kinds of parents they support.
//!
//! **Inline composition** (shown in the example above) calls a function directly inside
//! [`bsn!`]. The parent's templates are merged *unresolved* alongside the child's, and
//! everything resolves together in one pass:
//!
//! ```ignore
//! // Inline composition: call with parentheses, no `:`
//! bsn! {
//!     enemy()
//!     Health { max: 200 }
//! }
//! ```
//!
//! **Inheritance** uses the `:` prefix. The parent is **pre-resolved** — its templates are
//! fully flattened into a [`ResolvedScene`] *before* the child's patches apply on top:
//!
//! ```ignore
//! // Inheritance: `:` prefix, no parentheses or arguments
//! bsn! {
//!     :enemy
//!     Health { max: 200 }
//! }
//!
//! // Asset inheritance: `:` prefix with a string path to a ScenePatch asset
//! // DISCLAIMER: .bsn file format is not yet released!
//! bsn! {
//!    :"enemy.bsn"
//!    Health { max: 200 }
//! }
//! ```
//!
//! ### Which composition pattern should I choose?
//!
//! |                           | Inline composition           | Inheritance                           |
//! |---------------------------|------------------------------|---------------------------------------|
//! | Parent accepts parameters | Yes                          | No                                    |
//! | Parent from an asset file | No                           | Yes                                   |
//! | Resolution order          | Merged together in one pass  | Parent resolved first, then patched   |
//!
//! Use **inline composition** as the default — it's simpler and supports function parameters.
//! Reach for **inheritance** when the parent scene comes from an asset file,
//! or when you want the parent to be treated as a fully resolved, opaque base ("prefab").
//!
//! ## Loading Assets into Scenes
//!
//! Without the use of scenes, loading an asset requires referencing the [`AssetServer`] explicitly:
//!
//! ```ignore
//! let handle: Handle<Image> = asset_server.load("player.png");
//! commands.spawn(Sprite { image: handle, ..default() });
//! ```
//!
//! This can be particularly frustrating when defining helper functions,
//! requiring you to pipe asset handles or collections through multiple layers of function calls.
//!
//! In [`bsn!`], asset paths work directly as field values. When a component field is a
//! [`Handle<T>`], the [`bsn!`] macro accepts a string literal in its place. Under the hood,
//! this creates a [`HandleTemplate`] that calls [`AssetServer::load`] at resolve time.
//! If the asset has already been loaded, this returns the existing handle rather than
//! loading it again.
//!
//! ```ignore
//! commands.spawn_scene(bsn! {
//!     Sprite { image: "player.png" }
//! });
//! ```
//!
//! This also works for components you define yourself. Any `Handle<T>` field on a
//! [`FromTemplate`]-derived component automatically accepts asset path strings:
//!
//! ```ignore
//! #[derive(Component, FromTemplate)]
//! struct Icon {
//!     image: Handle<Image>,
//!     tint: Color,
//! }
//!
//! // "icon.png" is converted to a HandleTemplate<Image> via implicit .into()
//! commands.spawn_scene(bsn! {
//!     Icon { image: "icon.png", tint: Color::WHITE }
//! });
//! ```
//!
//! [`AssetServer`]: bevy_asset::AssetServer
//! [`AssetServer::load`]: bevy_asset::AssetServer::load
//! [`HandleTemplate`]: bevy_asset::HandleTemplate
//! [`Handle<T>`]: bevy_asset::Handle
//!
//! ## Observers
//!
//! Use [`on`] inside [`bsn!`] to attach an entity observer — a closure that runs when a
//! given [`EntityEvent`] fires on that entity. The first parameter's type determines
//! which event is observed. You can attach multiple observers to the same entity, and
//! the closure has full access to the ECS via system parameters:
//!
//! ```ignore
//! #[derive(EntityEvent)]
//! struct Damage(u32);
//!
//! #[derive(EntityEvent)]
//! struct Heal(u32);
//!
//! fn player() -> impl Scene {
//!     bsn! {
//!         Health { max: 100, current: 100 }
//!         // Each `on(...)` attaches a separate observer.
//!         on(|damage: On<Damage>, mut query: Query<&mut Health>| {
//!             let mut health = query.get_mut(damage.target()).unwrap();
//!             health.current = health.current.saturating_sub(damage.0);
//!         })
//!         on(|heal: On<Heal>, mut query: Query<&mut Health>| {
//!             let mut health = query.get_mut(heal.target()).unwrap();
//!             health.current = (health.current + heal.0).min(health.max);
//!         })
//!     }
//! }
//! ```
//!
//! This is useful for self-contained logic like click handlers, damage reactions,
//! or scripting-style triggers.
//! Closures passed to `on` work like any Rust closure:
//! you can use `move` and capture variables from the enclosing scope normally.
//!
//! ## Using Dynamic Expressions in Scenes
//!
//! The [`bsn!`] macro is not limited to static data. Because scene functions are plain
//! Rust functions, you can accept parameters and capture variables from the enclosing scope.
//! Use `{...}` (curly braces) anywhere a value is expected to embed an arbitrary Rust expression:
//!
//! ```ignore
//! fn enemy(hp: u32, name: &str) -> impl Scene {
//!    let sprite_path = name.to_string() + ".png";
//!
//!     bsn! {
//!         #{name}
//!         Health { current: {hp}, max: {hp} }
//!         Sprite { image: {sprite_path} }
//!     }
//! }
//!
//! // Call it like an ordinary Rust function
//! commands.spawn_scene(bsn! { enemy(200, "goblin.png") });
//! ```
//!
//! Braces are required when the macro would otherwise misparse the expression
//! and for complex expressions like `{hp * 2}`.
//! Variables used as positional or named fields (like `hp` above) also need braces.
//!
//! ### Dynamic children
//!
//! You can splice a runtime [`SceneList`] into a `Children [...]` block with `{...}`:
//!
//! ```ignore
//! fn container(contents: impl SceneList) -> impl Scene {
//!     bsn! {
//!         Children [
//!             #Header,
//!             {contents},
//!             #Footer,
//!         ]
//!     }
//! }
//!
//! let items = bsn_list![#A, #B, #C];
//! commands.spawn_scene(container(items));
//! ```
//!
//! ### Conditional components
//!
//! There is no `if`/`match` syntax inside the [`bsn!`] grammar, but you can embed
//! conditionals via `{...}` blocks or handle them outside the macro:
//!
//! ```ignore
//! fn unit(is_boss: bool) -> impl Scene {
//!     let hp = if is_boss { 500 } else { 100 };
//!     bsn! { Health { current: {hp}, max: {hp} } }
//! }
//! ```
//!
//! ### Expressions as scenes
//!
//! A `{...}` block can also represent a variable or
//! expression that implements [`Scene`].
//! This allows you to pass in scenes to helper functions,
//! allowing you to provide APIs based around partially complete scenes:
//!
//! ```ignore
//! fn unit_with_armor(unit_base: impl Scene) -> impl Scene {
//!     bsn! {
//!         {unit_base}
//!         Armor(50)
//!     }
//! }
//!
//! let my_unit = bsn! { Health { current: 100, max: 100 } };
//! commands.spawn_scene(unit_with_armor(my_unit));
//! ```
//!
//! ## .bsn Asset Format
//!
//! In future releases, Bevy intends to offer a `.bsn` asset format.
//! This would allow you to define your scenes on disk,
//! creating/modifying them in various authoring tools and using asset hot-reloading.
//!
//! This format is intended to have broad syntactic compatibility with the `bsn!` macro,
//! making it easy to port your content between both the macro and the asset form.
//!
//! Bevy does not currently have support for `.bsn` files:
//! for now, you should use existing non-Bevy asset formats like glTF,
//! search for ecosystem implementations or stick to `bsn!` macro calls.
//!
//! When planning, be aware that `.bsn` asset files, unlike `bsn!` macro calls,
//! will not support expressions or other dynamic features directly.
//!
//! [`Template`]: bevy_ecs::template::Template
//! [`FromTemplate`]: bevy_ecs::template::FromTemplate
//! [`Asset`]: bevy_asset::Asset
//! [`Entity`]: bevy_ecs::entity::Entity
//! [`RelationshipTarget`]: bevy_ecs::relationship::RelationshipTarget
//! [`EntityEvent`]: bevy_ecs::event::EntityEvent
//! [`Name`]: bevy_ecs::name::Name
//! [`World::spawn_scene`]: crate::WorldSceneExt::spawn_scene
//! [`Commands::spawn_scene`]: crate::CommandsSceneExt::spawn_scene
//! [`World::queue_spawn_scene`]: crate::WorldSceneExt::queue_spawn_scene
//! [`Commands::queue_spawn_scene`]: crate::CommandsSceneExt::queue_spawn_scene

/// The Bevy Scene prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    pub use crate::{
        bsn, bsn_list, on, template_value, CommandsSceneExt, EntityCommandsSceneExt,
        EntityWorldMutSceneExt, PatchFromTemplate, PatchTemplate, Scene, SceneList,
        ScenePatchInstance, SpawnListSystem, SpawnSystem, WorldSceneExt,
    };
}

/// Functionality used by the [`bsn!`] macro.
pub mod macro_utils;

extern crate alloc;

mod resolved_scene;
mod scene;
mod scene_list;
mod scene_patch;
mod spawn;
mod spawn_system;

pub use bevy_scene_macros::*;
pub use resolved_scene::*;
pub use scene::*;
pub use scene_list::*;
pub use scene_patch::*;
pub use spawn::*;
pub use spawn_system::*;

use bevy_app::{App, Plugin, SceneSpawnerSystems, SpawnScene};
use bevy_asset::AssetApp;
use bevy_ecs::prelude::*;

/// Adds support for spawning Bevy Scenes. See [`Scene`], [`SceneList`], [`ScenePatch`], and the [`bsn!`] macro for more information.
#[derive(Default)]
pub struct ScenePlugin;

impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QueuedScenes>()
            .init_resource::<WaitingScenes>()
            .init_asset::<ScenePatch>()
            .init_asset::<SceneListPatch>()
            .add_systems(
                SpawnScene,
                (resolve_scene_patches, spawn_queued)
                    .chain()
                    .in_set(SceneSpawnerSystems::SceneSpawn)
                    .after(SceneSpawnerSystems::WorldInstanceSpawn),
            )
            .add_observer(on_add_scene_patch_instance);
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as bevy_scene, ScenePlugin};
    use crate::{prelude::*, ScenePatch};
    use alloc::sync::Arc;
    use bevy_app::{App, TaskPoolPlugin};
    use bevy_asset::io::memory::{Dir, MemoryAssetReader};
    use bevy_asset::io::{AssetSourceBuilder, AssetSourceId};
    use bevy_asset::{Asset, AssetApp, AssetLoader, AssetPlugin, AssetServer, Assets, Handle};
    use bevy_ecs::lifecycle::HookContext;
    use bevy_ecs::prelude::*;
    use bevy_ecs::world::DeferredWorld;
    use bevy_reflect::TypePath;
    use std::path::Path;
    use std::sync::Mutex;

    fn test_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin::default(),
            ScenePlugin,
        ));
        app
    }

    #[test]
    fn inheritance_patching() {
        let mut app = test_app();
        let world = app.world_mut();

        #[derive(Component, FromTemplate)]
        struct Position {
            x: f32,
            y: f32,
            z: f32,
        }

        fn b() -> impl Scene {
            bsn! {
                :a
                Position { x: 1. }
                Children [ #Y ]
            }
        }

        fn a() -> impl Scene {
            bsn! {
                Position { y: 2. }
                Children [ #X ]
            }
        }

        let id = world.spawn_scene(b()).unwrap().id();
        let root = world.entity(id);

        let position = root.get::<Position>().unwrap();
        assert_eq!(position.x, 1.);
        assert_eq!(position.y, 2.);
        assert_eq!(position.z, 0.);

        let children = root.get::<Children>().unwrap();
        assert_eq!(children.len(), 2);

        let x = world.entity(children[0]);
        let name = x.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "X");

        let y = world.entity(children[1]);
        let name = y.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "Y");
    }

    #[test]
    fn loaded_asset_inheritance_patching() {
        #[derive(Component, FromTemplate)]
        struct Position {
            x: f32,
            y: f32,
            z: f32,
        }

        fn b() -> impl Scene {
            bsn! {
                :"a.bsn"
                Position { x: 1. }
                Children [ #Y ]
            }
        }

        fn a() -> impl Scene {
            bsn! {
                Position { y: 2. }
                Children [ #X ]
            }
        }

        let mut app = App::new();
        let dir = Dir::default();
        let dir_clone = dir.clone();
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSourceBuilder::new(move || {
                Box::new(MemoryAssetReader {
                    root: dir_clone.clone(),
                })
            }),
        );
        app.add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin::default(),
            ScenePlugin,
        ));

        app.finish();
        app.cleanup();
        // Create a fake loader to act as a ScenePatch loaded from a file.
        app.register_asset_loader(FakeSceneLoader);

        #[derive(TypePath)]
        struct FakeSceneLoader;

        impl AssetLoader for FakeSceneLoader {
            type Asset = ScenePatch;
            type Error = std::io::Error;
            type Settings = ();

            async fn load(
                &self,
                _reader: &mut dyn bevy_asset::io::Reader,
                _settings: &Self::Settings,
                load_context: &mut bevy_asset::LoadContext<'_>,
            ) -> Result<Self::Asset, Self::Error> {
                Ok(ScenePatch::load_with(load_context, a()))
            }
        }

        // Insert an asset that the fake loader can fake read.
        dir.insert_asset_text(Path::new("a.bsn"), "");
        let asset_server = app.world().resource::<AssetServer>().clone();
        let handle = asset_server.load("a.bsn");
        assert!(app.world().get_resource::<Assets<ScenePatch>>().is_some());
        run_app_until(&mut app, || asset_server.is_loaded(&handle));
        let patch = app
            .world()
            .resource::<Assets<ScenePatch>>()
            .get(&handle)
            .unwrap();
        assert!(patch.resolved.is_some());

        let world = app.world_mut();
        let id = world.spawn_scene(b()).unwrap().id();
        let root = world.entity(id);

        let position = root.get::<Position>().unwrap();
        assert_eq!(position.x, 1.);
        assert_eq!(position.y, 2.);
        assert_eq!(position.z, 0.);

        let children = root.get::<Children>().unwrap();
        assert_eq!(children.len(), 2);

        let x = world.entity(children[0]);
        let name = x.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "X");

        let y = world.entity(children[1]);
        let name = y.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "Y");
    }

    #[test]
    fn inline_scene_patching() {
        let mut app = test_app();
        let world = app.world_mut();

        #[derive(Component, FromTemplate)]
        struct Position {
            x: f32,
            y: f32,
            z: f32,
        }

        fn b() -> impl Scene {
            bsn! {
                a()
                Position { x: 1. }
                Children [ #Y ]
            }
        }

        fn a() -> impl Scene {
            bsn! {
                Position { y: 2. }
                Children [ #X ]
            }
        }

        let id = world.spawn_scene(b()).unwrap().id();
        let root = world.entity(id);

        let position = root.get::<Position>().unwrap();
        assert_eq!(position.x, 1.);
        assert_eq!(position.y, 2.);
        assert_eq!(position.z, 0.);

        let children = root.get::<Children>().unwrap();
        assert_eq!(children.len(), 2);

        let x = world.entity(children[0]);
        let name = x.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "X");

        let y = world.entity(children[1]);
        let name = y.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "Y");
    }

    #[test]
    fn hierarchy() {
        let mut app = test_app();
        let world = app.world_mut();

        fn scene() -> impl Scene {
            bsn! {
                #A
                Children [
                    (
                        #B
                        Children [
                            #X
                        ]
                    ),
                    (
                        #C
                        Children [
                            #Y
                        ]
                    )
                ]
            }
        }

        let id = world.spawn_scene(scene()).unwrap().id();

        let a = world.entity(id);
        let name = a.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "A");

        let children = a.get::<Children>().unwrap();
        assert_eq!(children.len(), 2);

        let b = world.entity(children[0]);
        let c = world.entity(children[1]);

        let name = b.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "B");

        let name = c.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "C");

        let children = b.get::<Children>().unwrap();
        assert_eq!(children.len(), 1);
        let x = world.entity(children[0]);
        let name = x.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "X");

        let children = c.get::<Children>().unwrap();
        assert_eq!(children.len(), 1);
        let y = world.entity(children[0]);
        let name = y.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "Y");
    }

    #[test]
    fn constant_values() {
        let mut app = test_app();
        let world = app.world_mut();

        const X_AXIS: usize = 1;
        const XAXIS: usize = 2;

        #[derive(Component, FromTemplate)]
        struct Value(usize);

        fn x_axis() -> impl Scene {
            bsn! {Value(X_AXIS)}
        }

        fn xaxis() -> impl Scene {
            bsn! {Value(XAXIS)}
        }

        let entity = world.spawn_scene(x_axis()).unwrap();
        assert_eq!(entity.get::<Value>().unwrap().0, 1);

        let entity = world.spawn_scene(xaxis()).unwrap();
        assert_eq!(entity.get::<Value>().unwrap().0, 2);
    }

    #[derive(Component, FromTemplate)]
    struct Reference(Entity);

    #[test]
    fn bsn_name_references() {
        let mut app = test_app();
        let world = app.world_mut();

        fn a() -> impl Scene {
            bsn! {
                #X
                Children [
                    (:b Reference(#X))
                ]
            }
        }

        fn b() -> impl Scene {
            let inline = bsn! {#Y Reference(#Y) Children [ Reference(#Y)] };
            bsn! {
                #X
                Children [
                    Reference(#X),
                    (inline Reference(#X)),
                ]
            }
        }

        let id = world.spawn_scene(a()).unwrap().id();

        let a = world.entity(id);
        let name = a.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "X");

        let children = a.get::<Children>().unwrap();
        assert_eq!(children.len(), 1);

        let b = world.entity(children[0]);
        let reference = b.get::<Reference>().unwrap();
        assert_eq!(reference.0, id);

        let b_name = b.get::<Name>().unwrap();
        assert_eq!(b_name.as_str(), "X");

        let grandchildren = b.get::<Children>().unwrap();
        assert_eq!(grandchildren.len(), 2);

        let grandchild = world.entity(grandchildren[0]);
        assert_eq!(grandchild.get::<Reference>().unwrap().0, b.id());

        let grandchild = world.entity(grandchildren[1]);
        assert_eq!(grandchild.get::<Reference>().unwrap().0, b.id());
        assert_eq!(grandchild.get::<Name>().unwrap().as_str(), "Y");

        assert_eq!(
            grandchild.id(),
            world
                .entity(grandchild.get::<Children>().unwrap()[0])
                .get::<Reference>()
                .unwrap()
                .0
        );
    }

    #[test]
    fn bsn_list_name_references() {
        let mut app = test_app();
        let world = app.world_mut();

        fn b() -> impl Scene {
            bsn! {
                #Z
                Children [
                    Reference(#Z)
                ]
            }
        }

        fn a() -> impl SceneList {
            bsn_list![
                (
                    #X
                    Reference(#Y)
                    Children [
                        (#Z Reference(#X))
                    ]

                ),
                (
                    #Y
                    Reference(#X)
                    Children [
                        Reference(#Y)
                    ]
                ),
                (:b #Z)
            ]
        }

        let ids = world.spawn_scene_list(a()).unwrap();
        assert_eq!(ids.len(), 3);

        let e0 = world.entity(ids[0]);
        let name = e0.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "X");
        let reference = e0.get::<Reference>().unwrap();
        assert_eq!(reference.0, ids[1]);

        let child0 = e0.get::<Children>().unwrap()[0];
        let reference = world.entity(child0).get::<Reference>().unwrap();
        assert_eq!(reference.0, ids[0]);

        let e1 = world.entity(ids[1]);
        let name = e1.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "Y");

        let reference = e1.get::<Reference>().unwrap();
        assert_eq!(reference.0, ids[0]);

        let child0 = e1.get::<Children>().unwrap()[0];
        let reference = world.entity(child0).get::<Reference>().unwrap();
        assert_eq!(reference.0, ids[1]);

        let e2 = world.entity(ids[2]);
        let name = e2.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "Z");
        let child0 = e2.get::<Children>().unwrap()[0];
        let reference = world.entity(child0).get::<Reference>().unwrap();
        assert_eq!(reference.0, ids[2]);
    }

    #[test]
    fn on_template() {
        #[derive(Resource)]
        struct Exploded(Option<Entity>);

        #[derive(EntityEvent)]
        struct Explode(Entity);

        let mut app = test_app();
        let world = app.world_mut();
        world.insert_resource(Exploded(None));

        fn scene() -> impl Scene {
            bsn! {
                on(|explode: On<Explode>, mut exploded: ResMut<Exploded>|{
                    exploded.0 = Some(explode.0);
                })
            }
        }

        let id = world.spawn_scene(scene()).unwrap().id();
        world.trigger(Explode(id));
        let exploded = world.resource::<Exploded>();
        assert_eq!(exploded.0, Some(id));
    }

    #[test]
    fn enum_patching() {
        let mut app = test_app();
        let world = app.world_mut();

        #[derive(Component, FromTemplate, PartialEq, Eq, Debug)]
        enum Foo {
            #[default]
            Bar {
                x: u32,
                y: u32,
                z: u32,
            },
            Baz(usize),
            Qux,
        }

        fn a() -> impl Scene {
            bsn! {
                Foo::Baz(10)
            }
        }

        fn b() -> impl Scene {
            bsn! {
                a()
                Foo::Bar { x: 1 }
            }
        }

        fn c() -> impl Scene {
            bsn! {
                b()
                Foo::Bar { y: 2 }
            }
        }

        fn d() -> impl Scene {
            bsn! {
                c()
                Foo::Qux
            }
        }

        let id = world.spawn_scene(c()).unwrap().id();
        let root = world.entity(id);

        let foo = root.get::<Foo>().unwrap();
        assert_eq!(Foo::Bar { x: 1, y: 2, z: 0 }, *foo);

        let id = world.spawn_scene(a()).unwrap().id();
        let root = world.entity(id);

        let foo = root.get::<Foo>().unwrap();
        assert_eq!(Foo::Baz(10), *foo);

        let id = world.spawn_scene(d()).unwrap().id();
        let root = world.entity(id);
        let foo = root.get::<Foo>().unwrap();
        assert_eq!(Foo::Qux, *foo);
    }

    #[test]
    fn struct_patching() {
        let mut app = test_app();
        let world = app.world_mut();

        #[derive(Component, FromTemplate, PartialEq, Eq, Debug)]
        struct Foo {
            x: u32,
            y: u32,
            z: u32,
            nested: Bar,
        }

        #[derive(Component, FromTemplate, PartialEq, Eq, Debug)]
        struct Bar(usize, usize, usize);

        fn a() -> impl Scene {
            bsn! {
                Foo {
                    x: 1,
                    nested: Bar(1, 1),
                }
            }
        }

        fn b() -> impl Scene {
            bsn! {
                a()
                Foo {
                    y: 2,
                    nested: Bar(2),
                }
            }
        }

        let id = world.spawn_scene(b()).unwrap().id();
        let root = world.entity(id);

        let foo = root.get::<Foo>().unwrap();
        assert_eq!(
            *foo,
            Foo {
                x: 1,
                y: 2,
                z: 0,
                nested: Bar(2, 1, 0)
            }
        );
    }

    #[test]
    fn handle_template() {
        let mut app = test_app();
        app.init_asset::<Image>();

        #[derive(Asset, TypePath)]
        struct Image;

        let handle = app.world().resource::<AssetServer>().load("image.png");
        let world = app.world_mut();

        #[derive(Component, FromTemplate, PartialEq, Eq, Debug)]
        struct Sprite(Handle<Image>);

        fn scene() -> impl Scene {
            bsn! {
                Sprite("image.png")
            }
        }

        let id = world.spawn_scene(scene()).unwrap().id();
        let root = world.entity(id);

        let sprite = root.get::<Sprite>().unwrap();
        assert_eq!(sprite.0, handle);
    }

    #[test]
    fn scene_list_children() {
        let mut app = test_app();
        let world = app.world_mut();

        fn root(children: impl SceneList) -> impl Scene {
            bsn! {
                Children [
                    #A,
                    {children},
                    #D
                ]
            }
        }

        let children = bsn_list! [
            #B,
            #C,
        ];

        let id = world.spawn_scene(root(children)).unwrap().id();
        let root = world.entity(id);
        let children = root.get::<Children>().unwrap();
        let a = world.entity(children[0]).get::<Name>().unwrap();
        let b = world.entity(children[1]).get::<Name>().unwrap();
        let c = world.entity(children[2]).get::<Name>().unwrap();
        let d = world.entity(children[3]).get::<Name>().unwrap();
        assert_eq!(a.as_str(), "A");
        assert_eq!(b.as_str(), "B");
        assert_eq!(c.as_str(), "C");
        assert_eq!(d.as_str(), "D");
    }

    #[test]
    fn generic_patching() {
        let mut app = test_app();
        let world = app.world_mut();

        #[derive(Component, FromTemplate, PartialEq, Eq, Debug)]
        struct Foo<T: FromTemplate<Template: Default + Template<Output = T>>> {
            value: T,
            number: u32,
        }

        #[derive(Component, FromTemplate, PartialEq, Eq, Debug)]
        struct Position {
            x: u32,
            y: u32,
            z: u32,
        }

        fn a() -> impl Scene {
            bsn! {
                Foo::<Position> {
                    value: Position { x: 1 }
                }
            }
        }

        fn b() -> impl Scene {
            bsn! {
                a()
                Foo::<Position> {
                    value: Position { y: 2 },
                    number: 10,
                }
            }
        }

        let id = world.spawn_scene(b()).unwrap().id();
        let root = world.entity(id);

        let foo = root.get::<Foo<Position>>().unwrap();
        assert_eq!(
            *foo,
            Foo {
                value: Position { x: 1, y: 2, z: 0 },
                number: 10
            }
        );
    }

    #[test]
    fn empty_scene_expressions() {
        let mut app = test_app();
        let world = app.world_mut();
        fn a() -> impl Scene {
            bsn! {
                {}
            }
        }
        world.spawn_scene(a()).unwrap();
    }

    #[test]
    fn closures_in_bsn() {
        #[derive(Resource, Default)]
        struct TotalHealed(u32);

        #[derive(EntityEvent)]
        struct Heal(Entity);

        let mut app = test_app();
        let world = app.world_mut();
        world.init_resource::<TotalHealed>();

        fn non_move_scene() -> impl Scene {
            bsn! {
                on(|_: On<Heal>, mut healed: ResMut<TotalHealed>| {
                    healed.0 += 1;
                })
            }
        }

        let id = world.spawn_scene(non_move_scene()).unwrap().id();
        world.trigger(Heal(id));
        assert_eq!(world.resource::<TotalHealed>().0, 1);
        world.resource_mut::<TotalHealed>().0 = 0;

        fn move_scene(bonus: u32) -> impl Scene {
            bsn! {
                on(move |_: On<Heal>, mut healed: ResMut<TotalHealed>| {
                    healed.0 += bonus;
                })
            }
        }

        let id = world.spawn_scene(move_scene(42)).unwrap().id();
        world.trigger(Heal(id));
        assert_eq!(world.resource::<TotalHealed>().0, 42);
    }

    #[test]
    fn comments_in_bsn() {
        let mut app = test_app();
        let world = app.world_mut();

        #[derive(Component, Clone, Default)]
        struct Marker;

        fn yappy() -> impl Scene {
            bsn! {
                // Look ma, a comment!
                #MyName
                /*
                   Wow, a block comment now?
                */
                Marker
            }
        }
        world.spawn_scene(yappy()).unwrap();
    }

    #[test]
    fn bsn_entry_can_surpass_tuple_limit() {
        let _ = bsn! {
            Name
            Name
            Name
            Name
            Name
            Name
            Name
            Name
            Name
            Name
            Name
            Name
            Name
            Name
        };
    }

    #[derive(Component, Default)]
    struct Fail;
    impl FromTemplate for Fail {
        type Template = Fail;
    }
    impl Template for Fail {
        type Output = Fail;

        fn build_template(
            &self,
            _context: &mut bevy_ecs::template::TemplateContext,
        ) -> Result<Self::Output> {
            Err(BevyError::error("fail!"))
        }

        fn clone_template(&self) -> Self {
            todo!()
        }
    }

    #[test]
    fn queue_spawn_scene_during_spawn() {
        #[derive(Component, Default, Clone)]
        #[component(on_insert)]
        struct SpawnOnInsert;

        impl SpawnOnInsert {
            fn on_insert(mut world: DeferredWorld, _context: HookContext) {
                world.commands().queue_spawn_scene(scene2());
            }
        }

        fn scene1() -> impl Scene {
            bsn!(SpawnOnInsert)
        }

        fn scene2() -> impl Scene {
            bsn!(#Name)
        }

        let mut app = test_app();
        let world = app.world_mut();
        world.queue_spawn_scene(scene1());

        app.update();
    }

    #[test]
    fn drop_is_called_for_uninserted_components() {
        #[derive(Component, FromTemplate)]
        struct DropTracker(Option<Arc<Mutex<usize>>>);

        impl Drop for DropTracker {
            fn drop(&mut self) {
                if let Some(count) = &mut self.0 {
                    *count.lock().unwrap() += 1;
                }
            }
        }

        let mut app = test_app();
        let world = app.world_mut();
        let count_arc = Arc::new(Mutex::new(0));
        let count = Some(count_arc.clone());
        let scene = bsn! {
            DropTracker({count.clone()})
            Fail
        };
        let result = world.spawn_scene(scene);
        assert!(result.is_err());
        assert_eq!(1, *count_arc.lock().unwrap());
    }

    #[test]
    fn despawn_on_failed_spawn() {
        let mut app = test_app();
        let world = app.world_mut();
        let current_entities = world.entities().len();
        let result = world.spawn_scene(bsn! {
           Fail
        });
        assert!(result.is_err());
        assert_eq!(current_entities, world.entities().len());
    }

    fn run_app_until(app: &mut App, mut predicate: impl FnMut() -> bool) {
        const LARGE_ITERATION_COUNT: usize = 10000;
        for _ in 0..LARGE_ITERATION_COUNT {
            app.update();
            if predicate() {
                return;
            }
        }

        panic!("Ran out of loops to return `Some` from `predicate`");
    }

    #[test]
    fn inheritance_with_generics() {
        #[derive(Component, FromTemplate, PartialEq, Eq, Debug)]
        struct Foo<T: FromTemplate<Template: Default + Template<Output = T>>> {
            value: T,
            number: u32,
        }

        fn b() -> impl Scene {
            bsn! {
                :a::<0, i32>
                Children [ #Y ]
            }
        }

        fn a<
            const A: u32,
            T: 'static
                + Send
                + Sync
                + FromTemplate<Template: Send + Sync + Default + Template<Output = T>>,
        >() -> impl Scene {
            bsn! {
                Foo<T>{
                    number: A
                }
            }
        }

        b();
    }
}
