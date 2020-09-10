use bevy::{prelude::*, type_registry::TypeRegistry};

/// This example illustrates loading and saving scenes from files
fn main() {
    App::build()
        .add_default_plugins()
        // Registering components informs Bevy that they exist. This allows them to be used when loading scenes
        // This step is only required if you want to load your components from scene files.
        // Unregistered components can still be used in your code, but they will be ignored during scene save/load.
        // In the future registering components will also make them usable from the Bevy editor.
        // The core Bevy plugins already register their components, so you only need this step for custom components.
        .register_component::<ComponentA>()
        .register_component::<ComponentB>()
        .add_startup_system(save_scene_system.thread_local_system())
        .add_startup_system(load_scene_system.system())
        .add_startup_system(infotext_system.system())
        .add_system(print_system.system())
        .run();
}

// Registered components must implement the `Properties` and `FromResources` traits.
// The `Properties` trait enables serialization, deserialization, dynamic property access, and change detection.
// `Properties` enable a bunch of cool behaviors, so its worth checking out the dedicated `properties.rs` example.
// The `FromResources` trait determines how your component is constructed when it loads. For simple use cases you can just
// implement the `Default` trait (which automatically implements FromResources). The simplest registered component just needs
// these two derives:
#[derive(Properties, Default)]
struct ComponentA {
    pub x: f32,
    pub y: f32,
}

// Some components have fields that cannot (or should not) be written to scene files. These can be ignored with
// the #[property(ignore)] attribute. This is also generally where the `FromResources` trait comes into play.
// `FromResources` gives you access to your App's current ECS `Resources` when you construct your component.
#[derive(Properties)]
struct ComponentB {
    pub value: String,
    #[property(ignore)]
    pub time_since_startup: std::time::Duration,
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
    let scene_handle: Handle<Scene> = asset_server
        .load("assets/scenes/load_scene_example.scn")
        .unwrap();

    // SceneSpawner can "spawn" scenes. "Spawning" a scene creates a new instance of the scene in the World with new entity ids.
    // This guarantees that it will not overwrite existing entities.
    scene_spawner.spawn(scene_handle);

    // This tells the AssetServer to watch for changes to assets.
    // It enables our scenes to automatically reload in game when we modify their files
    asset_server.watch_for_changes().unwrap();
}

// Using SceneSpawner.spawn() queues up the scene to be spawned. It will be added to the World at the beginning of the next update. However if
// you need scenes to load immediately, you can use the following approach. But be aware that this takes full control of the ECS world
// and therefore blocks other parallel systems from executing until it finishes. In most cases you should use the SceneSpawner.spawn() method.
#[allow(dead_code)]
fn load_scene_right_now_system(world: &mut World, resources: &mut Resources) {
    let scene_handle: Handle<Scene> = {
        let asset_server = resources.get::<AssetServer>().unwrap();
        let mut scenes = resources.get_mut::<Assets<Scene>>().unwrap();
        asset_server
            .load_sync(&mut scenes, "assets/scenes/load_scene_example.scn")
            .unwrap()
    };
    let mut scene_spawner = resources.get_mut::<SceneSpawner>().unwrap();
    scene_spawner
        .spawn_sync(world, resources, scene_handle)
        .unwrap();
}

// This system prints all ComponentA components in our world. Try making a change to a ComponentA in load_scene_example.scn.
// You should immediately see the changes appear in the console.
fn print_system(mut query: Query<(Entity, Changed<ComponentA>)>) {
    for (entity, component_a) in &mut query.iter() {
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
    ));
    world.spawn((ComponentA { x: 3.0, y: 4.0 },));

    // The component registry resource contains information about all registered components. This is used to construct scenes.
    let type_registry = resources.get::<TypeRegistry>().unwrap();
    let scene = Scene::from_world(&world, &type_registry.component.read());

    // Scenes can be serialized like this:
    println!(
        "{}",
        scene.serialize_ron(&type_registry.property.read()).unwrap()
    );

    // TODO: save scene
}

// This is only necessary for the info message in the UI. See examples/ui/text.rs for a standalone text example.
fn infotext_system(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font_handle = asset_server.load("assets/fonts/FiraSans-Bold.ttf").unwrap();
    commands
        .spawn(UiCameraComponents::default())
        .spawn(TextComponents {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                ..Default::default()
            },
            text: Text {
                value: "Nothing to see in this window! Check the console output!".to_string(),
                font: font_handle,
                style: TextStyle {
                    font_size: 50.0,
                    color: Color::WHITE,
                },
            },
            ..Default::default()
        });
}
