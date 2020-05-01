use bevy::{prelude::*, serialization::*};
use serde::{Deserialize, Serialize};
use type_uuid::TypeUuid;
fn main() {
    let mut app = App::build();
    app.add_default_plugins().add_startup_system(setup);

    let comp_registrations = [ComponentRegistration::of::<Test>()];

    let tag_registrations = [];

    let ser_helper = SerializeImpl::new(&comp_registrations, &tag_registrations);
    let serializable = legion::serialize::ser::serializable_world(&app.world(), &ser_helper);
    let serialized_data = serde_json::to_string(&serializable).unwrap();
    println!("{}", serialized_data);
    let de_helper = DeserializeImpl::new(
        ser_helper.comp_types,
        ser_helper.tag_types,
        ser_helper.entity_map,
    );

    let mut new_world = app.universe().create_world();
    let mut deserializer = serde_json::Deserializer::from_str(&serialized_data);
    legion::serialize::de::deserialize(&mut new_world, &de_helper, &mut deserializer).unwrap();
    let ser_helper = SerializeImpl::new_with_map(
        &comp_registrations,
        &tag_registrations,
        de_helper.entity_map.into_inner(),
    );
    let serializable = legion::serialize::ser::serializable_world(&new_world, &ser_helper);
    let roundtrip_data = serde_json::to_string(&serializable).unwrap();
    println!("{}", roundtrip_data);
    assert_eq!(roundtrip_data, serialized_data);
}

#[derive(Serialize, Deserialize, TypeUuid)]
#[uuid = "14dec17f-ae14-40a3-8e44-e487fc423287"]
struct Test {
    pub x: f32,
    pub y: f32,
}

fn setup(world: &mut World, _resources: &mut Resources) {
    // plane
    world.insert((), vec![(Test { x: 3.0, y: 4.0 },)]);
}
