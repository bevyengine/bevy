use bevy::{prelude::*, serialization::*};
use legion::serialize::{de::deserialize, ser::serializable_world};
use serde::{Deserialize, Serialize};

fn main() {
    App::build()
        .add_plugin(ScheduleRunnerPlugin::run_once())
        .add_startup_system(setup)
        .run();
}

#[derive(Serialize, Deserialize)]
struct Test {
    pub x: f32,
    pub y: f32,
}

fn setup(world: &mut World, resources: &mut Resources) {
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
    let de_helper = DeserializeImpl::new(&ser_helper.comp_types, ser_helper.entity_map.clone());
    let mut new_world = universe.create_world();
    let mut deserializer = serde_json::Deserializer::from_str(&serialized_data);
    deserialize(&mut new_world, &de_helper, &mut deserializer).unwrap();
    let round_trip_ser_helper =
        SerializeImpl::new_with_map(&comp_registrations, de_helper.entity_map.into_inner());
    let serializable = serializable_world(&new_world, &round_trip_ser_helper);
    let roundtrip_data = serde_json::to_string(&serializable).unwrap();
    println!("{}", roundtrip_data);
    assert_eq!(roundtrip_data, serialized_data);

    println!("RON (Round Trip)");
    let de_helper = DeserializeImpl::new(&ser_helper.comp_types, ser_helper.entity_map.clone());
    let mut new_world = universe.create_world();
    let mut deserializer = ron::de::Deserializer::from_str(&s).unwrap();
    deserialize(&mut new_world, &de_helper, &mut deserializer).unwrap();
    let round_trip_ser_helper =
        SerializeImpl::new_with_map(&comp_registrations, de_helper.entity_map.into_inner());
    let serializable = serializable_world(&new_world, &round_trip_ser_helper);
    let roundtrip_data = ron::ser::to_string_pretty(&serializable, pretty).expect("Serialization failed");
    println!("{}", roundtrip_data);
    assert_eq!(roundtrip_data, s);
}
