use bevy::{
    component_registry::ComponentRegistryContext, input::keyboard::KeyboardInput, prelude::*,
};
use bevy_app::FromResources;

fn main() {
    App::build()
        .add_default_plugins()
        // Registering components informs Bevy that they exist. This allows them to be used when loading scenes
        // This step is only required if you want to load your components from scene files.
        // Unregistered components can still be used in your code, but they won't be serialized / deserialized.
        // In the future registering components will also make them usable from the Bevy editor.
        // The core Bevy plugins already register their components, so you only need this step for custom components.
        .register_component::<ComponentA>()
        .register_component::<ComponentB>()
        .add_startup_system(save_scene_system)
        .add_startup_system(load_scene_system)
        .run();
}

// Registered components must implement the `Properties` and `FromResources` traits.
// The `Properties` trait enables serialization, deserialization, dynamic property access, and change detection.
// `Properties` enable a bunch of cool behaviors, so its worth checking out the dedicated `properties.rs` example.
// The `FromResources` trait determines how your component is constructed.
// For simple use cases you can just implement the `Default` trait (which automatically implements FromResources)
// The simplest registered component just needs these two derives:
#[derive(Properties, Default)]
struct ComponentA {
    pub x: f32,
    pub y: f32,
}

// Some components have fields that cannot (or should not) be written to scene files. These can be ignored with
// the #[property(ignore)] attribute. This is also generally where the `FromResources` trait comes into play.
// This gives you access to your App's current ECS `Resources` when you construct your component.
#[derive(Properties)]
struct ComponentB {
    pub value: String,
    #[property(ignore)]
    pub event_reader: EventReader<KeyboardInput>,
}

impl FromResources for ComponentB {
    fn from_resources(resources: &Resources) -> Self {
        let event_reader = resources.get_event_reader::<KeyboardInput>();
        ComponentB {
            event_reader,
            value: "Default Value".to_string(),
        }
    }
}

fn save_scene_system(world: &mut World, resources: &mut Resources) {
    // Scenes can be created from any ECS World.
    world
        .build()
        .build_entity()
        .add(ComponentA { x: 1.0, y: 2.0 })
        .add(ComponentB {
            value: "hello".to_string(),
            ..ComponentB::from_resources(resources)
        })
        .build_entity()
        .add(ComponentA { x: 3.0, y: 4.0 });

    // The component registry resource contains information about all registered components. This is used to construct scenes.
    let component_registry = resources.get::<ComponentRegistryContext>().unwrap();
    let scene = Scene::from_world(world, &component_registry.value.read().unwrap());

    // Scenes can be serialized like this:
    println!("{}", scene.serialize_ron().unwrap());

    // TODO: save scene
}

fn load_scene_system(world: &mut World, resources: &mut Resources) {
    let asset_server = resources.get::<AssetServer>().unwrap();
    let mut scenes = resources.get_mut::<Assets<Scene>>().unwrap();
    
    // Scenes are loaded just like any other asset.
    let scene_handle: Handle<Scene> = asset_server
        .load_sync(&mut scenes, "assets/scene/load_scene_example.scn")
        .unwrap();
    let scene = scenes.get(&scene_handle).unwrap();

    // Scenes can be added to any ECS World. Adding scenes also uses the component registry. 
    let component_registry = resources.get::<ComponentRegistryContext>().unwrap();
    scene
        .add_to_world(world, resources, &component_registry.value.read().unwrap())
        .unwrap();
}
