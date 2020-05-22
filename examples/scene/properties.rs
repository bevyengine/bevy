use bevy::{
    prelude::*,
    property::SerializableProperties,
    scene::{DynamicScene, SceneEntity},
};
use serde::ser::Serialize;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

#[derive(Properties, Default)]
pub struct Test {
    a: usize,
    b: String,
    c: f32,
}

fn setup() {
    let mut test = Test {
        a: 1,
        b: "hi".to_string(),
        c: 1.0,
    };

    test.set_prop_val::<usize>("a", 2);
    assert_eq!(test.a, 2);
    let x: u32 = 3;
    test.set_prop("a", &x);
    assert_eq!(test.a, 3);

    test.set_prop_val::<f32>("c", 2.0);
    let x: f64 = 3.0;
    test.set_prop("c", &x);
    assert_eq!(test.c, 3.0);

    let mut patch = DynamicProperties::default();
    patch.set::<usize>("a", 3);
    test.apply(&patch);

    assert_eq!(test.a, 3);

    let ser = SerializableProperties { props: &test };

    let mut serializer = ron::ser::Serializer::new(Some(ron::ser::PrettyConfig::default()), false);
    ser.serialize(&mut serializer).unwrap();
    let ron_string = serializer.into_output_string();
    println!("{}", ron_string);

    // let dynamic_scene = DynamicScene {
    //     entities: vec![SceneEntity {
    //         entity: 12345,
    //         components: vec![patch],
    //     }],
    // };

    // let mut serializer = ron::ser::Serializer::new(Some(ron::ser::PrettyConfig::default()), false);
    // dynamic_scene.entities.serialize(&mut serializer).unwrap();
    // println!("{}", serializer.into_output_string());

    let mut deserializer = ron::de::Deserializer::from_str(&ron_string).unwrap();
}
