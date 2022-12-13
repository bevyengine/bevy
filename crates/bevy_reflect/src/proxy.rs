use crate::{Reflect, TypeInfo};

/// This trait allows for types to behave as proxies to another type.
///
/// This is mainly used by [dynamic] types such as [`DynamicStruct`],
/// [`DynamicList`], etc.
///
/// [dynamic]: crate::TypeInfo::Dynamic
/// [`DynamicStruct`]: crate::DynamicStruct
/// [`DynamicList`]: crate::DynamicList
pub trait Proxy: Reflect {
    /// Returns the [`TypeInfo`] of the type this proxy represents, if any.
    ///
    /// This will return `None` if the proxy currently does not represent any concrete type.
    fn represents(&self) -> Option<&'static TypeInfo>;
}

#[cfg(test)]
mod tests {
    use super::Proxy;
    use crate as bevy_reflect;
    use crate::{
        Array, DynamicArray, DynamicEnum, DynamicList, DynamicMap, DynamicStruct, DynamicTuple,
        DynamicTupleStruct, Enum, List, Map, Reflect, Struct, Tuple, TupleStruct, TypeInfo, Typed,
    };
    use bevy_utils::HashMap;

    macro_rules! assert_proxy {
        ($expected_type_info: ident, $proxy: ident) => {
            assert_eq!(
                $expected_type_info.type_name(),
                $proxy.represents().unwrap().type_name()
            );

            let TypeInfo::Dynamic(info) = $proxy.get_type_info() else { panic!("expected `TypeInfo::Dynamic`") };
            assert_eq!(
                $expected_type_info.type_name(),
                info.represents(&$proxy).unwrap().type_name()
            )
        };
    }

    #[test]
    fn dynamic_tuple_should_represent_type() {
        let expected = <(f32, f32) as Typed>::type_info();
        let mut dyn_tuple = DynamicTuple::default();
        dyn_tuple.insert(1.23_f32);
        dyn_tuple.insert(3.21_f32);

        // Inserting to a DynamicTuple clears the represented type
        assert!(dyn_tuple.represents().is_none());

        dyn_tuple.set_represented_type(Some(expected));
        assert_proxy!(expected, dyn_tuple);

        let dyn_tuple = (1.23_f32, 3.21_f32).clone_dynamic();
        assert_proxy!(expected, dyn_tuple);
    }

    #[test]
    fn dynamic_array_should_represent_type() {
        let expected = <[f32; 2] as Typed>::type_info();
        let mut dyn_array = DynamicArray::from_vec(vec![1.23_f32, 3.21_f32]);

        // DynamicArrays initialize without a represented type
        assert!(dyn_array.represents().is_none());

        dyn_array.set_represented_type(Some(expected));
        assert_proxy!(expected, dyn_array);

        let dyn_array = [1.23_f32, 3.21_f32].clone_dynamic();
        assert_proxy!(expected, dyn_array);
    }

    #[test]
    fn dynamic_list_should_represent_type() {
        let expected = <Vec<f32> as Typed>::type_info();
        let mut dyn_list = DynamicList::default();
        dyn_list.push(1.23_f32);
        dyn_list.push(3.21_f32);

        // DynamicLists initialize without a represented type
        assert!(dyn_list.represents().is_none());

        dyn_list.set_represented_type(Some(expected));
        assert_proxy!(expected, dyn_list);

        let dyn_list = List::clone_dynamic(&vec![1.23_f32, 3.21_f32]);
        assert_proxy!(expected, dyn_list);
    }

    #[test]
    fn dynamic_map_should_represent_type() {
        let expected = <HashMap<usize, f32> as Typed>::type_info();
        let mut dyn_map = DynamicMap::default();
        dyn_map.insert(0_usize, 1.23_f32);
        dyn_map.insert(1_usize, 3.21_f32);

        // DynamicMaps initialize without a represented type
        assert!(dyn_map.represents().is_none());

        dyn_map.set_represented_type(Some(expected));
        assert_proxy!(expected, dyn_map);

        let mut map = HashMap::<usize, f32>::new();
        map.insert(0_usize, 1.23_f32);
        map.insert(1_usize, 3.21_f32);
        let dyn_map = map.clone_dynamic();
        assert_proxy!(expected, dyn_map);
    }

    #[test]
    fn dynamic_tuple_struct_should_represent_type() {
        #[derive(Reflect)]
        struct MyTupleStruct(f32, f32);

        let expected = <MyTupleStruct as Typed>::type_info();
        let mut dyn_tuple_struct = DynamicTupleStruct::default();
        dyn_tuple_struct.insert(1.23_f32);
        dyn_tuple_struct.insert(3.21_f32);

        // DynamicTupleStructs initialize without a represented type
        assert!(dyn_tuple_struct.represents().is_none());

        dyn_tuple_struct.set_represented_type(Some(expected));
        assert_proxy!(expected, dyn_tuple_struct);

        let dyn_tuple_struct = MyTupleStruct(1.23_f32, 3.21_f32).clone_dynamic();
        assert_proxy!(expected, dyn_tuple_struct);
    }

    #[test]
    fn dynamic_struct_should_represent_type() {
        #[derive(Reflect)]
        struct MyStruct {
            foo: f32,
            bar: f32,
        }

        let expected = <MyStruct as Typed>::type_info();
        let mut dyn_struct = DynamicStruct::default();
        dyn_struct.insert("foo", 1.23_f32);
        dyn_struct.insert("bar", 3.21_f32);

        // DynamicStructs initialize without a represented type
        assert!(dyn_struct.represents().is_none());

        dyn_struct.set_represented_type(Some(expected));
        assert_proxy!(expected, dyn_struct);

        let dyn_struct = MyStruct {
            foo: 1.23_f32,
            bar: 3.21_f32,
        }
        .clone_dynamic();
        assert_proxy!(expected, dyn_struct);
    }

    #[test]
    fn dynamic_enum_should_represent_type() {
        #[derive(Reflect)]
        enum MyEnum {
            Unit,
            Tuple(f32, f32),
            Struct { foo: f32, bar: f32 },
        }

        let expected = <MyEnum as Typed>::type_info();
        let mut dyn_enum = DynamicEnum::default();
        dyn_enum.set_variant("Unit", ());

        // DynamicEnums initialize without a represented type
        assert!(dyn_enum.represents().is_none());

        dyn_enum.set_represented_type(Some(expected));
        assert_proxy!(expected, dyn_enum);

        let dyn_unit_enum = MyEnum::Unit.clone_dynamic();
        assert_proxy!(expected, dyn_unit_enum);

        let dyn_tuple_enum = MyEnum::Tuple(1.23, 3.21).clone_dynamic();
        assert_proxy!(expected, dyn_tuple_enum);

        let dyn_struct_enum = MyEnum::Struct {
            foo: 1.23_f32,
            bar: 3.21_f32,
        }
        .clone_dynamic();
        assert_proxy!(expected, dyn_struct_enum);
    }
}
