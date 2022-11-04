//! This example illustrates loading scenes from files.
use std::fs::File;
use std::io::Write;

use bevy::{prelude::*, tasks::IoTaskPool, utils::Duration};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            // This tells the AssetServer to watch for changes to assets.
            // It enables our scenes to automatically reload in game when we modify their files.
            watch_for_changes: true,
            ..default()
        }))
        .register_type::<ComponentA>()
        .register_type::<ComponentB>()
        .add_startup_system(save_scene_system)
        .add_startup_system(load_scene_system)
        .add_startup_system(infotext_system)
        .add_system(log_system)
        .run();
}

// Registered components must implement the `Reflect` and `FromWorld` traits.
// The `Reflect` trait enables serialization, deserialization, and dynamic property access.
// `Reflect` enable a bunch of cool behaviors, so its worth checking out the dedicated `reflect.rs`
// example. The `FromWorld` trait determines how your component is constructed when it loads.
// For simple use cases you can just implement the `Default` trait (which automatically implements
// FromResources). The simplest registered component just needs these two derives:
#[derive(Component, Reflect, Default)]
#[reflect(Component)] // this tells the reflect derive to also reflect component behaviors
struct ComponentA {
    pub x: f32,
    pub y: f32,
}

// Some components have fields that cannot (or should not) be written to scene files. These can be
// ignored with the #[reflect(skip_serializing)] attribute. This is also generally where the `FromWorld`
// trait comes into play. `FromWorld` gives you access to your App's current ECS `Resources`
// when you construct your component.
#[derive(Component, Reflect)]
#[reflect(Component)]
struct ComponentB {
    pub value: String,
    #[reflect(skip_serializing)]
    pub _time_since_startup: Duration,
}

impl FromWorld for ComponentB {
    fn from_world(world: &mut World) -> Self {
        let time = world.resource::<Time>();
        ComponentB {
            _time_since_startup: time.elapsed(),
            value: "Default Value".to_string(),
        }
    }
}

// The initial scene file will be loaded below and not change when the scene is saved
const SCENE_FILE_PATH: &str = "scenes/load_scene_example.scn.ron";

// The new, updated scene data will be saved here so that you can see the changes
const NEW_SCENE_FILE_PATH: &str = "scenes/load_scene_example-new.scn.ron";

fn load_scene_system(mut commands: Commands, asset_server: Res<AssetServer>) {
    // "Spawning" a scene bundle creates a new entity and spawns new instances
    // of the given scene's entities as children of that entity.
    commands.spawn(DynamicSceneBundle {
        // Scenes are loaded just like any other asset.
        scene: asset_server.load(SCENE_FILE_PATH),
        ..default()
    });
}

// This system logs all ComponentA components in our world. Try making a change to a ComponentA in
// load_scene_example.scn. You should immediately see the changes appear in the console.
fn log_system(query: Query<(Entity, &ComponentA), Changed<ComponentA>>) {
    for (entity, component_a) in &query {
        info!("  Entity({})", entity.index());
        info!(
            "    ComponentA: {{ x: {} y: {} }}\n",
            component_a.x, component_a.y
        );
    }
}

fn save_scene_system(world: &mut World) {
    // Scenes can be created from any ECS World. You can either create a new one for the scene or
    // use the current World.
    let mut scene_world = World::new();
    let mut component_b = ComponentB::from_world(world);
    component_b.value = "hello".to_string();
    scene_world.spawn((
        component_b,
        ComponentA { x: 1.0, y: 2.0 },
        Transform::IDENTITY,
    ));
    scene_world.spawn(ComponentA { x: 3.0, y: 4.0 });

    // The TypeRegistry resource contains information about all registered types (including
    // components). This is used to construct scenes.
    let type_registry = world.resource::<AppTypeRegistry>();
    let scene = DynamicScene::from_world(&scene_world, type_registry);

    // Scenes can be serialized like this:
    let serialized_scene = scene.serialize_ron(type_registry).unwrap();

    // Showing the scene in the console
    info!("{}", serialized_scene);

    // Writing the scene to a new file. Using a task to avoid calling the filesystem APIs in a system
    // as they are blocking
    // This can't work in WASM as there is no filesystem access
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

// This is only necessary for the info message in the UI. See examples/ui/text.rs for a standalone
// text example.
fn infotext_system(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(
        TextBundle::from_section(
            "Nothing to see in this window! Check the console output!",
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 50.0,
                color: Color::WHITE,
            },
        )
        .with_style(Style {
            align_self: AlignSelf::FlexEnd,
            ..default()
        }),
    );
}
