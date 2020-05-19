use bevy::{prelude::*, serialization::*};
use serde::{Deserialize, Serialize};
use ron::ser::{to_string_pretty, PrettyConfig};
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

    let tag_registrations = [];

    let ser_helper = SerializeImpl::new(&comp_registrations, &tag_registrations);
    let serializable = legion::serialize::ser::serializable_world(world, &ser_helper);
    let serialized_data = serde_json::to_string(&serializable).unwrap();
    // println!("{}", serialized_data);
    let pretty = PrettyConfig {
        depth_limit: 2,
        separate_tuple_members: true,
        enumerate_arrays: true,
        ..Default::default()
    };
    let s = to_string_pretty(&serializable, pretty).expect("Serialization failed");
    println!("{}", s);
    let de_helper = DeserializeImpl::new(
        ser_helper.comp_types,
        ser_helper.tag_types,
        ser_helper.entity_map,
    );

    let universe = resources.get_mut::<Universe>().unwrap();
    let mut new_world = universe.create_world();
    let mut deserializer = serde_json::Deserializer::from_str(&serialized_data);
    legion::serialize::de::deserialize(&mut new_world, &de_helper, &mut deserializer).unwrap();
    let ser_helper = SerializeImpl::new_with_map(
        &comp_registrations,
        &tag_registrations,
        de_helper.entity_map.into_inner(),
    );
    let serializable = legion::serialize::ser::serializable_world(&new_world, &ser_helper);

    let roundtrip_data = serde_json::to_string(&serializable).unwrap();
    // println!("{}", roundtrip_data);
    assert_eq!(roundtrip_data, serialized_data);
}
