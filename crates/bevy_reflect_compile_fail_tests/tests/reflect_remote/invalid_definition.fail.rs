mod structs {
    use bevy_reflect::reflect_remote;

    mod external_crate {
        pub struct TheirStruct {
            pub value: u32,
        }
    }

    #[reflect_remote(external_crate::TheirStruct)]
    struct MyStruct {
        // Reason: Should be `u32`
        pub value: bool,
    }
}

mod tuple_structs {
    use bevy_reflect::reflect_remote;

    mod external_crate {
        pub struct TheirStruct(pub u32);
    }

    #[reflect_remote(external_crate::TheirStruct)]
    struct MyStruct(
        // Reason: Should be `u32`
        pub bool,
    );
}

mod enums {
    use bevy_reflect::reflect_remote;

    mod external_crate {
        pub enum TheirStruct {
            Unit,
            Tuple(u32),
            Struct { value: usize },
        }
    }

    #[reflect_remote(external_crate::TheirStruct, FromReflect)]
    enum MyStruct {
        // Reason: Should be unit variant
        Unit(i32),
        // Reason: Should be `u32`
        Tuple(bool),
        // Reason: Should be `usize`
        Struct { value: String },
    }
}

fn main() {}
