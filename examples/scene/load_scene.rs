use bevy::{component_registry::ComponentRegistryContext, prelude::*};

fn main() {
    App::build()
        .add_default_plugins()
        // Registering components informs Bevy that they exist. This allows them to be used when loading scenes
        // This step is only required if you want to load your components from scene files.
        // In the future registered components will also be usable from the Bevy editor.
        .register_component::<Test>()
        .register_component::<Foo>()
        .add_startup_system(load_scene)
        // .add_startup_system(serialize_scene)
        .run();
}

#[derive(Properties, Default)]
struct Test {
    pub x: f32,
    pub y: f32,
}

#[derive(Properties, Default)]
struct Foo {
    pub value: String,
}

fn load_scene(world: &mut World, resources: &mut Resources) {
    let asset_server = resources.get::<AssetServer>().unwrap();
    let mut scenes = resources.get_mut::<Assets<Scene>>().unwrap();
    let component_registry = resources.get::<ComponentRegistryContext>().unwrap();

    let scene_handle: Handle<Scene> = asset_server
        .load_sync(&mut scenes, "assets/scene/load_scene_example.scn")
        .unwrap();
    let scene = scenes.get(&scene_handle).unwrap();
    scene
        .add_to_world(world, resources, &component_registry.value.read().unwrap())
        .unwrap();
}

#[allow(dead_code)]
fn serialize_scene(world: &mut World, resources: &mut Resources) {
    let component_registry = resources.get::<ComponentRegistryContext>().unwrap();
    world
        .build()
        .add(Test { x: 1.0, y: 2.0 })
        .add(Foo {
            value: "hello".to_string(),
        })
        .build_entity()
        .add(Test { x: 3.0, y: 4.0 });

    let scene = Scene::from_world(world, &component_registry.value.read().unwrap());
    println!("{}", scene.serialize_ron().unwrap());
}
