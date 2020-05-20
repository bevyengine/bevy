use bevy::{prelude::*, serialization::*};
use legion::serialize::{de::deserialize, ser::serializable_world};
use serde::{Deserialize, Serialize};

fn main() {
    App::build()
        .add_plugin(ScheduleRunnerPlugin::run_once())
        // .add_startup_system(setup)
        .add_startup_system(setup_scene.system())
        .run();
}

#[derive(Serialize, Deserialize)]
struct Test {
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize, Deserialize)]
struct Foo {
    pub value: String,
}

fn setup_scene() {
    let mut component_registry = ComponentRegistry::default();
    component_registry.register::<Test>();
    component_registry.register::<Foo>();

    let mut scene = Scene::default();
    scene.world.insert((), vec![(Test { x: 3.0, y: 4.0 }, Foo { value: "hi".to_string()}),]);
    scene.world.insert((), vec![(Test { x: 3.0, y: 4.0 },)]);

    let serializable_scene = SerializableScene::new(&scene, &component_registry);

    let mut serializer = ron::ser::Serializer::new(Some(ron::ser::PrettyConfig::default()), true);
    serializable_scene.serialize(&mut serializer).unwrap();
    println!("{}", serializer.into_output_string());
}

fn _setup(world: &mut World, resources: &mut Resources) {
    world.insert((), vec![(Test { x: 3.0, y: 4.0 },)]);

    let comp_registrations = [ComponentRegistration::of::<Test>()];

    let ser_helper = SerializeImpl::new(&comp_registrations);
    let serializable = serializable_world(world, &ser_helper);
    println!("JSON");
    let serialized_data = serde_json::to_string(&serializable).unwrap();
    println!("{}", serialized_data);
    println!();

    println!("RON");
    let pretty = ron::ser::PrettyConfig {
        depth_limit: 2,
        separate_tuple_members: true,
        enumerate_arrays: true,
        ..Default::default()
    };
    let s = ron::ser::to_string_pretty(&serializable, pretty.clone()).expect("Serialization failed");
    println!("{}", s);
    println!();


    let universe = resources.get_mut::<Universe>().unwrap();
    println!("JSON (Round Trip)");
    let de_helper = DeserializeImpl::new(&ser_helper.comp_types);
    let mut new_world = universe.create_world();
    let mut deserializer = serde_json::Deserializer::from_str(&serialized_data);
    deserialize(&mut new_world, &de_helper, &mut deserializer).unwrap();
    let round_trip_ser_helper =
        SerializeImpl::new(&comp_registrations);
    let serializable = serializable_world(&new_world, &round_trip_ser_helper);
    let roundtrip_data = serde_json::to_string(&serializable).unwrap();
    println!("{}", roundtrip_data);
    assert_eq!(roundtrip_data, serialized_data);

    println!("RON (Round Trip)");
    let de_helper = DeserializeImpl::new(&ser_helper.comp_types);
    let mut new_world = universe.create_world();
    let mut deserializer = ron::de::Deserializer::from_str(&s).unwrap();
    deserialize(&mut new_world, &de_helper, &mut deserializer).unwrap();
    let round_trip_ser_helper =
        SerializeImpl::new(&comp_registrations);
    let serializable = serializable_world(&new_world, &round_trip_ser_helper);
    let roundtrip_data = ron::ser::to_string_pretty(&serializable, pretty).expect("Serialization failed");
    println!("{}", roundtrip_data);
    assert_eq!(roundtrip_data, s);
}
