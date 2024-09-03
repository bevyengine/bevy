//@check-pass
use bevy_reflect::{reflect_remote, std_traits::ReflectDefault};

mod external_crate {
    #[derive(Debug, Default)]
    pub struct TheirType {
        pub value: String,
    }
}

#[reflect_remote(external_crate::TheirType)]
#[derive(Debug, Default)]
#[reflect(Debug, Default)]
struct MyType {
    pub value: String,
}

fn main() {}
