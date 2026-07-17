#![expect(unsafe_code, reason = "Unsafe code is used to improve performance.")]
//! Composable scene authoring for Bevy, defined using the Bevy Scene Notation (BSN) format.
//!
//! Game entities rarely exist in isolation.
//! A 3D level might be made up of walls, floors, props and enemies.
//! A 2D character might need a distinct sprite entity for weapon, hat and boots.
//! A UI popup might need text and multiple buttons for accept, cancel, minimize and close actions.
//! Spawning these collections as individual, disjointed entities is tedious, error-prone, and hard to reuse.
//! A **scene** lets you describe a conceptual **object**, made of an entity, its components, children, and assets, once
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
//! The macro includes best-effort Rust-Analyzer support. Autocomplete, go-to-definition, and hover docs
//! should work inside the macros, and this effort should transfer over correctly to other LSPs!
//!
//! ## BSN syntax reference
//!
//! For a quick rundown on how to read and write BSN syntax, see the docs for [`bsn!`].
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
//!     #Player // This names the entity "Player"
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
//! - **[`Scene`]**: Describes what a spawned [`Entity`] should look like, created using [`bsn!`] or,
//!   in the future, `.bsn` asset files. Conceptually, a [`Scene`] contains a list of "entries" to apply to an [`Entity`].
//! - **[`SceneList`]**: A list of scenes, returned by [`bsn_list!`].
//!   Each [`Scene`] in the list produces one [`Entity`].
//! - **Scene Composition**: Composition works by including scenes in other scenes. The included scenes "entries" will be
//!   treated as if they were written in the outer scene.
//! - **[`Template`]**: A [`Template`] is something that, given a spawn context (target [`Entity`], [`World`], etc), can produce some output. Think of it
//!   as a "superpowered ECS-aware constructor" for a type. In the context of scenes, [`Template`]s are used to produce [`Component`]s and [`Bundle`]s. This
//!   enables defining scenes without needing to pass in a bunch of their dependencies (such as assets). The [`FromTemplate`] trait is used to associate some
//!   final output type (ex: a [`Component`]) with a canonical [`Template`] that produces it. [`FromTemplate`] / [`Template`] is automatically implemented for
//!   types that implement [`Default`] + [`Clone`], which is generally preferred. You should manually derive [`FromTemplate`] when a type needs custom template logic
//!   (ex: one of its fields is an "asset handle", which has custom template logic).
//! - **[`RelatedScenes`]**: These add a [`SceneList`] as related to this [`Scene`] by a specific [`relationship`](bevy_ecs::relationship::Relationship).
//!   This kind of change is added to the [`Scene`] by specifying a [`RelationshipTarget`] component like [`Children`], followed by a [`SceneList`].
//!
//! ## Spawning Scenes
//!
//! There are two approaches to spawning scenes:
//!
//! - **Immediate**: [`World::spawn_scene`] and [`Commands::spawn_scene`]
//!   resolve and spawn in one step.
//!   Returns an error if any asset dependencies are not yet loaded.
//! - **Queued**: [`World::queue_spawn_scene`] and [`Commands::queue_spawn_scene`]
//!   register the scene's dependencies and wait for them to load before resolving and spawning.
//!   When the dependencies are loaded (or there are no dependencies), the scene will spawn during
//!   that frame's [`SpawnScene`] schedule, between [`Update`](bevy_app::Update) and
//!   [`PostUpdate`](bevy_app::PostUpdate).
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
//! [`Children`] (and entities within [`bsn_list!`]) are separated by commas;
//! add multiple components to the same entity by listing them without a comma:
//!
//! ```ignore
//! // Spawns one child entity with components A, B and C
//! bsn! { #Parent Children [A B C] }
//!
//! // Spawns two child entities, one with A and B, the other with C, due to the added comma
//! bsn! { #Parent Children [A B, C] }
//!
//! // Spawns two child entities, but more clearly separated due to parentheses.
//! bsn! { #Parent Children [(A B), C] }
//! ```
//!
//! These invocations can be nested to build deeper hierarchies.
//!
//! ```ignore
//! bsn! {
//!   #Parent
//!   Children [
//!     #Child1 SomeComponent,
//!     #Child2
//!     SomeComponent
//!     Children [
//!        #GrandChild1 SomeComponent,
//!        #GrandChild2
//!     ]
//!   ]
//! }
//! ```
//!
//! We can improve clarity at the cost of compactness through the careful use of newlines, parentheses and indentation:
//!
//! ```ignore
//! bsn! {
//!   #Parent
//!   Children [
//!      (
//!        #Child1
//!        SomeComponent
//!      ),
//!      (
//!        #Child2
//!        Children [
//!           (
//!             #GrandChild1
//!             SomeComponent
//!           ),
//!           (
//!             #GrandChild2
//!           )
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
//! The `#Name` syntax assigns a [`Name`] to an entity and registers it for cross-referencing within the same macro invocation.
//! Within the same bsn! invocation / scope, it is possible to reference an entity by its `#Name`, generating an [`EntityTemplate`] which ultimately resolves to an [`Entity`]:
//!
//! ```ignore
//! bsn! {
//!     #Name
//!     my_scene(#Name)
//!     ComponentA(#Name)
//!     ComponentB { entity: #Name }
//!     Children [
//!         ComponentC(#Name)
//!     ]
//! }
//! ```
//!
//! Notice that the "child entity" was able to access the parent entity via `#Name`. It is also possible for ancestors to access
//! their descendants:
//!
//! ```ignore
//! bsn! {
//!     #Root
//!     Children [
//!         Reference(#Root)
//!     ]
//! }
//! ```
//!
//! Using `#Name` as a value in [`bsn!`] will result in an [`EntityTemplate`], which is a [`Template`] that resolves to an [`Entity`]
//! [`Component`]s with [`Entity`] fields should generally derive [`FromTemplate`], because [`Entity`] uses [`FromTemplate`] to map to [`EntityTemplate`].
//!
//! ### Scope rules
//!
//! Each [`bsn!`] invocation creates its own name scope. A name is visible to the root
//! entity, its children, and any deeper descendants in the same call. The reverse is also
//! true: descendants can "look up" the hierarchy.
//! Composed scenes (via `my_scene(#Name)`) or [`SceneComponents`](SceneComponent)
//! each contain their own [`bsn!`] invocation and therefore their own scope,
//! so re-using the same name across multiple different scenes is fine.
//! However, the results of a named entity reference, the [`EntityTemplate`],
//! can be passed to other scenes. It is valid only during the spawning of a scene.
//! That means [`Components`](Component) should never store [`EntityTemplate`] fields,
//! they should store the resolved [`Entity`] instead and
//! derive [`FromTemplate`] to convert [`EntityTemplate`] automatically.
//!
//! If both a parent and a composed child define the same name (e.g. both use `#X`),
//! each scope's `#X` resolves to its own entity, avoiding conflicts or potentially unintuitive shadowing.
//!
//! In a [`bsn_list!`], all root entities share a single name scope, so sibling scenes
//! can reference each other by name. This is useful for wiring up relationships between
//! entities that are spawned together. For example, a group of UI panels where each
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
//! ### Dynamic Name Values and Entity References
//!
//! `#SomeName` syntax will set the value of the [`Name`] component to `Name("SomeName")`, and  make the entity reference-able in [`bsn!`]. `#Name` syntax is _always_ scoped and
//! doesn't support "dynamic" names. If you would like to _both_ reference an entity in [`bsn!`] _and_ provide a dynamic name, you can do this:
//!
//! ```ignore
//! let i = 0;
//! bsn! {
//!   #Root
//!   Name({format!("Entity {i}")})
//!   Children [
//!     Reference(#Root)
//!   ]
//! }
//! ```
//! Adding `Name("desired name")` after the `#SomeName` reference will patch over the `Name` component created by the reference to give it a custom name.
//!
//! ## Patching
//!
//! When you insert a component into an [`Entity`] in normal ECS code, the entire pre-existing value is replaced.
//! If a scene sets `Button { width: 100, height: 300 }` and a caller wants to
//! change just `width`, ordinary component insertion would force them to respecify `height` too.
//!
//! **Patching** avoids this. When you write `Button { width: 200 }` in [`bsn!`], it creates
//! a *patch* that sets only the `width` field. Unmentioned fields keep their existing values
//! (from a included scene, an earlier patch, or the type's defaults). Multiple patches to the
//! same component and its values are applied in order, only overwriting the fields they changed.
//!
//! The following scenes all end up with a button which is 200 wide and 300 high.
//! ```ignore
//! impl Default for Button {
//!     fn default() -> Self {
//!         Button { width: 100, height: 300 }
//!     }
//! }
//!
//! bsn! { Button { width: 200, height: 300 } } // fully specified
//! bsn! { Button { width: 200 } }              // only changing width, height defaults to 300
//!
//! bsn! {
//!     Button                 // inserts defaults
//!     Button { width: 200 }  // changes width
//!     Button { height: 300 } // changes height
//! }
//! ```
//!
//! ### Required Traits
//!
//! To make a component available in [`bsn!`], derive either [`Default`] + [`Clone`], or [`FromTemplate`].
//! Both support patching: unmentioned fields keep their values from earlier patches or the
//! type's defaults, and multiple patches merge rather than overwrite.
//!
//! The distinction is about what values a field can hold at spawn time:
//!
//! - **[`Clone`] + [`Default`]** (e.g. `#[derive(Component, Default, Clone)]`): covers the simple case, and should be your default choice.
//! - **[`FromTemplate`]** (e.g. `#[derive(Component, FromTemplate)]`) is needed when a field requires spawn-time context.
//!   Examples include [`Handle<T>`] fields which need [`AssetServer`] to resolve asset paths, or [`Entity`]
//!   fields which resolve [`EntityTemplate`]s from named entity references. If any of your fields' types
//!   implement [`FromTemplate`] manually / have custom template logic, you should derive it for the parent type as well if you want your type
//!   to use that logic.
//!
//! Deriving [`FromTemplate`] and [`Default`] on the same type is not allowed, as both would supply a [`FromTemplate`] impl and conflict.
//! [`FromTemplate`] derivers still have access to a default constructor of sorts though: the derive generates a companion struct
//! for `YourType` named `YourTypeTemplate` which implements `Default`, so `YourTypeTemplate::default()` serves the same purpose.
//!
//! #### Enums in bsn
//!
//! Enums are special-cased to allow for better implicit defaults: [`bsn!`] requires that enums have defaults for all variant arms, not just the type as a whole.
//!
//! When [`bsn!`] encounters a Enum, it will try to get the default value for the variant using static methods like `default_{variant_lower}`.
//! To help with setting up these methods, theres a pseudo-`derive` called [`VariantDefaults`](bevy_ecs::VariantDefaults).
//! It works like a normal `derive` macro, but without a matching Trait. It just generates a impl block with the `default_{variant_lower}` static methods.
//!
//! Deriving [`FromTemplate`] also implies/works like [`VariantDefaults`](bevy_ecs::VariantDefaults).
//!
//! ## Composition
//!
//! Composition relies on patching to work nicely, allowing you to include other scenes in the current ones.
//! All of their patches will be applied at the position they're included.
//!
//! Example:
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
//! // Include `enemy()` and patch just the `max` field:
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
//! ## Scene Caching
//!
//! <div class="warning">
//!
//! Note: Caching is currently only implemented for scene assets. It hasn't yet been wired up for "function scenes" or [`SceneComponent`]s. Attempting to use
//! it in those cases will result in a compile error.
//!
//! </div>
//!
//! Scenes can be cached, improving performance. Since this can change the semantics in some cases, this requires an explicit opt-in.
//! Caching works by resolving the included scene and storing the resulting [`ResolvedScene`] for future use. When the outer scene is spawned again,
//! it will not need to resolve the included scene again, instead patching on top of the cached version (using copy-on-write semantics for each [`Template`]).
//! This means caching can only be used if the scene is the first scene entry.
//!
//! This scene includes an uncached "enemy" scene:
//! ```ignore
//! bsn! {
//!     enemy()
//!     Health { max: 200 }
//! }
//! ```
//!
//! This scene caches the "enemy" scene by adding the  `:` prefix (however caching scene functions like this is not currently supported)
//! ```ignore
//! bsn! {
//!     :enemy
//!     Health { max: 200 }
//! }
//! ```
//!
//! Scene assets always need to be cached using the `:` prefix.
//! Note that the `.bsn` file format is not yet released. (This already works, assuming theres a loader for the asset format)
//! ```ignore
//! bsn! {
//!    :"enemy.bsn"
//!    Health { max: 200 }
//! }
//! ```
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
//! This can be particularly frustrating when defining helper functions for spawning entities,
//! which require you to pass [`AssetServer`] or handles through multiple layers of function calls.
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
//! A [`Component`] must also derive [`FromTemplate`] to accept asset paths:
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
//! Use [`on()`](on) inside [`bsn!`] to attach an entity [`Observer`]. Entity observers are closures or
//! functions which fire when a given [`EntityEvent`] is triggered and targets this entity.
//! The first parameter's type determines which event is observed. Multiple observers can be added to
//! the same entity, and the observer has full access to the ECS via [system parameters](bevy_ecs::system#system-parameter-list):
//!
//! ```ignore
//! #[derive(EntityEvent)]
//! struct Damage {
//!     entity: Entity,
//!     amount: u32,
//! }
//!
//! #[derive(EntityEvent)]
//! struct Heal {
//!     entity: Entity,
//!     amount: u32,
//! }
//!
//! fn player() -> impl Scene {
//!     bsn! {
//!         Health { max: 100, current: 100 }
//!         // Each `on(...)` attaches a separate observer.
//!         on(|damage: On<Damage>, mut query: Query<&mut Health>| {
//!             let mut health = query.get_mut(damage.entity).unwrap();
//!             health.current = health.current.saturating_sub(damage.amount);
//!         })
//!         on(on_heal)
//!     }
//! }
//!
//! fn on_heal(heal: On<Heal>, query: Query<&mut Health>){
//!     let mut health = query.get_mut(heal.entity).unwrap();
//!     health.current = (health.current + heal.amount).min(health.max);
//! }
//! ```
//!
//! This is useful for self-contained logic like click handlers, damage reactions,
//! or scripting-style triggers.
//! Closures passed to [`on`] work like any Rust closure:
//! you can use [`move`](https://doc.rust-lang.org/std/keyword.move.html) and capture variables from the enclosing scope normally.
//!
//! ## Using Dynamic Expressions in Scenes
//!
//! The [`bsn!`] macro is not limited to static data. Because scene functions are plain
//! Rust functions, you can accept parameters and capture variables from the enclosing scope.
//! Use `{...}` (curly braces) anywhere a value is expected to embed an arbitrary Rust expression:
//!
//! ```ignore
//! fn enemy(hp: u32, name: &str) -> impl Scene {
//!     let name_string = name.to_string();
//!     bsn! {
//!         #{name}
//!         Health { current: {hp / 2}, max: hp }
//!         Sprite { image: {name_string + ".png"} }
//!     }
//! }
//!
//! // Call it like an ordinary Rust function
//! commands.spawn_scene(bsn! { enemy(200, "goblin") });
//! ```
//!
//! Braces are required when the macro would otherwise misparse the expression
//! and for complex expressions like `{hp * 2}`.
//!
//! ### Dynamic template values
//!
//! A [`Template`] value, such as an instance of a Component, cannot be directly passed in to a `bsn!` block, as `bsn!`
//! expects "scene variables" in that position. Instead use `template_value(...)` which accepts a given component [`Template`] value
//! and returns a [`Scene`] implementation for it.
//!
//! ```ignore
//! fn enemy(translation: Vec3){
//!     let transform = Transform::from_translation(translation);
//!     bsn! {
//!         #Foo
//!         template_value(transform)
//!     }
//!
//! }
//! ```
//!
//! ### Ad-hoc template functions
//!
//! Sometimes you need custom behavior or world access to create a [`Template`].
//! If this is the case, you can use [`template`](fn@template) instead of a custom [`FromTemplate`] or [`Template`] implementation.
//! In [`template`](fn@template) you get access to a [`TemplateContext`](bevy_ecs::template::TemplateContext) which
//! contains the [`EntityWorldMut`] and a collection of named entity references.
//!
//! ```ignore
//! bsn! {
//!     #Foo
//!     template(|ctx| {
//!         Foo(ctx.resource::<MyAssetCollection>().get("generated_asset_name"))
//!     })
//! }
//! ```
//!
//! ### Expressions as scenes
//!
//! You can insert a [`Scene`] or [`SceneList`] in another Scene using curly-bracketed expressions:
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
//! let items = bsn_list![#A, #B, #C]; // or bsn! if container takes a `impl Scene`
//! commands.spawn_scene(container(items));
//! ```
//!
//! ### Conditional values
//!
//! There is no `if`/`match` syntax inside the [`bsn!`] grammar (yet!), but you can embed
//! conditionals via `{...}` blocks or handle them outside the macro:
//!
//! ```ignore
//! fn unit(is_boss: bool) -> impl Scene {
//!     let hp = if is_boss { 500 } else { 100 };
//!     bsn! { Health { current: hp, max: hp } }
//! }
//! ```
//! One way to achieve conditional scenes is using a [`Box<dyn Scene>`] to store different scenes in one variable.
//! ```ignore
//! fn unit(is_boss: bool, level: u32) -> impl Scene {
//!     let scene: Box<dyn Scene> = if is_boss {
//!         Box::new(bsn! {
//!             Boss
//!             Followers [ // the boss is followed by some grunts
//!                 :unit(false, level - 1) #Grunt1,
//!                 :unit(false, level - 2) #Grunt2
//!             ]
//!         })
//!     } else {
//!         Box::new(bsn! { Grunt })
//!     };
//!     bsn! {
//!         Level(level)
//!         {scene}
//!     }
//! }
//! ```
//!
//! We plan on making "conditional scenes" easier to define in future releases.
//!
//! ## Scene Components
//!
//! A [`SceneComponent`] is a specialized type of [`Component`] that has an associated [`Scene`]:
//!
//! ```
//! # use bevy_scene::prelude::*;
//! # use bevy_ecs::prelude::*;
//! # #[derive(Component, Default, Clone)]
//! # struct Sword;
//! # #[derive(Component, Default, Clone)]
//! # struct Shield;
//! #[derive(SceneComponent, Default, Clone)]
//! struct Player {
//!     score: usize
//! }
//!
//! impl Player {
//!     fn scene() -> impl Scene {
//!         bsn! {
//!             #Player
//!             Children [
//!                 #RightHand Sword,
//!                 #LeftHand Shield,
//!             ]
//!         }
//!     }
//! }
//! ```
//!
//! This enables including the [`SceneComponent`] as a scene, using the following syntax:
//!
//! ```no_run
//! # use bevy_scene::prelude::*;
//! # use bevy_ecs::prelude::*;
//! # #[derive(SceneComponent, Default, Clone)]
//! # struct Player {
//! #    score: usize
//! # }
//! # impl Player {
//! #   fn scene() -> impl Scene {}
//! # }
//! # let mut world = World::new();
//! world.spawn_scene(bsn! {
//!  @Player { score: 0 }
//! });
//! ```
//!
//! This will spawn the `Player` component _and_ the entire scene with it. This means that you write
//! systems that query for the `Player` component, they can generally assume the rest of the scene will be there
//! too!
//!
//! [`SceneComponent`]s can only be spawned using scene APIs like [`World::spawn_scene`]. Spawning
//! them using [`World::spawn`] will log an error.
//!
//! ### Custom Scene Functions
//!
//! When deriving [`SceneComponent`], it defaults to using `Self::scene` as the "scene function".
//! Scene functions can also be manually specified:
//!
//! ```
//! # use bevy_scene::prelude::*;
//! # use bevy_ecs::prelude::*;
//! #[derive(SceneComponent, Default, Clone)]
//! #[scene(player)]
//! struct Player;
//!
//! fn player() -> impl Scene {
//!    bsn! { /* scene here */}
//! }
//! ```
//!
//! ### `SceneComponent` Asset Paths
//!
//! Note: Currently, Bevy does not include a `.bsn` asset format. These docs exist to help you understand what is planned, and what is currently possible
//! with third-party asset formats.
//!
//! Alternatively, a scene asset path can be specified:
//!
//! ```
//! # use bevy_scene::prelude::*;
//! # use bevy_ecs::prelude::*;
//! #[derive(SceneComponent, Default, Clone)]
//! #[scene("player.bsn")]
//! struct Player {
//!     score: usize
//! }
//! ```
//!
//!
//! ### Scene Components are Template-able
//!
//! Just like other [`Component`]s, [`SceneComponent`]s are "template-able"
//!
//! ```no_run
//! # use bevy_scene::prelude::*;
//! # use bevy_ecs::{prelude::*, template::TemplateContext};
//! # let mut world = World::new();
//! # struct Handle<T>(std::marker::PhantomData<T>);
//! # struct HandleTemplate<T>(String, std::marker::PhantomData<T>);
//! # impl<'a, T> From<&'a str> for HandleTemplate<T> {
//! #   fn from(value: &'a str) -> Self { todo!() }
//! # }
//! # impl<T> Default for HandleTemplate<T> {
//! #   fn default() -> Self { todo!() }
//! # }
//! # struct Image;
//! # impl<T> Template for HandleTemplate<T> {
//! #   type Output = Handle<T>;
//! #   fn build_template(&self, context: &mut TemplateContext) -> Result<Handle<T>> { todo!() }
//! #   fn clone_template(&self) -> Self { todo!() }
//! # }
//! # impl<T> FromTemplate for Handle<T> {
//! #   type Template = HandleTemplate<T>;
//! # }
//! #[derive(SceneComponent, FromTemplate)]
//! struct Player {
//!     image: Handle<Image>,
//! }
//!
//! impl Player {
//!     fn scene() -> impl Scene {
//!         bsn! { /* scene here */}
//!     }
//! }
//!
//! world.spawn_scene(bsn! {
//!    @Player { image: "player.png" }
//! });
//! ```
//!
//! ### `SceneComponent` Props
//!
//! Sometimes it is desirable to "parameterize" a scene: pass in values to the scene which determine
//! what the scene outputs are. The answer to this in BSN is "scene props":
//!
//! ```no_run
//! # use bevy_scene::prelude::*;
//! # use bevy_ecs::prelude::*;
//! # #[derive(Component, Default, Clone)]
//! # struct Node;
//! # #[derive(Component, Default, Clone)]
//! # struct Text(String);
//! # let mut world = World::new();
//! /// A UI widget that repeats "hello" text a given number of times.
//! #[derive(SceneComponent, Default, Clone)]
//! #[scene(HelloRepeaterProps)]
//! struct HelloRepeater;
//!
//! #[derive(Default)]
//! struct HelloRepeaterProps {
//!     repeat: usize,
//! }
//!
//! impl HelloRepeater {
//!     fn scene(props: HelloRepeaterProps) -> impl Scene {
//!         let hellos = (0..props.repeat)
//!             .map(|_| bsn! { Text("hello") })
//!             .collect::<Vec<_>>();
//!         bsn! {
//!             Node
//!             Children [
//!                 {hellos}
//!             ]
//!         }
//!     }
//! }
//!
//! world.spawn_scene(bsn! {
//!    @HelloRepeater {
//!        @repeat: 5
//!    }
//! });
//! ```
//!
//! Notice the `@field` syntax, which specifies that a prop is being set instead of a field.
//! Props are evaluated "immediately" when the scene is included in another scene.
//! This means that they are not "patchable", as at that point they have already been evaluated,
//! and they _produce_ "patchable" outputs.
//!
//! You can set _both_ props and normal fields at the same time:
//! ```no_run
//! # use bevy_scene::prelude::*;
//! # use bevy_ecs::prelude::*;
//! # let mut world = World::new();
//! # impl Widget {
//! #   fn scene(props: WidgetProps) -> impl Scene {}
//! # }
//! #[derive(SceneComponent, Default, Clone)]
//! #[scene(WidgetProps)]
//! struct Widget {
//!     value: usize
//! }
//!
//! #[derive(Default)]
//! struct WidgetProps {
//!     border: bool,
//! }
//!
//! world.spawn_scene(bsn! {
//!    @Widget {
//!        @border: true,
//!        value: 10,
//!    }
//! });
//! ```
//!
//! ### The Scene Component is Always Added
//!
//! Specifying the scene component manually in the scene function is not necessary. It will be added
//! automatically:
//!
//! ```
//! # use bevy_scene::prelude::*;
//! #[derive(SceneComponent, Default, Clone)]
//! struct Player;
//!
//! impl Player {
//!     fn scene() -> impl Scene {
//!         bsn! {
//!             // No need to specify a Player component here.
//!             // It is implied!
//!         }
//!     }
//! }
//! ```
//! However you _can_ patch the scene component in the scene if you would like. This comes in handy
//! if you would like props to contribute to the scene component's fields:
//!
//! ```
//! # use bevy_scene::prelude::*;
//! # #[derive(Default)]
//! # struct PlayerProps { size_in_millimeters: f32 };
//! # #[derive(SceneComponent, Default, Clone)]
//! # #[scene(PlayerProps)]
//! # struct Player { size_in_meters: f32 }
//! impl Player {
//!     fn scene(props: PlayerProps) -> impl Scene {
//!         bsn! {
//!             Player {
//!                 size_in_meters: {props.size_in_millimeters / 1000. }
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! ### Scene Components vs Required Components
//!
//! At first glance, Scene Components and [Required Components](bevy_ecs::component::Component) solve
//! similar problems. They both provide a mechanism to initialize components with other components.
//!
//! They are functionally quite different however. It is worth understanding the differences and
//! tradeoffs:
//!
//! - **Required Components**: Context-less (ex: Default constructors), non-hierarchical, can always
//!   be applied immediately, not dependency aware, automatically enforced at runtime as components
//!   are added, not patchable, pretty low overhead, not a lot of features / functionality
//! - **Scene Components**: Require context (ex: World access and "Entity Spawn Context", such as
//!   entity references), hierarchical (spawn children), cannot always be applied immediately
//!   (can have dependencies that aren't loaded yet), dependency aware, only enforced at spawn
//!   time, patchable, more dynamic / higher overhead, many features.
//!
//! Some good rules of thumb:
//!
//! - Are you building something "hierarchical" / with related entities? Use [`SceneComponent`].
//! - Do you want or need the full capabilities of the scene system? Use [`SceneComponent`].
//! - Are you spawning something that has dependencies / needs World access? use [`SceneComponent`].
//! - Are you defining "flat" components that aren't really scenes on their own? Use required components.
//! - Do you need the "required" components to be automatically added in non-scene contexts?  Use required components.
//! - Is spawn performance a very high priority? Use required components.
//!
//! ## .bsn Asset Format
//!
//! Bevy does not currently have support for `.bsn` files,
//! but intends to offer a `.bsn` asset format in future releases.
//!
//! This would allow you to define your scenes on disk,
//! creating/modifying them in various authoring tools and using asset hot-reloading.
//!
//! This format is intended to have broad syntactic compatibility with the `bsn!` macro,
//! making it easy to port your content between both the macro and the asset form.
//!
//! When planning your future use of `.bsn` asset files (which are not currently shipped), be aware that
//! unlike `bsn!` macro calls `.bsn` assets will not support expressions or other dynamic features directly.
//!
//! For now, you should use existing non-Bevy asset formats like glTF,
//! search for ecosystem implementations or stick to `bsn!` macro calls.
//!
//! Note that the architecture to support an asset format already exists,
//! allowing community implementations/experimentation until an official version exists. An example of how to go about this
//! can be found in the [scene benchmarks](<https://github.com/bevyengine/bevy/blob/v0.19.0/benches/benches/bevy_scene/spawn.rs#L414>)
//!
//! [`Template`]: bevy_ecs::template::Template
//! [`FromTemplate`]: bevy_ecs::template::FromTemplate
//! [`Asset`]: bevy_asset::Asset
//! [`Entity`]: bevy_ecs::entity::Entity
//! [`EntityTemplate`]: bevy_ecs::template::EntityTemplate
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
        EntityWorldMutSceneExt, PatchFromTemplate, PatchTemplate, Scene, SceneComponent, SceneList,
        ScenePatchInstance, SpawnListSystem, SpawnSystem, WorldSceneExt,
    };
}

