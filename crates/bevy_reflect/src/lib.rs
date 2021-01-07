mod list;
mod map;
mod path;
mod reflect;
mod struct_trait;
mod tuple;
mod tuple_struct;
mod type_registry;
mod type_uuid;
mod impls {
    #[cfg(feature = "bevy_app")]
    mod bevy_app;
    #[cfg(feature = "bevy_ecs")]
    mod bevy_ecs;
    #[cfg(feature = "glam")]
    mod glam;
    #[cfg(feature = "smallvec")]
    mod smallvec;
    mod std;

    #[cfg(feature = "bevy_app")]
    pub use self::bevy_app::*;
    #[cfg(feature = "bevy_ecs")]
    pub use self::bevy_ecs::*;
    #[cfg(feature = "glam")]
    pub use self::glam::*;
    #[cfg(feature = "smallvec")]
    pub use self::smallvec::*;
    pub use self::std::*;
}

pub mod serde;
pub mod prelude {
    #[cfg(feature = "bevy_ecs")]
    pub use crate::ReflectComponent;
    #[cfg(feature = "bevy_app")]
    pub use crate::RegisterTypeBuilder;
    pub use crate::{
        reflect_trait, GetField, GetTupleStructField, Reflect, ReflectDeserialize, Struct,
        TupleStruct,
    };
}

pub use impls::*;
pub use list::*;
pub use map::*;
pub use path::*;
pub use reflect::*;
pub use struct_trait::*;
pub use tuple::*;
pub use tuple_struct::*;
pub use type_registry::*;
pub use type_uuid::*;

pub use bevy_reflect_derive::*;
pub use erased_serde;

#[cfg(test)]
mod tests {
    use ::serde::de::DeserializeSeed;
    use bevy_utils::HashMap;
    use ron::{
        ser::{to_string_pretty, PrettyConfig},
        Deserializer,
    };

    use crate::serde::{ReflectDeserializer, ReflectSerializer};

    use super::*;
    #[test]
    fn reflect_struct() {
        #[derive(Reflect)]
        struct Foo {
            a: u32,
            b: f32,
            c: Bar,
        }
        #[derive(Reflect)]
        struct Bar {
            x: u32,
        }

        let mut foo = Foo {
            a: 42,
            b: 3.14,
            c: Bar { x: 1 },
        };

        let a = *foo.get_field::<u32>("a").unwrap();
        assert_eq!(a, 42);

        *foo.get_field_mut::<u32>("a").unwrap() += 1;
        assert_eq!(foo.a, 43);

        let bar = foo.get_field::<Bar>("c").unwrap();
        assert_eq!(bar.x, 1);

        // nested retrieval
        let c = foo.field("c").unwrap();
        if let ReflectRef::Struct(value) = c.reflect_ref() {
            assert_eq!(*value.get_field::<u32>("x").unwrap(), 1);
        } else {
            panic!("Expected a struct.");
        }

        // patch Foo with a dynamic struct
        let mut dynamic_struct = DynamicStruct::default();
        dynamic_struct.insert("a", 123u32);
        dynamic_struct.insert("should_be_ignored", 456);

        foo.apply(&dynamic_struct);
        assert_eq!(foo.a, 123);
    }

    #[test]
    fn reflect_map() {
        #[derive(Reflect, Hash)]
        #[reflect(Hash)]
        struct Foo {
            a: u32,
            b: String,
        }

        let key_a = Foo {
            a: 1,
            b: "k1".to_string(),
        };

        let key_b = Foo {
            a: 1,
            b: "k1".to_string(),
        };

        let key_c = Foo {
            a: 3,
            b: "k3".to_string(),
        };

        let mut map = DynamicMap::default();
        map.insert(key_a, 10u32);
        assert_eq!(10, *map.get(&key_b).unwrap().downcast_ref::<u32>().unwrap());
        assert!(map.get(&key_c).is_none());
        *map.get_mut(&key_b).unwrap().downcast_mut::<u32>().unwrap() = 20;
        assert_eq!(20, *map.get(&key_b).unwrap().downcast_ref::<u32>().unwrap());
    }

    #[test]
    fn reflect_unit_struct() {
        #[derive(Reflect)]
        struct Foo(u32, u64);

        let mut foo = Foo(1, 2);
        assert_eq!(1, *foo.get_field::<u32>(0).unwrap());
        assert_eq!(2, *foo.get_field::<u64>(1).unwrap());

        let mut patch = DynamicTupleStruct::default();
        patch.insert(3u32);
        patch.insert(4u64);
        assert_eq!(3, *patch.field(0).unwrap().downcast_ref::<u32>().unwrap());
        assert_eq!(4, *patch.field(1).unwrap().downcast_ref::<u64>().unwrap());

        foo.apply(&patch);
        assert_eq!(3, foo.0);
        assert_eq!(4, foo.1);

        let mut iter = patch.iter_fields();
        assert_eq!(3, *iter.next().unwrap().downcast_ref::<u32>().unwrap());
        assert_eq!(4, *iter.next().unwrap().downcast_ref::<u64>().unwrap());
    }

