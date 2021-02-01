use bevy::{prelude::*, reflect::TypeRegistry, utils::Duration};

/// This example illustrates loading and saving scenes from files
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .register_type::<ComponentA>()
        .register_type::<ComponentB>()
        .add_startup_system(save_scene_system.system())
        .add_startup_system(load_scene_system.system())
        .add_startup_system(infotext_system.system())
        .add_system(print_system.system())
        .run();
}

// Registered components must implement the `Reflect` and `FromResources` traits.
// The `Reflect` trait enables serialization, deserialization, and dynamic property access.
// `Reflect` enable a bunch of cool behaviors, so its worth checking out the dedicated `reflect.rs` example.
// The `FromResources` trait determines how your component is constructed when it loads. For simple use cases you can just
// implement the `Default` trait (which automatically implements FromResources). The simplest registered component just needs
// these two derives:
#[derive(Reflect, Default)]
#[reflect(Component)] // this tells the reflect derive to also reflect component behaviors
struct ComponentA {
    pub x: f32,
    pub y: f32,
}

// Some components have fields that cannot (or should not) be written to scene files. These can be ignored with
// the #[reflect(ignore)] attribute. This is also generally where the `FromResources` trait comes into play.
// `FromResources` gives you access to your App's current ECS `Resources` when you construct your component.
#[derive(Reflect)]
#[reflect(Component)]
struct ComponentB {
    pub value: String,
    #[reflect(ignore)]
    pub time_since_startup: Duration,
}

impl FromResources for ComponentB {
    fn from_resources(resources: &Resources) -> Self {
        let time = resources.get::<Time>().unwrap();
        ComponentB {
            time_since_startup: time.time_since_startup(),
            value: "Default Value".to_string(),
        }
    }
}

fn load_scene_system(asset_server: Res<AssetServer>, mut scene_spawner: ResMut<SceneSpawner>) {
    // Scenes are loaded just like any other asset.
    let scene_handle: Handle<DynamicScene> = asset_server.load("scenes/load_scene_example.scn");

    // SceneSpawner can "spawn" scenes. "Spawning" a scene creates a new instance of the scene in the World with new entity ids.
    // This guarantees that it will not overwrite existing entities.
    scene_spawner.spawn_dynamic(scene_handle);

    // This tells the AssetServer to watch for changes to assets.
    // It enables our scenes to automatically reload in game when we modify their files
    asset_server.watch_for_changes().unwrap();
}

// This system prints all ComponentA components in our world. Try making a change to a ComponentA in load_scene_example.scn.
// You should immediately see the changes appear in the console.
fn print_system(query: Query<(Entity, &ComponentA), Changed<ComponentA>>) {
    for (entity, component_a) in query.iter() {
        println!("  Entity({})", entity.id());
        println!(
            "    ComponentA: {{ x: {} y: {} }}\n",
            component_a.x, component_a.y
        );
    }
}

fn save_scene_system(_world: &mut World, resources: &mut Resources) {
    // Scenes can be created from any ECS World. You can either create a new one for the scene or use the current World.
    let mut world = World::new();
    world.spawn((
        ComponentA { x: 1.0, y: 2.0 },
        ComponentB {
            value: "hello".to_string(),
            ..ComponentB::from_resources(resources)
        },
        Transform::default(),
    ));
    world.spawn((ComponentA { x: 3.0, y: 4.0 },));

    // The TypeRegistry resource contains information about all registered types (including components). This is used to construct scenes.
    let type_registry = resources.get::<TypeRegistry>().unwrap();
    let scene = DynamicScene::from_world(&world, &type_registry);

    // Scenes can be serialized like this:
    println!("{}", scene.serialize_ron(&type_registry).unwrap());

    // TODO: save scene
}

// This is only necessary for the info message in the UI. See examples/ui/text.rs for a standalone text example.
fn infotext_system(commands: &mut Commands, asset_server: Res<AssetServer>) {
    commands.spawn(CameraUiBundle::default()).spawn(TextBundle {
        style: Style {
            align_self: AlignSelf::FlexEnd,
            ..Default::default()
        },
        text: Text {
            value: "Nothing to see in this window! Check the console output!".to_string(),
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            style: TextStyle {
                font_size: 50.0,
                color: Color::WHITE,
                ..Default::default()
            },
        },
        ..Default::default()
    });
}
