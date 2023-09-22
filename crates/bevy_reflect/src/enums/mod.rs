mod dynamic_enum;
mod enum_trait;
mod helpers;
mod variants;

pub use dynamic_enum::*;
pub use enum_trait::*;
pub use helpers::*;
pub use variants::*;

#[cfg(test)]
mod tests {
    use crate as bevy_reflect;
    use crate::*;

    #[derive(Reflect, Debug, PartialEq)]
    enum MyEnum {
        A,
        B(usize, i32),
        C { foo: f32, bar: bool },
    }

    #[test]
    fn should_get_enum_type_info() {
        let info = MyEnum::type_info();
        if let TypeInfo::Enum(info) = info {
            assert!(info.is::<MyEnum>(), "expected type to be `MyEnum`");
            assert_eq!(std::any::type_name::<MyEnum>(), info.type_name());

            // === MyEnum::A === //
            assert_eq!("A", info.variant_at(0).unwrap().name());
            assert_eq!("A", info.variant("A").unwrap().name());
            if let VariantInfo::Unit(variant) = info.variant("A").unwrap() {
                assert_eq!("A", variant.name());
            } else {
                panic!("Expected `VariantInfo::Unit`");
            }

            // === MyEnum::B === //
            assert_eq!("B", info.variant_at(1).unwrap().name());
            assert_eq!("B", info.variant("B").unwrap().name());
            if let VariantInfo::Tuple(variant) = info.variant("B").unwrap() {
                assert!(variant.field_at(0).unwrap().is::<usize>());
                assert!(variant.field_at(1).unwrap().is::<i32>());
            } else {
                panic!("Expected `VariantInfo::Tuple`");
            }

            // === MyEnum::C === //
            assert_eq!("C", info.variant_at(2).unwrap().name());
            assert_eq!("C", info.variant("C").unwrap().name());
            if let VariantInfo::Struct(variant) = info.variant("C").unwrap() {
                assert!(variant.field_at(0).unwrap().is::<f32>());
                assert!(variant.field("foo").unwrap().is::<f32>());
            } else {
                panic!("Expected `VariantInfo::Struct`");
            }
        } else {
            panic!("Expected `TypeInfo::Enum`");
        }
    }

    #[test]
    fn dynamic_enum_should_set_variant_fields() {
        // === Unit === //
        let mut value = MyEnum::A;
        let dyn_enum = DynamicEnum::from(MyEnum::A);
        value.apply(&dyn_enum);
        assert_eq!(MyEnum::A, value);

        // === Tuple === //
        let mut value = MyEnum::B(0, 0);
        let dyn_enum = DynamicEnum::from(MyEnum::B(123, 321));
        value.apply(&dyn_enum);
        assert_eq!(MyEnum::B(123, 321), value);

        // === Struct === //
        let mut value = MyEnum::C {
            foo: 0.0,
            bar: false,
        };
        let dyn_enum = DynamicEnum::from(MyEnum::C {
            foo: 1.23,
            bar: true,
        });
        value.apply(&dyn_enum);
        assert_eq!(
            MyEnum::C {
                foo: 1.23,
                bar: true,
            },
            value
        );
    }

    #[test]
    fn partial_dynamic_enum_should_set_variant_fields() {
        // === Tuple === //
        let mut value = MyEnum::B(0, 0);

        let mut data = DynamicTuple::default();
        data.insert(123usize);

        let mut dyn_enum = DynamicEnum::default();
        dyn_enum.set_variant("B", data);
        value.apply(&dyn_enum);
        assert_eq!(MyEnum::B(123, 0), value);

        // === Struct === //
        let mut value = MyEnum::C {
            foo: 1.23,
            bar: false,
        };

        let mut data = DynamicStruct::default();
        data.insert("bar", true);

        let mut dyn_enum = DynamicEnum::default();
        dyn_enum.set_variant("C", data);
        value.apply(&dyn_enum);
        assert_eq!(
            MyEnum::C {
                foo: 1.23,
                bar: true,
            },
            value
        );
    }

