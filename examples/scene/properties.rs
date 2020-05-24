use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

#[derive(Properties, Default)]
pub struct Test {
    a: usize,
    nested: Nested,
}

#[derive(Properties, Default)]
pub struct Nested {
    b: usize,
}

fn setup() {
    let mut test = Test {
        a: 1,
        nested: Nested { b: 8 },
    };

    // You can set a property value like this. The type must match exactly or this will fail.
    test.set_prop_val::<usize>("a", 2);
    assert_eq!(test.a, 2);

    // You can also set properties dynamically. set_prop accepts any type that implements Property
    let x: u32 = 3;
    test.set_prop("a", &x);
    assert_eq!(test.a, 3);

    // DynamicProperties also implements the Properties trait.
    let mut patch = DynamicProperties::default();
    patch.set::<usize>("a", 4);

    // You can "apply" Properties on top of other Properties. This will only set properties with the same name and type.
    // You can use this to "patch" your components with new values.
    test.apply(&patch);
    assert_eq!(test.a, 4);

    // Properties implement the serde Serialize trait. You don't need to derive it yourself!
    let pretty_config = ron::ser::PrettyConfig::default().with_decimal_floats(true);

    let ron_string = ron::ser::to_string_pretty(&test, pretty_config.clone()).unwrap();
    println!("{}\n", ron_string);

    // Dynamic properties can be deserialized
    let dynamic_properties = ron::from_str::<DynamicProperties>(&ron_string).unwrap();
    let round_tripped = ron::ser::to_string_pretty(&dynamic_properties, pretty_config).unwrap();
    println!("{}", round_tripped);
    assert_eq!(ron_string, round_tripped);

    // This means you can patch Properties with dynamic properties deserialized from a string
    test.apply(&dynamic_properties);
}
