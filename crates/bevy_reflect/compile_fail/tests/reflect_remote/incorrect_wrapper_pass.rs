//@check-pass
use bevy_reflect::{reflect_remote, Reflect};

mod external_crate {
    pub struct TheirFoo {
        pub value: u32,
    }
}

#[reflect_remote(external_crate::TheirFoo)]
struct MyFoo {
    pub value: u32,
}

#[derive(Reflect)]
struct MyStruct {
    #[reflect(remote = MyFoo)]
    foo: external_crate::TheirFoo,
}

fn main() {}