    #[test]
    fn dynamic_enum_should_apply_dynamic_enum() {
        let mut a = DynamicEnum::from(MyEnum::B(123, 321));
        let b = DynamicEnum::from(MyEnum::B(123, 321));

        // Sanity check that equality check works
        assert!(
            a.reflect_partial_eq(&b).unwrap_or_default(),
            "dynamic enums should be equal"
        );

        a.set_variant("A", ());
        assert!(
            !a.reflect_partial_eq(&b).unwrap_or_default(),
            "dynamic enums should not be equal"
        );

        a.apply(&b);
        assert!(a.reflect_partial_eq(&b).unwrap_or_default());
    }

    #[test]
    fn dynamic_enum_should_change_variant() {
        let mut value = MyEnum::A;

        // === MyEnum::A -> MyEnum::B === //
        let mut dyn_enum = DynamicEnum::from(MyEnum::B(123, 321));
        value.apply(&dyn_enum);
        assert_eq!(MyEnum::B(123, 321), value);

        // === MyEnum::B -> MyEnum::C === //
        let mut data = DynamicStruct::default();
        data.insert("foo", 1.23_f32);
        data.insert("bar", true);
        dyn_enum.set_variant("C", data);
        value.apply(&dyn_enum);
        assert_eq!(
            MyEnum::C {
                foo: 1.23,
                bar: true
            },
            value
        );

        // === MyEnum::C -> MyEnum::B === //
        let mut data = DynamicTuple::default();
        data.insert(123_usize);
        data.insert(321_i32);
        dyn_enum.set_variant("B", data);
        value.apply(&dyn_enum);
        assert_eq!(MyEnum::B(123, 321), value);

        // === MyEnum::B -> MyEnum::A === //
        dyn_enum.set_variant("A", ());
        value.apply(&dyn_enum);
        assert_eq!(MyEnum::A, value);
    }

    #[test]
    fn enum_should_iterate_fields() {
        // === Unit === //
        let value: &dyn Enum = &MyEnum::A;
        assert_eq!(0, value.field_len());
        let mut iter = value.iter_fields();
        assert!(iter.next().is_none());

        // === Tuple === //
        let value: &dyn Enum = &MyEnum::B(123, 321);
        assert_eq!(2, value.field_len());
        let mut iter = value.iter_fields();
        assert!(iter
            .next()
            .and_then(|field| field.value().reflect_partial_eq(&123_usize))
            .unwrap_or_default());
        assert!(iter
            .next()
            .and_then(|field| field.value().reflect_partial_eq(&321_i32))
            .unwrap_or_default());

        // === Struct === //
        let value: &dyn Enum = &MyEnum::C {
            foo: 1.23,
            bar: true,
        };
        assert_eq!(2, value.field_len());
        let mut iter = value.iter_fields();
        assert!(iter
            .next()
            .and_then(|field| field
                .value()
                .reflect_partial_eq(&1.23_f32)
                .and(field.name().map(|name| name == "foo")))
            .unwrap_or_default());
        assert!(iter
            .next()
            .and_then(|field| field
                .value()
                .reflect_partial_eq(&true)
                .and(field.name().map(|name| name == "bar")))
            .unwrap_or_default());
    }

    #[test]
    fn enum_should_return_correct_variant_type() {
        // === Unit === //
        let value = MyEnum::A;
        assert_eq!(VariantType::Unit, value.variant_type());

        // === Tuple === //
        let value = MyEnum::B(0, 0);
        assert_eq!(VariantType::Tuple, value.variant_type());

        // === Struct === //
        let value = MyEnum::C {
            foo: 1.23,
            bar: true,
        };
        assert_eq!(VariantType::Struct, value.variant_type());
    }

