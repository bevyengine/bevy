//! This example demonstrates how to load scene data from files and then dynamically
//! apply that data to entities in your Bevy `World`. This includes spawning new
//! entities and applying updates to existing ones. Scenes in Bevy encapsulate
//! serialized and deserialized `Components` or `Resources` so that you can easily
//! store, load, and manipulate data outside of a purely code-driven context.
//!
//! This example also shows how to do the following:
//! * Register your custom types for reflection, which allows them to be serialized,
//!   deserialized, and manipulated dynamically.
//! * Skip serialization of fields you don't want stored in your scene files (like
//!   runtime values that should always be computed dynamically).
//! * Save a new scene to disk to show how it can be updated compared to the original
//!   scene file (and how that updated scene file might then be used later on).
//!
//! The example proceeds by creating components and resources, registering their types,
//! loading a scene from a file, logging when changes are detected, and finally saving
//! a new scene file to disk. This is useful for anyone wanting to see how to integrate
//! file-based scene workflows into their Bevy projects.
//!
//! # Note on working with files
//!
//! The saving behavior uses the standard filesystem APIs, which are blocking, so it
//! utilizes a thread pool (`IoTaskPool`) to avoid stalling the main thread. This
//! won't work on WASM because WASM typically doesn't have direct filesystem access.
//!

use bevy::{asset::LoadState, prelude::*, tasks::IoTaskPool};
use core::time::Duration;
use std::{fs::File, io::Write};

/// The entry point of our Bevy app.
///
/// Sets up default plugins, registers all necessary component/resource types
/// for serialization/reflection, and runs the various systems in the correct schedule.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(
            Startup,
            (save_scene_system, load_scene_system, infotext_system),
        )
        .add_systems(Update, (log_system, panic_on_fail))
        .run();
}

/// # Components, Resources, and Reflection
///
/// Below are some simple examples of how to define your own Bevy `Component` types
/// and `Resource` types so that they can be properly reflected, serialized, and
/// deserialized. The `#[derive(Reflect)]` macro enables Bevy's reflection features,
/// and we add component-specific reflection by using `#[reflect(Component)]`.
/// We also illustrate how to skip serializing fields and how `FromWorld` can help
/// create runtime-initialized data.
///
/// A sample component that is fully serializable.
///
/// This component has public `x` and `y` fields that will be included in
/// the scene files. Notice how it derives `Default`, `Reflect`, and declares
/// itself as a reflected component with `#[reflect(Component)]`.
#[derive(Component, Reflect, Default)]
#[reflect(Component)] // this tells the reflect derive to also reflect component behaviors
struct ComponentA {
    /// An example `f32` field
    pub x: f32,
    /// Another example `f32` field
    pub y: f32,
}

/// A sample component that includes both serializable and non-serializable fields.
///
/// This is useful for skipping serialization of runtime data or fields you
/// don't want written to scene files.
#[derive(Component, Reflect)]
#[reflect(Component)]
struct ComponentB {
    /// A string field that will be serialized.
    pub value: String,
    /// A `Duration` field that should never be serialized to the scene file, so we skip it.
    #[reflect(skip_serializing)]
    pub _time_since_startup: Duration,
}

/// This implements `FromWorld` for `ComponentB`, letting us initialize runtime fields
/// by accessing the current ECS resources. In this case, we acquire the `Time` resource
/// and store the current elapsed time.
impl FromWorld for ComponentB {
    fn from_world(world: &mut World) -> Self {
        let time = world.resource::<Time>();
        ComponentB {
            _time_since_startup: time.elapsed(),
            value: "Default Value".to_string(),
        }
    }
}

/// A simple resource that also derives `Reflect`, allowing it to be stored in scenes.
///
/// Just like a component, you can skip serializing fields or implement `FromWorld` if needed.
#[derive(Resource, Reflect, Default)]
#[reflect(Resource)]
struct ResourceA {
    /// This resource tracks a `score` value.
    pub score: u32,
}

/// # Scene File Paths
///
/// `SCENE_FILE_PATH` points to the original scene file that we'll be loading.
/// `NEW_SCENE_FILE_PATH` points to the new scene file that we'll be creating
/// (and demonstrating how to serialize to disk).
///
/// The initial scene file will be loaded below and not change when the scene is saved.
const SCENE_FILE_PATH: &str = "scenes/load_scene_example.scn.ron";

/// The new, updated scene data will be saved here so that you can see the changes.
const NEW_SCENE_FILE_PATH: &str = "scenes/load_scene_example-new.scn.ron";

