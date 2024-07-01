//! Demonstrates how to register and access custom attributes on reflected types.

use bevy::reflect::{Reflect, TypeInfo, Typed};
use std::any::TypeId;
use std::ops::RangeInclusive;

fn main() {
    // Bevy supports statically registering custom attribute data on reflected types,
    // which can then be accessed at runtime via the type's `TypeInfo`.
    // Attributes are registered using the `#[reflect(@...)]` syntax,
    // where the `...` is any expression that resolves to a value which implements `Reflect`.
    // Note that these attributes are stored based on their type:
    // if two attributes have the same type, the second one will overwrite the first.

    // Here is an example of registering custom attributes on a type:
    #[derive(Reflect)]
    struct Slider {
        #[reflect(@RangeInclusive::<f32>::new(0.0, 1.0))]
        // Alternatively, we could have used the `0.0..=1.0` syntax,
        // but remember to ensure the type is the one you want!
        #[reflect(@0.0..=1.0_f32)]
        value: f32,
    }

    // Now, we can access the custom attributes at runtime:
    let TypeInfo::Struct(type_info) = Slider::type_info() else {
        panic!("expected struct");
    };

    let field = type_info.field("value").unwrap();

    let range = field.get_attribute::<RangeInclusive<f32>>().unwrap();
    assert_eq!(*range, 0.0..=1.0);

    // And remember that our attributes can be any type that implements `Reflect`:
    #[derive(Reflect)]
    struct Required;

    #[derive(Reflect, PartialEq, Debug)]
    struct Tooltip(String);

    impl Tooltip {
        fn new(text: &str) -> Self {
            Self(text.to_string())
        }
    }

    #[derive(Reflect)]
    #[reflect(@Required, @Tooltip::new("An ID is required!"))]
    struct Id(u8);

    let TypeInfo::TupleStruct(type_info) = Id::type_info() else {
        panic!("expected struct");
    };

    // We can check if an attribute simply exists on our type:
    assert!(type_info.has_attribute::<Required>());

    // We can also get attribute data dynamically:
    let some_type_id = TypeId::of::<Tooltip>();

    let tooltip: &dyn Reflect = type_info.get_attribute_by_id(some_type_id).unwrap();
    assert_eq!(
        tooltip.downcast_ref::<Tooltip>(),
        Some(&Tooltip::new("An ID is required!"))
    );

    // And again, attributes of the same type will overwrite each other:
    #[derive(Reflect)]
    enum Status {
        // This will result in `false` being stored:
        #[reflect(@true)]
        #[reflect(@false)]
        Disabled,
        // This will result in `true` being stored:
        #[reflect(@false)]
        #[reflect(@true)]
        Enabled,
    }

    let TypeInfo::Enum(type_info) = Status::type_info() else {
        panic!("expected enum");
    };

    let disabled = type_info.variant("Disabled").unwrap();
    assert!(!disabled.get_attribute::<bool>().unwrap());

    let enabled = type_info.variant("Enabled").unwrap();
    assert!(enabled.get_attribute::<bool>().unwrap());
}