    #[test]
    fn enum_should_return_correct_variant_path() {
        // === Unit === //
        let value = MyEnum::A;
        assert_eq!(
            "bevy_reflect::enums::tests::MyEnum::A",
            value.variant_path()
        );

        // === Tuple === //
        let value = MyEnum::B(0, 0);
        assert_eq!(
            "bevy_reflect::enums::tests::MyEnum::B",
            value.variant_path()
        );

        // === Struct === //
        let value = MyEnum::C {
            foo: 1.23,
            bar: true,
        };
        assert_eq!(
            "bevy_reflect::enums::tests::MyEnum::C",
            value.variant_path()
        );
    }

    #[test]
    #[should_panic(expected = "`((usize, i32))` is not an enum")]
    fn applying_non_enum_should_panic() {
        let mut value = MyEnum::B(0, 0);
        let mut dyn_tuple = DynamicTuple::default();
        dyn_tuple.insert((123_usize, 321_i32));
        value.apply(&dyn_tuple);
    }

    #[test]
    fn should_skip_ignored_fields() {
        #[derive(Reflect, Debug, PartialEq)]
        enum TestEnum {
            A,
            B,
            C {
                #[reflect(ignore)]
                foo: f32,
                bar: bool,
            },
        }

        if let TypeInfo::Enum(info) = TestEnum::type_info() {
            assert_eq!(3, info.variant_len());
            if let VariantInfo::Struct(variant) = info.variant("C").unwrap() {
                assert_eq!(
                    1,
                    variant.field_len(),
                    "expected one of the fields to be ignored"
                );
                assert!(variant.field_at(0).unwrap().is::<bool>());
            } else {
                panic!("expected `VariantInfo::Struct`");
            }
        } else {
            panic!("expected `TypeInfo::Enum`");
        }
    }

    #[test]
    fn enum_should_allow_generics() {
        #[derive(Reflect, Debug, PartialEq)]
        enum TestEnum<T: FromReflect> {
            A,
            B(T),
            C { value: T },
        }

        if let TypeInfo::Enum(info) = TestEnum::<f32>::type_info() {
            if let VariantInfo::Tuple(variant) = info.variant("B").unwrap() {
                assert!(variant.field_at(0).unwrap().is::<f32>());
            } else {
                panic!("expected `VariantInfo::Struct`");
            }
            if let VariantInfo::Struct(variant) = info.variant("C").unwrap() {
                assert!(variant.field("value").unwrap().is::<f32>());
            } else {
                panic!("expected `VariantInfo::Struct`");
            }
        } else {
            panic!("expected `TypeInfo::Enum`");
        }

        let mut value = TestEnum::<f32>::A;

        // === Tuple === //
        let mut data = DynamicTuple::default();
        data.insert(1.23_f32);
        let dyn_enum = DynamicEnum::new("B", data);
        value.apply(&dyn_enum);
        assert_eq!(TestEnum::B(1.23), value);

        // === Struct === //
        let mut data = DynamicStruct::default();
        data.insert("value", 1.23_f32);
        let dyn_enum = DynamicEnum::new("C", data);
        value.apply(&dyn_enum);
        assert_eq!(TestEnum::C { value: 1.23 }, value);
    }

    #[test]
    fn enum_should_allow_struct_fields() {
        #[derive(Reflect, Debug, PartialEq)]
        enum TestEnum {
            A,
            B(TestStruct),
            C { value: TestStruct },
        }

        #[derive(Reflect, Debug, PartialEq)]
        struct TestStruct(usize);

        let mut value = TestEnum::A;

        // === Tuple === //
        let mut data = DynamicTuple::default();
        data.insert(TestStruct(123));
        let dyn_enum = DynamicEnum::new("B", data);
        value.apply(&dyn_enum);
        assert_eq!(TestEnum::B(TestStruct(123)), value);

        // === Struct === //
        let mut data = DynamicStruct::default();
        data.insert("value", TestStruct(123));
        let dyn_enum = DynamicEnum::new("C", data);
        value.apply(&dyn_enum);
        assert_eq!(
            TestEnum::C {
                value: TestStruct(123)
            },
            value
        );
    }