/// Loads a scene from an asset file and spawns it in the current world.
///
/// Spawning a `DynamicSceneRoot` creates a new parent entity, which then spawns new
/// instances of the scene's entities as its children. If you modify the
/// `SCENE_FILE_PATH` scene file, or if you enable file watching, you can see
/// changes reflected immediately.
fn load_scene_system(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(DynamicSceneRoot(asset_server.load(SCENE_FILE_PATH)));
}

/// Logs changes made to `ComponentA` entities, and also checks whether `ResourceA`
/// has been recently added.
///
/// Any time a `ComponentA` is modified, that change will appear here. This system
/// demonstrates how you might detect and handle scene updates at runtime.
fn log_system(
    query: Query<(Entity, &ComponentA), Changed<ComponentA>>,
    res: Option<Res<ResourceA>>,
) {
    for (entity, component_a) in &query {
        info!("  Entity({})", entity.index());
        info!(
            "    ComponentA: {{ x: {} y: {} }}\n",
            component_a.x, component_a.y
        );
    }
    if let Some(res) = res
        && res.is_added()
    {
        info!("  New ResourceA: {{ score: {} }}\n", res.score);
    }
}

/// Demonstrates how to create a new scene from scratch, populate it with data,
/// and then serialize it to a file. The new file is written to `NEW_SCENE_FILE_PATH`.
///
/// This system creates a fresh world, duplicates the type registry so that our
/// custom component types are recognized, spawns some sample entities and resources,
/// and then serializes the resulting dynamic scene.
fn save_scene_system(world: &mut World) {
    // Scenes can be created from any ECS World.
    // You can either create a new one for the scene or use the current World.
    // For demonstration purposes, we'll create a new one.
    let mut scene_world = World::new();

    // The `TypeRegistry` resource contains information about all registered types (including components).
    // This is used to construct scenes, so we'll want to ensure that our previous type registrations
    // exist in this new scene world as well.
    // To do this, we can simply clone the `AppTypeRegistry` resource.
    let type_registry = world.resource::<AppTypeRegistry>().clone();
    scene_world.insert_resource(type_registry);

    let mut component_b = ComponentB::from_world(world);
    component_b.value = "hello".to_string();
    scene_world.spawn((
        component_b,
        ComponentA { x: 1.0, y: 2.0 },
        Transform::IDENTITY,
        Name::new("joe"),
    ));
    scene_world.spawn(ComponentA { x: 3.0, y: 4.0 });
    scene_world.insert_resource(ResourceA { score: 1 });

    // With our sample world ready to go, we can now create our scene using DynamicScene or DynamicSceneBuilder.
    // For simplicity, we will create our scene using DynamicScene:
    let scene = DynamicScene::from_world(&scene_world);

    // Scenes can be serialized like this:
    let type_registry = world.resource::<AppTypeRegistry>();
    let type_registry = type_registry.read();
    let serialized_scene = scene.serialize(&type_registry).unwrap();

    // Showing the scene in the console
    info!("{}", serialized_scene);

    // Writing the scene to a new file. Using a task to avoid calling the filesystem APIs in a system
    // as they are blocking.
    //
    // This can't work in Wasm as there is no filesystem access.
    #[cfg(not(target_arch = "wasm32"))]
    IoTaskPool::get()
        .spawn(async move {
            // Write the scene RON data to file
            File::create(format!("assets/{NEW_SCENE_FILE_PATH}"))
                .and_then(|mut file| file.write(serialized_scene.as_bytes()))
                .expect("Error while writing scene to file");
        })
        .detach();
}

/// Spawns a simple 2D camera and some text indicating that the user should
/// check the console output for scene loading/saving messages.
///
/// This system is only necessary for the info message in the UI.
fn infotext_system(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        Text::new("Nothing to see in this window! Check the console output!"),
        TextFont {
            font_size: 42.0,
            ..default()
        },
        Node {
            align_self: AlignSelf::FlexEnd,
            ..default()
        },
    ));
}

/// To help with Bevy's automated testing, we want the example to close with an appropriate if the
/// scene fails to load. This is most likely not something you want in your own app.
fn panic_on_fail(scenes: Query<&DynamicSceneRoot>, asset_server: Res<AssetServer>) {
    for scene in &scenes {
        if let Some(LoadState::Failed(err)) = asset_server.get_load_state(&scene.0) {
            panic!("Failed to load scene. {err}");
        }
    }
}
