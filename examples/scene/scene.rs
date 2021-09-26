use bevy::{prelude::*, reflect::TypeRegistry, utils::Duration};

/// This example illustrates loading and saving scenes from files
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .register_type::<ComponentA>()
        .register_type::<ComponentB>()
        .add_startup_system(save_scene_system.exclusive_system())
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
// FromWorld). The simplest registered component just needs these two derives:
#[derive(Reflect, Default)]
#[reflect(Component)] // this tells the reflect derive to also reflect component behaviors
struct ComponentA {
    pub x: f32,
    pub y: f32,
}

// Some components have fields that cannot (or should not) be written to scene files. These can be
// ignored with the #[reflect(ignore)] attribute. This is also generally where the `FromWorld`
// trait comes into play. `FromWorld` gives you access to your App's current ECS `Resources`
// when you construct your component.
#[derive(Reflect)]
#[reflect(Component)]
struct ComponentB {
    pub value: String,
    #[reflect(ignore)]
    pub _time_since_startup: Duration,
}

impl FromWorld for ComponentB {
    fn from_world(world: &mut World) -> Self {
        let time = world.get_resource::<Time>().unwrap();
        ComponentB {
            _time_since_startup: time.time_since_startup(),
            value: "Default Value".to_string(),
        }
    }
}

fn load_scene_system(asset_server: Res<AssetServer>, mut scene_spawner: ResMut<SceneSpawner>) {
    // Scenes are loaded just like any other asset.
    let scene_handle: Handle<DynamicScene> = asset_server.load("scenes/load_scene_example.scn.ron");

    // SceneSpawner can "spawn" scenes. "Spawning" a scene creates a new instance of the scene in
    // the World with new entity ids. This guarantees that it will not overwrite existing
    // entities.
    scene_spawner.spawn_dynamic(scene_handle);

    // This tells the AssetServer to watch for changes to assets.
    // It enables our scenes to automatically reload in game when we modify their files
    asset_server.watch_for_changes().unwrap();
}

// This system logs all ComponentA components in our world. Try making a change to a ComponentA in
// load_scene_example.scn. You should immediately see the changes appear in the console.
fn log_system(query: Query<(Entity, &ComponentA), Changed<ComponentA>>) {
    for (entity, component_a) in query.iter() {
        info!("  Entity({})", entity.id());
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
    scene_world.spawn().insert_bundle((
        component_b,
        ComponentA { x: 1.0, y: 2.0 },
        Transform::identity(),
    ));
    scene_world
        .spawn()
        .insert_bundle((ComponentA { x: 3.0, y: 4.0 },));

    // The TypeRegistry resource contains information about all registered types (including
    // components). This is used to construct scenes.
    let type_registry = world.get_resource::<TypeRegistry>().unwrap();
    let scene = DynamicScene::from_world(&scene_world, type_registry);

    // Scenes can be serialized like this:
    info!("{}", scene.serialize_ron(type_registry).unwrap());

    // TODO: save scene
}

// This is only necessary for the info message in the UI. See examples/ui/text.rs for a standalone
// text example.
fn infotext_system(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(UiCameraBundle::default());
    commands.spawn_bundle(TextBundle {
        style: Style {
            align_self: AlignSelf::FlexEnd,
            ..Default::default()
        },
        text: Text::with_section(
            "Nothing to see in this window! Check the console output!",
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 50.0,
                color: Color::WHITE,
            },
            Default::default(),
        ),
        ..Default::default()
    });
}
