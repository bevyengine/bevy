//! This example demonstrates the use of dynamic types in Bevy's reflection system.

use bevy::reflect::{
    reflect_trait, serde::TypedReflectDeserializer, std_traits::ReflectDefault, DynamicArray,
    DynamicEnum, DynamicList, DynamicMap, DynamicStruct, DynamicTuple, DynamicTupleStruct,
    DynamicVariant, FromReflect, Reflect, ReflectFromReflect, ReflectRef, TypeRegistry, Typed,
};
use serde::de::DeserializeSeed;
use std::collections::HashMap;

fn main() {
    #[derive(Reflect, Default)]
    #[reflect(Identifiable, Default)]
    struct Player {
        id: u32,
    }

    #[reflect_trait]
    trait Identifiable {
        fn id(&self) -> u32;
    }

    impl Identifiable for Player {
        fn id(&self) -> u32 {
            self.id
        }
    }

    // Normally, when instantiating a type, you get back exactly that type.
    // This is because the type is known at compile time.
    // We call this the "concrete" or "canonical" type.
    let player: Player = Player { id: 123 };

    // When working with reflected types, however, we often "erase" this type information
    // using the `Reflect` trait object.
    // The underlying type is still the same (in this case, `Player`),
    // but now we've hidden that information from the compiler.
    let reflected: Box<dyn Reflect> = Box::new(player);

    // Because it's the same type under the hood, we can still downcast it back to the original type.
    assert!(reflected.downcast_ref::<Player>().is_some());

    // But now let's "clone" our type using `Reflect::clone_value`.
    let cloned: Box<dyn Reflect> = reflected.clone_value();

    // If we try to downcast back to `Player`, we'll get an error.
    assert!(cloned.downcast_ref::<Player>().is_none());

    // Why is this?
    // Well the reason is that `Reflect::clone_value` actually creates a dynamic type.
    // Since `Player` is a struct, we actually get a `DynamicStruct` back.
    assert!(cloned.is::<DynamicStruct>());

    // This dynamic type is used to represent (or "proxy") the original type,
    // so that we can continue to access its fields and overall structure.
    let ReflectRef::Struct(cloned_ref) = cloned.reflect_ref() else {
        panic!("expected struct")
    };
    let id = cloned_ref.field("id").unwrap().downcast_ref::<u32>();
    assert_eq!(id, Some(&123));

    // It also enables us to create a representation of a type without having compile-time
    // access to the actual type. This is how the reflection deserializers work.
    // They generally can't know how to construct a type ahead of time,
    // so they instead build and return these dynamic representations.
    let input = "(id: 123)";
    let mut registry = TypeRegistry::default();
    registry.register::<Player>();
    let registration = registry.get(std::any::TypeId::of::<Player>()).unwrap();
    let deserialized = TypedReflectDeserializer::new(registration, &registry)
        .deserialize(&mut ron::Deserializer::from_str(input).unwrap())
        .unwrap();

    // Our deserialized output is a `DynamicStruct` that proxies/represents a `Player`.
    assert!(deserialized.downcast_ref::<DynamicStruct>().is_some());
    assert!(deserialized.represents::<Player>());

    // And while this does allow us to access the fields and structure of the type,
    // there may be instances where we need the actual type.
    // For example, if we want to convert our `dyn Reflect` into a `dyn Identifiable`,
    // we can't use the `DynamicStruct` proxy.
    let reflect_identifiable = registration
        .data::<ReflectIdentifiable>()
        .expect("`ReflectIdentifiable` should be registered");

    // This fails since the underlying type of `deserialized` is `DynamicStruct` and not `Player`.
    assert!(reflect_identifiable
        .get(deserialized.as_reflect())
        .is_none());

    // So how can we go from a dynamic type to a concrete type?
    // There are two ways:

    // 1. Using `Reflect::apply`.
    {
        // If you know the type at compile time, you can construct a new value and apply the dynamic
        // value to it.
        let mut value = Player::default();
        value.apply(deserialized.as_reflect());
        assert_eq!(value.id, 123);

        // If you don't know the type at compile time, you need a dynamic way of constructing
        // an instance of the type. One such way is to use the `ReflectDefault` type data.
        let reflect_default = registration
            .data::<ReflectDefault>()
            .expect("`ReflectDefault` should be registered");

        let mut value: Box<dyn Reflect> = reflect_default.default();
        value.apply(deserialized.as_reflect());

        let identifiable: &dyn Identifiable = reflect_identifiable.get(value.as_reflect()).unwrap();
        assert_eq!(identifiable.id(), 123);
    }

    // 2. Using `FromReflect`
    {
        // If you know the type at compile time, you can use the `FromReflect` trait to convert the
        // dynamic value into the concrete type directly.
        let value: Player = Player::from_reflect(deserialized.as_reflect()).unwrap();
        assert_eq!(value.id, 123);

        // If you don't know the type at compile time, you can use the `ReflectFromReflect` type data
        // to perform the conversion dynamically.
        let reflect_from_reflect = registration
            .data::<ReflectFromReflect>()
            .expect("`ReflectFromReflect` should be registered");

        let value: Box<dyn Reflect> = reflect_from_reflect
            .from_reflect(deserialized.as_reflect())
            .unwrap();
        let identifiable: &dyn Identifiable = reflect_identifiable.get(value.as_reflect()).unwrap();
        assert_eq!(identifiable.id(), 123);
    }

    // Lastly, while dynamic types are commonly generated via reflection methods like
    // `Reflect::clone_value` or via the reflection deserializers,
    // you can also construct them manually.
    let mut my_dynamic_list = DynamicList::default();
    my_dynamic_list.push(1u32);
    my_dynamic_list.push(2u32);
    my_dynamic_list.push(3u32);

    // This is useful when you just need to apply some subset of changes to a type.
    let mut my_list: Vec<u32> = Vec::new();
    my_list.apply(&my_dynamic_list);
    assert_eq!(my_list, vec![1, 2, 3]);

    // And if you want it to actually proxy a type, you can configure it to do that as well:
    assert!(!my_dynamic_list.as_reflect().represents::<Vec<u32>>());
    my_dynamic_list.set_represented_type(Some(<Vec<u32>>::type_info()));
    assert!(my_dynamic_list.as_reflect().represents::<Vec<u32>>());

    // ============================= REFERENCE ============================= //
    // For reference, here are all the available dynamic types:

    // 1. `DynamicTuple`
    {
        let mut dynamic_tuple = DynamicTuple::default();
        dynamic_tuple.insert(1u32);
        dynamic_tuple.insert(2u32);
        dynamic_tuple.insert(3u32);

        let mut my_tuple: (u32, u32, u32) = (0, 0, 0);
        my_tuple.apply(&dynamic_tuple);
        assert_eq!(my_tuple, (1, 2, 3));
    }

    // 2. `DynamicArray`
    {
        let dynamic_array = DynamicArray::from_vec(vec![1u32, 2u32, 3u32]);

        let mut my_array = [0u32; 3];
        my_array.apply(&dynamic_array);
        assert_eq!(my_array, [1, 2, 3]);
    }

    // 3. `DynamicList`
    {
        let mut dynamic_list = DynamicList::default();
        dynamic_list.push(1u32);
        dynamic_list.push(2u32);
        dynamic_list.push(3u32);

        let mut my_list: Vec<u32> = Vec::new();
        my_list.apply(&dynamic_list);
        assert_eq!(my_list, vec![1, 2, 3]);
    }

    // 4. `DynamicMap`
    {
        let mut dynamic_map = DynamicMap::default();
        dynamic_map.insert("x", 1u32);
        dynamic_map.insert("y", 2u32);
        dynamic_map.insert("z", 3u32);

        let mut my_map: HashMap<&str, u32> = HashMap::new();
        my_map.apply(&dynamic_map);
        assert_eq!(my_map.get("x"), Some(&1));
        assert_eq!(my_map.get("y"), Some(&2));
        assert_eq!(my_map.get("z"), Some(&3));
    }

    // 5. `DynamicStruct`
    {
        #[derive(Reflect, Default, Debug, PartialEq)]
        struct MyStruct {
            x: u32,
            y: u32,
            z: u32,
        }

        let mut dynamic_struct = DynamicStruct::default();
        dynamic_struct.insert("x", 1u32);
        dynamic_struct.insert("y", 2u32);
        dynamic_struct.insert("z", 3u32);

        let mut my_struct = MyStruct::default();
        my_struct.apply(&dynamic_struct);
        assert_eq!(my_struct, MyStruct { x: 1, y: 2, z: 3 });
    }

    // 6. `DynamicTupleStruct`
    {
        #[derive(Reflect, Default, Debug, PartialEq)]
        struct MyTupleStruct(u32, u32, u32);

        let mut dynamic_tuple_struct = DynamicTupleStruct::default();
        dynamic_tuple_struct.insert(1u32);
        dynamic_tuple_struct.insert(2u32);
        dynamic_tuple_struct.insert(3u32);

        let mut my_tuple_struct = MyTupleStruct::default();
        my_tuple_struct.apply(&dynamic_tuple_struct);
        assert_eq!(my_tuple_struct, MyTupleStruct(1, 2, 3));
    }

    // 7. `DynamicEnum`
    {
        #[derive(Reflect, Default, Debug, PartialEq)]
        enum MyEnum {
            #[default]
            Empty,
            Xyz(u32, u32, u32),
        }

        let mut values = DynamicTuple::default();
        values.insert(1u32);
        values.insert(2u32);
        values.insert(3u32);

        let dynamic_variant = DynamicVariant::Tuple(values);
        let dynamic_enum = DynamicEnum::new("Xyz", dynamic_variant);

        let mut my_enum = MyEnum::default();
        my_enum.apply(&dynamic_enum);
        assert_eq!(my_enum, MyEnum::Xyz(1, 2, 3));
    }
}