    #[test]
    #[should_panic(expected = "the given key does not support hashing")]
    fn reflect_map_no_hash() {
        #[derive(Reflect)]
        struct Foo {
            a: u32,
        }

        let foo = Foo { a: 1 };

        let mut map = DynamicMap::default();
        map.insert(foo, 10u32);
    }

    #[test]
    fn reflect_ignore() {
        #[derive(Reflect)]
        struct Foo {
            a: u32,
            #[reflect(ignore)]
            _b: u32,
        }

        let foo = Foo { a: 1, _b: 2 };

        let values: Vec<u32> = foo
            .iter_fields()
            .map(|value| *value.downcast_ref::<u32>().unwrap())
            .collect();
        assert_eq!(values, vec![1]);
    }

    #[test]
    fn reflect_complex_patch() {
        #[derive(Reflect, Eq, PartialEq, Debug)]
        struct Foo {
            a: u32,
            #[reflect(ignore)]
            _b: u32,
            c: Vec<isize>,
            d: HashMap<usize, i8>,
            e: Bar,
            f: (i32, Vec<isize>, Bar),
        }

        #[derive(Reflect, Eq, PartialEq, Debug)]
        struct Bar {
            x: u32,
        }

        let mut hash_map = HashMap::default();
        hash_map.insert(1, 1);
        hash_map.insert(2, 2);
        let mut foo = Foo {
            a: 1,
            _b: 1,
            c: vec![1, 2],
            d: hash_map,
            e: Bar { x: 1 },
            f: (1, vec![1, 2], Bar { x: 1 }),
        };

        let mut foo_patch = DynamicStruct::default();
        foo_patch.insert("a", 2u32);
        foo_patch.insert("b", 2u32); // this should be ignored

        let mut list = DynamicList::default();
        list.push(3isize);
        list.push(4isize);
        list.push(5isize);
        foo_patch.insert("c", list.clone_dynamic());

        let mut map = DynamicMap::default();
        map.insert(2usize, 3i8);
        foo_patch.insert("d", map);

        let mut bar_patch = DynamicStruct::default();
        bar_patch.insert("x", 2u32);
        foo_patch.insert("e", bar_patch.clone_dynamic());

        let mut tuple = DynamicTuple::default();
        tuple.insert(2i32);
        tuple.insert(list);
        tuple.insert(bar_patch);
        foo_patch.insert("f", tuple);

        foo.apply(&foo_patch);

        let mut hash_map = HashMap::default();
        hash_map.insert(1, 1);
        hash_map.insert(2, 3);
        let expected_foo = Foo {
            a: 2,
            _b: 1,
            c: vec![3, 4, 5],
            d: hash_map,
            e: Bar { x: 2 },
            f: (2, vec![3, 4, 5], Bar { x: 2 }),
        };

        assert_eq!(foo, expected_foo);
    }

    #[test]
    fn reflect_serialize() {
        #[derive(Reflect)]
        struct Foo {
            a: u32,
            #[reflect(ignore)]
            _b: u32,
            c: Vec<isize>,
            d: HashMap<usize, i8>,
            e: Bar,
            f: String,
            g: (i32, Vec<isize>, Bar),
        }

        #[derive(Reflect)]
        struct Bar {
            x: u32,
        }

        let mut hash_map = HashMap::default();
        hash_map.insert(1, 1);
        hash_map.insert(2, 2);
        let foo = Foo {
            a: 1,
            _b: 1,
            c: vec![1, 2],
            d: hash_map,
            e: Bar { x: 1 },
            f: "hi".to_string(),
            g: (1, vec![1, 2], Bar { x: 1 }),
        };

        let mut registry = TypeRegistry::default();
        registry.register::<u32>();
        registry.register::<isize>();
        registry.register::<usize>();
        registry.register::<Bar>();
        registry.register::<String>();
        registry.register::<i8>();
        registry.register::<i32>();

        let serializer = ReflectSerializer::new(&foo, &registry);
        let serialized = to_string_pretty(&serializer, PrettyConfig::default()).unwrap();

        let mut deserializer = Deserializer::from_str(&serialized).unwrap();
        let reflect_deserializer = ReflectDeserializer::new(&registry);
        let value = reflect_deserializer.deserialize(&mut deserializer).unwrap();
        let dynamic_struct = value.take::<DynamicStruct>().unwrap();

        assert!(foo.reflect_partial_eq(&dynamic_struct).unwrap());
    }

    #[test]
    fn reflect_take() {
        #[derive(Reflect, Debug, PartialEq)]
        struct Bar {
            x: u32,
        }

        let x: Box<dyn Reflect> = Box::new(Bar { x: 2 });
        let y = x.take::<Bar>().unwrap();
        assert_eq!(y, Bar { x: 2 });
    }
}
