use bevy::prelude::*;
use bevy_props::{DynamicScene, SerializableProps, SceneEntity};
use serde::ser::Serialize;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

#[derive(Properties)]
pub struct Test {
    a: usize,
    b: String,
}

fn setup() {
    let mut test = Test {
        a: 1,
        b: "hi".to_string(),
    };

    test.set_prop_val::<usize>("a", 2);
    assert_eq!(test.a, 2);

    let mut patch = DynamicProperties::default();
    patch.set::<usize>("a", 3);
    test.apply(&patch);

    assert_eq!(test.a, 3);


    let ser = SerializableProps {
        props: &test,
    };

    let mut serializer = ron::ser::Serializer::new(Some(ron::ser::PrettyConfig::default()), false);
    ser.serialize(&mut serializer).unwrap();
    println!("{}", serializer.into_output_string());

    let dynamic_scene = DynamicScene {
       entities: vec![
           SceneEntity {
               entity: 12345,
               components: vec![
                patch
               ]
           }
       ] 
    };
    
    let mut serializer = ron::ser::Serializer::new(Some(ron::ser::PrettyConfig::default()), false);
    dynamic_scene.entities.serialize(&mut serializer).unwrap();
    println!("{}", serializer.into_output_string());
}