    #[test]
    fn enum_should_allow_nesting_enums() {
        #[derive(Reflect, Debug, PartialEq)]
        enum TestEnum {
            A,
            B(OtherEnum),
            C { value: OtherEnum },
        }

        #[derive(Reflect, Debug, PartialEq)]
        enum OtherEnum {
            A,
            B(usize),
            C { value: f32 },
        }

        let mut value = TestEnum::A;

        // === Tuple === //
        let mut data = DynamicTuple::default();
        data.insert(OtherEnum::B(123));
        let dyn_enum = DynamicEnum::new("B", data);
        value.apply(&dyn_enum);
        assert_eq!(TestEnum::B(OtherEnum::B(123)), value);

        // === Struct === //
        let mut data = DynamicStruct::default();
        data.insert("value", OtherEnum::C { value: 1.23 });
        let dyn_enum = DynamicEnum::new("C", data);
        value.apply(&dyn_enum);
        assert_eq!(
            TestEnum::C {
                value: OtherEnum::C { value: 1.23 }
            },
            value
        );
    }

    #[test]
    fn enum_should_apply() {
        let mut value: Box<dyn Reflect> = Box::new(MyEnum::A);

        // === MyEnum::A -> MyEnum::A === //
        value.apply(&MyEnum::A);
        assert!(value.reflect_partial_eq(&MyEnum::A).unwrap_or_default());

        // === MyEnum::A -> MyEnum::B === //
        value.apply(&MyEnum::B(123, 321));
        assert!(value
            .reflect_partial_eq(&MyEnum::B(123, 321))
            .unwrap_or_default());

        // === MyEnum::B -> MyEnum::B === //
        value.apply(&MyEnum::B(321, 123));
        assert!(value
            .reflect_partial_eq(&MyEnum::B(321, 123))
            .unwrap_or_default());

        // === MyEnum::B -> MyEnum::C === //
        value.apply(&MyEnum::C {
            foo: 1.23,
            bar: true,
        });
        assert!(value
            .reflect_partial_eq(&MyEnum::C {
                foo: 1.23,
                bar: true
            })
            .unwrap_or_default());

        // === MyEnum::C -> MyEnum::C === //
        value.apply(&MyEnum::C {
            foo: 3.21,
            bar: false,
        });
        assert!(value
            .reflect_partial_eq(&MyEnum::C {
                foo: 3.21,
                bar: false
            })
            .unwrap_or_default());

        // === MyEnum::C -> MyEnum::B === //
        value.apply(&MyEnum::B(123, 321));
        assert!(value
            .reflect_partial_eq(&MyEnum::B(123, 321))
            .unwrap_or_default());

        // === MyEnum::B -> MyEnum::A === //
        value.apply(&MyEnum::A);
        assert!(value.reflect_partial_eq(&MyEnum::A).unwrap_or_default());
    }

    #[test]
    fn enum_should_set() {
        let mut value: Box<dyn Reflect> = Box::new(MyEnum::A);

        // === MyEnum::A -> MyEnum::A === //
        value.set(Box::new(MyEnum::A)).unwrap();
        assert!(value.reflect_partial_eq(&MyEnum::A).unwrap_or_default());

        // === MyEnum::A -> MyEnum::B === //
        value.set(Box::new(MyEnum::B(123, 321))).unwrap();
        assert!(value
            .reflect_partial_eq(&MyEnum::B(123, 321))
            .unwrap_or_default());

        // === MyEnum::B -> MyEnum::B === //
        value.set(Box::new(MyEnum::B(321, 123))).unwrap();
        assert!(value
            .reflect_partial_eq(&MyEnum::B(321, 123))
            .unwrap_or_default());

        // === MyEnum::B -> MyEnum::C === //
        value
            .set(Box::new(MyEnum::C {
                foo: 1.23,
                bar: true,
            }))
            .unwrap();
        assert!(value
            .reflect_partial_eq(&MyEnum::C {
                foo: 1.23,
                bar: true
            })
            .unwrap_or_default());

        // === MyEnum::C -> MyEnum::C === //
        value
            .set(Box::new(MyEnum::C {
                foo: 3.21,
                bar: false,
            }))
            .unwrap();
        assert!(value
            .reflect_partial_eq(&MyEnum::C {
                foo: 3.21,
                bar: false
            })
            .unwrap_or_default());

        // === MyEnum::C -> MyEnum::B === //
        value.set(Box::new(MyEnum::B(123, 321))).unwrap();
        assert!(value
            .reflect_partial_eq(&MyEnum::B(123, 321))
            .unwrap_or_default());

        // === MyEnum::B -> MyEnum::A === //
        value.set(Box::new(MyEnum::A)).unwrap();
        assert!(value.reflect_partial_eq(&MyEnum::A).unwrap_or_default());
    }

