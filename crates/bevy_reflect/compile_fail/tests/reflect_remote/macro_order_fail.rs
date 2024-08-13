use bevy_reflect::{reflect_remote, std_traits::ReflectDefault};

mod external_crate {
    #[derive(Debug, Default)]
    pub struct TheirType {
        pub value: String,
    }
}

#[derive(Debug, Default)]
#[reflect_remote(external_crate::TheirType)]
#[reflect(Debug, Default)]
struct MyType {
    pub value: String,
    //~^ ERROR: no field `value` on type `&MyType`
    //~| ERROR: struct `MyType` has no field named `value`
}

fn main() {}
