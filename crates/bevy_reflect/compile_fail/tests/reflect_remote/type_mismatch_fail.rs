//@no-rustfix

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
    //~^ ERROR: mismatched types
    //~| ERROR: mismatched types
    struct MyStruct {
        // Reason: Should use `MyFoo`
        #[reflect(remote = MyBar)]
        //~^ ERROR: mismatched types
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
    //~^ ERROR: mismatched types
    //~| ERROR: mismatched types
    struct MyStruct(
        // Reason: Should use `MyFoo`
        #[reflect(remote = MyBar)] external_crate::TheirFoo,
        //~^ ERROR: mismatched types
    );
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
    //~^ ERROR: `?` operator has incompatible types
    //~| ERROR: mismatched types
    enum MyBar {
        // Reason: Should use `i32`
        Value(u32),
        //~^ ERROR: mismatched types
    }

    #[derive(Reflect)]
    //~^ ERROR: mismatched types
    //~| ERROR: mismatched types
    //~| ERROR: mismatched types
    enum MyStruct {
        Value(
            // Reason: Should use `MyFoo`
            #[reflect(remote = MyBar)] external_crate::TheirFoo,
            //~^ ERROR: mismatched types
        ),
    }
}

fn main() {}