    #[test]
    fn enum_should_partial_eq() {
        #[derive(Reflect)]
        enum TestEnum {
            A,
            A1,
            B(usize),
            B1(usize),
            B2(usize, usize),
            C { value: i32 },
            C1 { value: i32 },
            C2 { value: f32 },
        }

        let a: &dyn Reflect = &TestEnum::A;
        let b: &dyn Reflect = &TestEnum::A;
        assert!(
            a.reflect_partial_eq(b).unwrap_or_default(),
            "expected TestEnum::A == TestEnum::A"
        );

        let a: &dyn Reflect = &TestEnum::A;
        let b: &dyn Reflect = &TestEnum::A1;
        assert!(
            !a.reflect_partial_eq(b).unwrap_or_default(),
            "expected TestEnum::A != TestEnum::A1"
        );

        let a: &dyn Reflect = &TestEnum::B(123);
        let b: &dyn Reflect = &TestEnum::B(123);
        assert!(
            a.reflect_partial_eq(b).unwrap_or_default(),
            "expected TestEnum::B(123) == TestEnum::B(123)"
        );

        let a: &dyn Reflect = &TestEnum::B(123);
        let b: &dyn Reflect = &TestEnum::B(321);
        assert!(
            !a.reflect_partial_eq(b).unwrap_or_default(),
            "expected TestEnum::B(123) != TestEnum::B(321)"
        );

        let a: &dyn Reflect = &TestEnum::B(123);
        let b: &dyn Reflect = &TestEnum::B1(123);
        assert!(
            !a.reflect_partial_eq(b).unwrap_or_default(),
            "expected TestEnum::B(123) != TestEnum::B1(123)"
        );

        let a: &dyn Reflect = &TestEnum::B(123);
        let b: &dyn Reflect = &TestEnum::B2(123, 123);
        assert!(
            !a.reflect_partial_eq(b).unwrap_or_default(),
            "expected TestEnum::B(123) != TestEnum::B2(123, 123)"
        );

        let a: &dyn Reflect = &TestEnum::C { value: 123 };
        let b: &dyn Reflect = &TestEnum::C { value: 123 };
        assert!(
            a.reflect_partial_eq(b).unwrap_or_default(),
            "expected TestEnum::C{{value: 123}} == TestEnum::C{{value: 123}}"
        );

        let a: &dyn Reflect = &TestEnum::C { value: 123 };
        let b: &dyn Reflect = &TestEnum::C { value: 321 };
        assert!(
            !a.reflect_partial_eq(b).unwrap_or_default(),
            "expected TestEnum::C{{value: 123}} != TestEnum::C{{value: 321}}"
        );

        let a: &dyn Reflect = &TestEnum::C { value: 123 };
        let b: &dyn Reflect = &TestEnum::C1 { value: 123 };
        assert!(
            !a.reflect_partial_eq(b).unwrap_or_default(),
            "expected TestEnum::C{{value: 123}} != TestEnum::C1{{value: 123}}"
        );

        let a: &dyn Reflect = &TestEnum::C { value: 123 };
        let b: &dyn Reflect = &TestEnum::C2 { value: 1.23 };
        assert!(
            !a.reflect_partial_eq(b).unwrap_or_default(),
            "expected TestEnum::C{{value: 123}} != TestEnum::C2{{value: 1.23}}"
        );
    }
}
