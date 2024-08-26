//@check-pass

mod structs {
    use bevy_reflect::{reflect_remote, Reflect};

    mod external_crate {
        pub struct TheirFoo {
            pub value: u32,
        }
        pub struct TheirBar {
            pub value: i32,
        }
    }

    #[reflect_remote(external_crate::TheirFoo)]
    struct MyFoo {
        pub value: u32,
    }
    #[reflect_remote(external_crate::TheirBar)]
    struct MyBar {
        pub value: i32,
    }

    #[derive(Reflect)]
    struct MyStruct {
        #[reflect(remote = MyFoo)]
        foo: external_crate::TheirFoo,
    }
}

mod tuple_structs {
    use bevy_reflect::{reflect_remote, Reflect};

    mod external_crate {
        pub struct TheirFoo(pub u32);

        pub struct TheirBar(pub i32);
    }

    #[reflect_remote(external_crate::TheirFoo)]
    struct MyFoo(pub u32);

    #[reflect_remote(external_crate::TheirBar)]
    struct MyBar(pub i32);

    #[derive(Reflect)]
    struct MyStruct(#[reflect(remote = MyFoo)] external_crate::TheirFoo);
}

mod enums {
    use bevy_reflect::{reflect_remote, Reflect};

    mod external_crate {
        pub enum TheirFoo {
            Value(u32),
        }

        pub enum TheirBar {
            Value(i32),
        }
    }

    #[reflect_remote(external_crate::TheirFoo)]
    enum MyFoo {
        Value(u32),
    }

    #[reflect_remote(external_crate::TheirBar)]
    enum MyBar {
        Value(i32),
    }

    #[derive(Reflect)]
    enum MyStruct {
        Value(#[reflect(remote = MyFoo)] external_crate::TheirFoo),
    }
}

fn main() {}