/// Functionality used by the [`bsn!`] macro.
pub mod macro_utils;

extern crate alloc;

mod resolved_scene;
mod scene;
mod scene_component;
mod scene_list;
mod scene_patch;
mod spawn;
mod spawn_system;

pub use resolved_scene::*;
pub use scene::*;
pub use scene_component::*;
pub use scene_list::*;
pub use scene_patch::*;
pub use spawn::*;
pub use spawn_system::*;

use bevy_app::{App, Plugin, SceneSpawnerSystems, SpawnScene};
use bevy_asset::AssetApp;
use bevy_ecs::prelude::*;

pub use bevy_scene_macros::bsn;

pub use bevy_scene_macros::bsn_list;

pub use bevy_scene_macros::SceneComponent;

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
    use bevy_ecs::name::Name;
    use bevy_ecs::prelude::*;
    use bevy_ecs::relationship::Relationship;
    use bevy_ecs::system::{system_value, SystemHandle};
    use bevy_ecs::world::DeferredWorld;
    use bevy_reflect::TypePath;
    use bevy_scene_macros::SceneComponent;
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
    fn supports_fully_qualified_component_paths() {
        let mut app = test_app();
        let world = app.world_mut();

        assert!(world
            .spawn_scene(bsn! {
              ::bevy_ecs::prelude::Children[]
            })
            .is_ok());
    }

    #[test]
    fn cached_patching() {
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
    fn cached_patching_order() {
        let mut app = test_app();
        let world = app.world_mut();

        #[derive(Component, FromTemplate)]
        struct Position {
            x: f32,
            y: f32,
            z: f32,
        }

        fn a() -> impl Scene {
            bsn! {
                Position { x: 2. }
            }
        }

        fn b() -> impl Scene {
            bsn! {
                Position { x: 1., y: 1., z: 1. }
                a()
            }
        }

        let root = world.spawn_scene(b()).unwrap();
        let position = root.get::<Position>().unwrap();
        // Overridden by a
        assert_eq!(position.x, 2.);
        // Remains from b
        assert_eq!(position.y, 1.);
        assert_eq!(position.z, 1.);
    }

    #[test]
    fn loaded_asset_cached_patching() {
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

        #[derive(SceneComponent, Default, Clone)]
        #[scene("a.bsn")]
        struct AWidget {
            value: usize,
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

        // "a.bsn" as AWidget's "component scene"
        let id = world
            .spawn_scene(bsn! {@AWidget { value: 2 }})
            .unwrap()
            .id();
        let root = world.entity(id);

        let a_widget = root.get::<AWidget>().unwrap();
        assert_eq!(a_widget.value, 2);
        let position = root.get::<Position>().unwrap();
        assert_eq!(position.x, 0.);
        assert_eq!(position.y, 2.);
        assert_eq!(position.z, 0.);

        let children = root.get::<Children>().unwrap();
        assert_eq!(children.len(), 1);

        let x = world.entity(children[0]);
        let name = x.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "X");
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
                    (b() Reference(#X))
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
    fn bsn_reverse_reference() {
        let mut app = test_app();
        let world = app.world_mut();

        fn a() -> impl Scene {
            bsn! {
                Reference(#Last)
                Children [
                    #First,
                    #Second,
                    #Last
                ]
            }
        }
        let id = world.spawn_scene(a()).unwrap().id();
        let ref_id = world.entity(id).get::<Reference>().unwrap();

        let children = world.entity(id).get::<Children>().unwrap();
        assert_eq!(children[2], ref_id.0);

        let name = world.entity(ref_id.0).get::<Name>().unwrap();
        assert_eq!(name.as_str(), "Last");
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
                (b() #Z)
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
    fn primitive_literals() {
        #![allow(dead_code, reason = "test")]
        // test that bsn compiles and doesn't fail to spawn a scene with all sorts of literal values
        macro_rules! types_fields {
            (def $name:ident, $($typ:ident),*) => {
                #[derive(Component, FromTemplate)]
                struct $name {
                    $(
                        $typ: $typ,
                    )*
                }
            };
            ($world:ident, $name:ident($a:expr, $b:expr, $c:expr, $d:expr, $e:expr), $($typ:ident),*) => {
                types_fields!($world, $name($a, $b, $c, $d), $($typ),*);
                types_fields!(val $world, $e, $name, $($typ),*);
            };
            ($world:ident, $name:ident($a:expr, $b:expr, $c:expr, $d:expr), $($typ:ident),*) => {
                types_fields!($world, $name($a, $b, $c), $($typ),*);
                types_fields!(val $world, $d, $name, $($typ),*);
            };
            ($world:ident, $name:ident($a:expr, $b:expr, $c:expr), $($typ:ident),*) => {
                types_fields!($world, $name($a, $b), $($typ),*);
                types_fields!(val $world, $c, $name, $($typ),*);

            };
            ($world:ident, $name:ident($a:expr, $b:expr), $($typ:ident),*) => {
                types_fields!($world, $name($a), $($typ),*);
                types_fields!(val $world, $b, $name, $($typ),*);
            };
            ($world:ident, $name:ident($a:expr), $($typ:ident),*) => {
                types_fields!(def $name, $($typ),*);
                types_fields!(val $world, $a, $name, $($typ),*);
            };
            (val $world:ident, $a:expr, $name:ident, $($typ:ident),*) => {
                let v = bsn!{ $name {
                    $(
                        $typ: $a,
                    )*
                }};
                $world.spawn_scene(v).unwrap();
            };
        }
        let mut app = test_app();
        let world = app.world_mut();
        types_fields!(world, Unsigned(0, 1), usize, u8, u16, u32, u64, u128);
        types_fields!(world, Signed(-1, 0, 1), isize, i8, i16, i32, i64, i128);
        types_fields!(
            world,
            Float(-1.0, 0.0, 1.0, core::f32::consts::PI, -1.),
            f32,
            f64
        );
        types_fields!(world, Bool(true, false), bool);
        #[derive(Component, FromTemplate)]
        struct Random {
            str: &'static str,
            string: String,
            vec: Vec<u8>,
            array: [u8; 4],
        }
        let scene = bsn! {
            Random{
                str: "test",
                string: "test",
                vec: {vec![0, 1]},
                array: {[0, 1, 2, 3]}
            }
        };
        world.spawn_scene(scene).unwrap();
    }
    #[test]
    fn children_list_expr() {
        fn container(items: impl SceneList) -> impl Scene {
            bsn! {
                #Root
                Children [
                    #First,
                    {items},
                    #Last
                ]
            }
        }
        let mut app = test_app();
        let world = app.world_mut();
        let items = bsn_list![
            #Second,
            #Third
        ];
        let id = world.spawn_scene(container(items)).unwrap().id();
        let children = world.entity(id).get::<Children>().unwrap();
        let names: Vec<_> = children
            .iter()
            .map(|id| world.entity(id).get::<Name>().unwrap().as_str())
            .collect();
        assert_eq!(&names, &["First", "Second", "Third", "Last"]);
    }
    #[test]
    fn children_single_expr() {
        fn container(item: impl Scene) -> impl Scene {
            bsn! {
                #Root
                Children [
                    #First,
                    ({item}),
                    #Last
                ]
            }
        }
        let mut app = test_app();
        let world = app.world_mut();
        let items = bsn![
            #Second
        ];
        let id = world.spawn_scene(container(items)).unwrap().id();
        let children = world.entity(id).get::<Children>().unwrap();
        let names: Vec<_> = children
            .iter()
            .map(|id| world.entity(id).get::<Name>().unwrap().as_str())
            .collect();
        assert_eq!(&names, &["First", "Second", "Last"]);
    }
    #[test]
    fn conditional_scene() {
        #[derive(Component, Clone, Default)]
        struct Grunt;
        #[derive(Component, Clone, Default)]
        struct Boss;
        #[derive(Component, Clone, Default, PartialEq, Eq)]
        struct Level(u32);

        fn unit(is_boss: bool, level: u32) -> impl Scene {
            let scene: Box<dyn Scene> = if is_boss {
                Box::new(bsn! {
                    Boss
                    Children [ unit(false, level - 1) #Grunt1, unit(false, level - 1) #Grunt2]
                })
            } else {
                Box::new(bsn! { Grunt })
            };
            bsn! {
                Level(level)
                {scene}
            }
        }
        let mut app = test_app();
        let world = app.world_mut();

        let id = world.spawn_scene(unit(true, 10)).unwrap().id();
        let children = world.entity(id).get::<Children>().unwrap();
        let names: Vec<_> = children
            .iter()
            .map(|id| world.entity(id).get::<Name>().unwrap().as_str())
            .collect();
        assert_eq!(&names, &["Grunt1", "Grunt2"]);
        let names: Vec<_> = children
            .iter()
            .map(|id| world.entity(id).get::<Level>().unwrap().0)
            .collect();
        assert_eq!(&names, &[9, 9]);
    }
    #[test]
    fn partial_tuple_struct() {
        // Tests that only part of a tuple struct can be patched,
        // since its different to named fields
        let mut app = test_app();
        let world = app.world_mut();
        #[derive(Component, Default, Clone)]
        struct TupleStruct(f32, u32);

        fn a() -> impl Scene {
            bsn! {
                TupleStruct(0.1)
            }
        }
        let id = world.spawn_scene(a()).unwrap().id();
        let root = world.entity(id);

        let foo = root.get::<TupleStruct>().unwrap();
        assert_eq!(foo.0, 0.1);
        assert_eq!(foo.1, 0);
    }

    #[test]
    fn scene_expression_passing_pointless() {
        // This test exists mostly to ensure that the practice of not passing `impl Scene` into a scene
        // is the same as the preferred option, using patching.
        #[derive(Component, Default, Clone)]
        struct Health {
            current: u8,
            max: u8,
        }
        #[derive(Component, Default, Clone)]
        struct Armor(u8);

        fn unit_with_armor(unit_base: impl Scene) -> impl Scene {
            bsn! {
                {unit_base}
                Armor(50)
            }
        }
        fn armor() -> impl Scene {
            bsn! {
                Armor(50)
            }
        }
        let mut app = test_app();
        let world = app.world_mut();
        let inner = bsn! { Health { current: 100, max: 100 } };
        let ida = world.spawn_scene(unit_with_armor(inner)).unwrap().id();

        // inheritance is the same!
        let entity_b = bsn! {
            armor()
            Health { current: 100, max: 100 }
        };
        let idb = world.spawn_scene(entity_b).unwrap().id();

        assert_eq!(
            world.entity(ida).archetype().components(),
            world.entity(idb).archetype().components()
        );
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
    fn field_patching_with_default() {
        let mut app = test_app();
        let world = app.world_mut();

        #[derive(Component, Clone, Default, PartialEq, Eq, Debug)]
        struct Foo {
            x: u32,
            y: u32,
            z: u32,
            nested: Bar,
        }

        #[derive(Component, Clone, Default, PartialEq, Eq, Debug)]
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
    fn child_of_template() {
        let mut app = test_app();

        let world = app.world_mut();

        fn scene(root: Entity) -> impl SceneList {
            bsn_list! {
                ( #Child1 ChildOf(root) ),
                ( #Child2 ChildOf(#Child1) ),
            }
        }

        let root = world.spawn_empty().id();

        let ids = world.spawn_scene_list(scene(root)).unwrap();
        assert_eq!(ids.len(), 2);

        let [a, b] = world.entity(&*ids)[..] else {
            unreachable!()
        };
        assert_eq!(a.get::<Name>().unwrap().as_str(), "Child1");
        assert_eq!(a.get::<ChildOf>().unwrap().get(), root);

        assert_eq!(b.get::<Name>().unwrap().as_str(), "Child2");
        assert_eq!(b.get::<ChildOf>().unwrap().get(), a.id());
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
        fn on_heal(_: On<Heal>, mut healed: ResMut<TotalHealed>) {
            healed.0 += 2;
        }
        fn function_scene() -> impl Scene {
            bsn! {
                on(on_heal)
            }
        }

        let id = world.spawn_scene(function_scene()).unwrap().id();
        world.trigger(Heal(id));
        assert_eq!(world.resource::<TotalHealed>().0, 2);
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
    fn component_scene() {
        #[derive(SceneComponent, Default, Clone)]
        struct Widget;

        impl Widget {
            fn scene() -> impl Scene {
                bsn! {Name("widget")}
            }
        }

        let mut app = test_app();
        let world = app.world_mut();
        let entity = world.spawn_scene(bsn! {@Widget}).unwrap();
        assert_eq!(entity.get::<Name>().unwrap().as_str(), "widget");
        assert!(entity.contains::<Widget>());

        #[derive(SceneComponent, Default, Clone)]
        #[scene(Widget::scene)]
        struct OtherWidget;
        let entity = world.spawn_scene(bsn! {@OtherWidget}).unwrap();
        assert_eq!(entity.get::<Name>().unwrap().as_str(), "widget");
        assert!(entity.contains::<OtherWidget>());
        assert!(
            !entity.contains::<Widget>(),
            "This reuses the Widget::scene function, but that scene does not explicitly add Widget"
        );
    }

    #[test]
    fn component_scene_props() {
        #[derive(SceneComponent, Default, Clone)]
        #[scene(WidgetProps)]
        struct Widget {
            value: usize,
        }

        #[derive(Default)]
        struct WidgetProps {
            children: usize,
        }

        impl Widget {
            fn scene(props: WidgetProps) -> impl Scene {
                let children = (0..props.children)
                    .map(|i| {
                        bsn! {
                            Name({format!("Child{i}")})
                        }
                    })
                    .collect::<Vec<_>>();
                bsn! {
                    Children [
                        {children}
                    ]
                }
            }
        }

        let mut app = test_app();
        let world = app.world_mut();
        let entity = world
            .spawn_scene(bsn! {@Widget {
                @children: 2,
                value: 10,
            }})
            .unwrap();
        assert_eq!(entity.get::<Widget>().unwrap().value, 10);
        assert_eq!(entity.get::<Children>().unwrap().len(), 2);

        #[derive(SceneComponent, Default, Clone)]
        #[scene(Widget::scene(WidgetProps))]
        struct OtherWidget {
            value: usize,
        }

        let entity = world
            .spawn_scene(bsn! {@OtherWidget {
                @children: 2,
                value: 10,
            }})
            .unwrap();
        assert_eq!(entity.get::<OtherWidget>().unwrap().value, 10);
        assert_eq!(entity.get::<Children>().unwrap().len(), 2);

        fn const_val<const N: usize>() -> impl Scene {
            bsn! {
                @Widget {
                    @children: N
                }
            }
        }
        #[derive(SceneComponent, Clone, Default)]
        #[scene(const_val::<5>)]
        struct SpecificWidget;
        let entity = world.spawn_scene(bsn! { @SpecificWidget }).unwrap();
        assert_eq!(entity.get::<Children>().unwrap().len(), 5);
    }

    #[test]
    fn scene_without_explicit_component_still_spawns_component() {
        #[derive(SceneComponent, Default, Clone)]
        struct Widget;

        impl Widget {
            fn scene() -> impl Scene {
                bsn! {}
            }
        }

        let mut app = test_app();
        let world = app.world_mut();
        let entity = world.spawn_scene(bsn! {@Widget}).unwrap();
        assert!(entity.contains::<Widget>());
    }

    #[test]
    fn tuple_scene_component_name_reference() {
        #[derive(SceneComponent, FromTemplate)]
        struct Widget(pub Entity);

        impl Widget {
            fn scene() -> impl Scene {
                bsn! {}
            }
        }

        let scene = bsn! {
            #Name
            Children [
                @Widget(#Name)
            ]
        };

        let mut app = test_app();
        let world = app.world_mut();
        let entity = world.spawn_scene(scene).unwrap().id();
        let root = world.entity(entity);
        let children = root.get::<Children>().unwrap();
        let child_widget = world.entity(children[0]).get::<Widget>().unwrap();
        assert_eq!(child_widget.0, entity);
    }

    #[test]
    fn named_scene_component_name_reference() {
        #[derive(SceneComponent, FromTemplate)]
        struct Widget {
            entity: Entity,
        }

        impl Widget {
            fn scene() -> impl Scene {
                bsn! {}
            }
        }

        let scene = bsn! {
            #Name
            Children [
                @Widget{
                    entity: #Name
                }
            ]
        };

        let mut app = test_app();
        let world = app.world_mut();
        let entity = world.spawn_scene(scene).unwrap().id();
        let root = world.entity(entity);
        let children = root.get::<Children>().unwrap();
        let child_widget = world.entity(children[0]).get::<Widget>().unwrap();
        assert_eq!(child_widget.entity, entity);
    }

    #[test]
    fn scene_function_name_reference() {
        use bevy_ecs::template::EntityTemplate;
        #[derive(Component, FromTemplate)]
        struct Reference(Entity);
        fn widget(entity: EntityTemplate) -> impl Scene {
            bsn! {
                Reference(entity)
            }
        }
        let mut app = test_app();
        let world = app.world_mut();

        let pass_expr = bsn! {
            #Name
            Children [
                widget(Entity::PLACEHOLDER.into())
            ]
        };
        let entity = world.spawn_scene(pass_expr).unwrap().id();
        let root = world.entity(entity);
        let children = root.get::<Children>().unwrap();
        let child_widget = world.entity(children[0]).get::<Reference>().unwrap();
        assert_eq!(child_widget.0, Entity::PLACEHOLDER);

        let pass_name = bsn! {
            #Name
            Children [
                widget(#Name)
            ]
        };
        let entity = world.spawn_scene(pass_name).unwrap().id();
        let root = world.entity(entity);
        let children = root.get::<Children>().unwrap();
        let child_widget = world.entity(children[0]).get::<Reference>().unwrap();
        assert_eq!(child_widget.0, entity);

        // This allows both passing entity id by name reference and a custom dynamic name
        let i = 5;
        let pass_name_expr = bsn! {
            #Root
            Name({format!("Foo{i}")})
            Children [
                #Name
                widget(#Root)
            ]
        };
        let entity = world.spawn_scene(pass_name_expr).unwrap().id();
        let root = world.entity(entity);
        let children = root.get::<Children>().unwrap();
        let child_widget = world.entity(children[0]).get::<Reference>().unwrap();
        assert_eq!(child_widget.0, entity);
        let name = root.get::<Name>().unwrap();
        assert_eq!(name.as_str(), "Foo5");
    }

    #[test]
    fn scene_component_prop_name_reference() {
        use bevy_ecs::template::EntityTemplate;
        #[derive(Component, FromTemplate)]
        struct Reference(Entity);

        #[derive(SceneComponent, Clone, Default)]
        #[scene(WidgetProps)]
        struct Widget;

        #[derive(Default)]
        struct WidgetProps {
            entity: EntityTemplate,
        }

        impl Widget {
            fn scene(props: WidgetProps) -> impl Scene {
                bsn! {
                    Reference({props.entity})
                }
            }
        }
        let mut app = test_app();
        let world = app.world_mut();

        let prop_expr = bsn! {
            Children [
                @Widget {
                    @entity: Entity::PLACEHOLDER
                }
            ]
        };
        let entity = world.spawn_scene(prop_expr).unwrap().id();
        let root = world.entity(entity);
        let children = root.get::<Children>().unwrap();
        let child_widget = world.entity(children[0]).get::<Reference>().unwrap();
        assert_eq!(child_widget.0, Entity::PLACEHOLDER);
        let scene_prop = bsn! {
            #Name
            Children [
                @Widget {
                    @entity: #Name
                }
            ]
        };
        let entity = world.spawn_scene(scene_prop).unwrap().id();
        let root = world.entity(entity);
        let children = root.get::<Children>().unwrap();
        let child_widget = world.entity(children[0]).get::<Reference>().unwrap();
        assert_eq!(child_widget.0, entity);
    }

    #[test]
    fn repeated_call_entity_reference() {
        let scenes = (0..6).map(|_: u32| bsn! { #Name }).collect::<Vec<_>>();
        let scenes_len = scenes.len();
        let mut app = test_app();
        let world = app.world_mut();
        world.spawn_scene_list(scenes).unwrap();
        assert_eq!(world.query::<&Name>().query(world).count(), scenes_len);
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

    // NOTE: function scene caching is not yet implemented
    // #[test]
    // fn caching_with_generics() {
    //     #[derive(Component, FromTemplate, PartialEq, Eq, Debug)]
    //     struct Foo<T: FromTemplate<Template: Default + Template<Output = T>>> {
    //         value: T,
    //         number: u32,
    //     }

    //     fn b() -> impl Scene {
    //         bsn! {
    //             :a::<0, i32>
    //             Children [ #Y ]
    //         }
    //     }

    //     fn a<
    //         const A: u32,
    //         T: 'static
    //             + Send
    //             + Sync
    //             + FromTemplate<Template: Send + Sync + Default + Template<Output = T>>,
    //     >() -> impl Scene {
    //         bsn! {
    //             Foo<T>{
    //                 number: A
    //             }
    //         }
    //     }

    //     b();
    // }

    #[test]
    fn scene_with_blocks() {
        #[derive(Component, Clone, Default)]
        struct Holder {
            const_block: i32,
            unsafe_block: i32,
        }

        fn func() -> impl Scene {
            bsn! {
                Holder {
                    const_block: const {0},
                    unsafe_block: unsafe {0},
                }
            }
        }

        func();
    }
    #[test]
    fn macro_doc_test() {
        #![allow(unused, reason = "test")]
        #![allow(dead_code, reason = "test")]

        fn some_scene() -> impl Scene {}
        #[derive(Component, Default, Clone)]
        struct ComponentA;
        #[derive(Component, Default, Clone)]
        struct ComponentB(f32, u8);
        #[derive(Component, Default, Clone)]
        struct Node {
            height: f32,
            width: f32,
        }
        fn px(v: f32) -> f32 {
            v
        }
        #[derive(EntityEvent)]
        struct MyEntityEvent {
            entity: Entity,
            value: f32,
        }
        fn other_scene() -> impl Scene {}
        #[derive(Component, FromTemplate)]
        struct Link(Entity);
        #[derive(SceneComponent, FromTemplate)]
        #[scene(scenecomponentscene(Props))]
        struct MySceneComponent {
            normal_field: u8,
        }
        #[derive(Default)]
        struct Props {
            some_prop: u8,
        }
        fn scenecomponentscene(props: Props) -> impl Scene {}
        let some_var: f32 = 0.;
        #[derive(SceneComponent, FromTemplate)]
        #[scene(scenecomponentscene2(Props2))]
        struct Container;
        struct Props2 {
            items: Box<dyn SceneList>,
        }
        impl Default for Props2 {
            fn default() -> Self {
                Self {
                    items: Box::new(bsn_list!()),
                }
            }
        }
        fn scenecomponentscene2(props: Props2) -> impl Scene {}
        #[derive(Component, Default, Clone)]
        struct SomeComponent;
        // Copy of the macro from bevy_scene/macros/src/lib.rs
        // why? because it should be tested
        // why not doctests? because the macro can't depend on this crate
        // why not include! it here and include_str! it in the docs? because rust-analyzer inline docs ignores #[doc = include_str!()]
        let scene = bsn! {
            some_scene()        // include a scene function
            #SomeName           // entity name, will insert Name("SomeName")
            ComponentA          // component without a value will use default
            ComponentB(0.0)     // passing a value, other fields will use default
            Node {
                height: px(0.1) // same with named fields, unmentioned ones stay default
            }
            on(|evt: On<MyEntityEvent>, mut query: Query<&mut ComponentB>| {  // add an observer
                let mut b = query.get_mut(evt.entity).unwrap();
                b.0 += evt.value;
            })
            Children [                   // spawning multiple related entities using a RelationshipTarget component
                #Child1 ComponentA       // whitespace doesn't have to be newlines
                ,                        // entities are comma-separated
                (other_scene() #Child3), // parentheses around a single entity are optional
                Link(#SomeName),         // passing a entity reference to a component as `Entity`, component has to implement FromTemplate
                @MySceneComponent {      // components which derive SceneComponent have scenes and can be inherited from
                    @some_prop: 3,       // props, look like fields prefixed with @ but end up passed to the components scene as arguments
                    normal_field: 5      // while normal fields are the actual fields of the component
                },
                (
                    Node {
                        width: some_var      // you can directly use variables without {}
                    }
                    ComponentB({some_var + 3.})  // values can be expressions, when wrapped in {}
                    @Container {
                        @items: {
                            bsn_list![                // sometimes you may need to nest macro calls
                                #item1 SomeComponent, // note: the name #item1 here is in its own scope
                                some_scene() #item2
                            ]
                        }
                    }
                )
            ]
        };
        // just checking it spawns correctly
        let mut app = test_app();
        let world = app.world_mut();
        let entity = world.spawn_scene(scene).unwrap().id();
    }

    #[test]
    fn scene_with_oneshot_system() {
        #[derive(Component, FromTemplate)]
        struct Callback {
            callback: SystemHandle<(), ()>,
        }

        fn my_system() {}

        let mut app = test_app();
        let world = app.world_mut();

        let direct = bsn! {
            Callback {
                callback: system_value(my_system)
            }
        };
        let direct_ent = world.spawn_scene(direct).unwrap();
        assert!(direct_ent.get::<Callback>().is_some());

        let id = world.register_tracked_system(my_system);
        let id2 = id.clone();

        let indirect = bsn! {
            Callback {
                callback: id
            }
        };
        let indirect_ent = world.spawn_scene(indirect).unwrap();
        assert!(indirect_ent.get::<Callback>().unwrap().callback == id2);
    }

    #[test]
    fn direct_macro_values_in_bsn() {
        let mut app = test_app();
        let world = app.world_mut();

        #[derive(Component, Default, Clone)]
        struct Foo {
            value: Vec<usize>,
        }
        let entity = world
            .spawn_scene(bsn! {
                Foo {
                    value: vec! [ 10 ],
                }
            })
            .unwrap();
        assert_eq!(entity.get::<Foo>().unwrap().value, vec![10]);
    }

    #[test]
    fn enum_variant_field_values_use_implicit_into() {
        let mut app = test_app();
        let world = app.world_mut();

        #[derive(Component, Default, Clone)]
        struct TextFont {
            font_size: FontSize,
        }

        #[derive(Default, Clone, Debug, PartialEq, Eq)]
        struct FontSize(u32);

        enum TextSize {
            Large,
        }

        impl From<TextSize> for FontSize {
            fn from(value: TextSize) -> Self {
                match value {
                    TextSize::Large => FontSize(24),
                }
            }
        }

        let entity = world
            .spawn_scene(bsn! {
                TextFont {
                    font_size: TextSize::Large,
                }
            })
            .unwrap();

        assert_eq!(entity.get::<TextFont>().unwrap().font_size, FontSize(24));
    }

    #[test]
    fn enum_variant_subexpressions_are_hoisted() {
        #[derive(Component, FromTemplate, PartialEq, Eq, Debug, Clone)]
        enum FontSource {
            #[default]
            Handle { value: String },
        }

        struct Config {
            value: String,
        }

        fn make_scene(config: &Config) -> impl Scene {
            bsn! {
                Children [
                    (FontSource::Handle { value: { config.value.clone() } }),
                    (FontSource::Handle { value: { config.value.clone() } }),
                ]
            }
        }

        let mut app = test_app();
        let world = app.world_mut();
        let config = Config {
            value: "test".to_string(),
        };
        let entity = world.spawn_scene(make_scene(&config)).unwrap().id();
        let children = world.entity(entity).get::<Children>().unwrap();
        assert_eq!(
            world.entity(children[0]).get::<FontSource>().unwrap(),
            &FontSource::Handle {
                value: "test".to_string(),
            }
        );
        assert_eq!(
            world.entity(children[1]).get::<FontSource>().unwrap(),
            &FontSource::Handle {
                value: "test".to_string(),
            }
        );
    }

    #[test]
    fn field_name_shorthand() {
        let mut app = test_app();
        let world = app.world_mut();

        #[derive(Component, Default, Clone)]
        struct Foo {
            value: usize,
        }
        let value = 10usize;
        let entity = world.spawn_scene(bsn! { Foo { value } }).unwrap();
        assert_eq!(entity.get::<Foo>().unwrap().value, 10);

        #[derive(SceneComponent, Default, Clone)]
        #[scene(BarProps)]
        struct Bar {
            value: usize,
        }

        #[derive(Default)]
        struct BarProps {
            value: usize,
        }

        impl Bar {
            fn scene(props: BarProps) -> impl Scene {
                bsn! {Bar {
                    value: {props.value}
                }}
            }
        }

        let value = 10usize;
        let entity = world.spawn_scene(bsn! { @Bar { @value } }).unwrap();
        assert_eq!(entity.get::<Bar>().unwrap().value, 10);

        #[derive(Component, Default, Clone)]
        struct Baz {
            value: X,
        }

        #[derive(Default, Clone)]
        struct X;

        #[derive(Default, Clone)]
        struct Y;

        impl From<Y> for X {
            fn from(_: Y) -> Self {
                X
            }
        }

        let value = Y;
        // ensure implicit Into works
        let _ = world.spawn_scene(bsn! { Baz { value } }).unwrap();
    }

    #[test]
    fn scene_with_optional_components() {
        let mut app = test_app();
        let world = app.world_mut();

        #[derive(Component, Default, Clone)]
        struct Foo;

        let optional_component = Some(bsn! {
            Foo
        });

        let entity = world
            .spawn_scene(bsn! {
                #MaybeFoo
                {optional_component}
            })
            .unwrap();
        assert!(entity.get::<Foo>().is_some());
    }

    #[test]
    fn scene_nested_entity_references() {
        let mut app = test_app();
        let world = app.world_mut();

        #[derive(Component, FromTemplate)]
        struct Ref(Entity);

        let patch = bsn! {
            #patch
            Children [
                Ref(#patch)
            ]
        };

        let root = bsn! {
            #root
            patch
        };

        let expected_id = Some(world.spawn_scene(root).unwrap().id());
        let actual_id = world
            .query::<&Ref>()
            .query(world)
            .single()
            .ok()
            .map(|r| r.0);

        assert_eq!(expected_id, actual_id);
    }

    #[test]
    fn scene_list_nested_entity_references() {
        let mut app = test_app();
        let world = app.world_mut();

        #[derive(Component, FromTemplate)]
        struct Ref(Entity);

        let patch = bsn! {
            #patch
            Children [
                Ref(#patch)
            ]
        };

        let root = bsn_list! {
            #root patch
        };

        let expected_id = Some(world.spawn_scene_list(root).unwrap()[0]);
        let actual_id = world
            .query::<&Ref>()
            .query(world)
            .single()
            .ok()
            .map(|r| r.0);

        assert_eq!(expected_id, actual_id);
    }
}
