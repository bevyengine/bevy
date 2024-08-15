//@check-pass
use bevy_reflect::{FromReflect, GetTypeRegistration, reflect_remote, Reflect, Typed};

mod external_crate {
    pub struct TheirOuter<T> {
        pub inner: TheirInner<T>,
    }
    pub struct TheirInner<T>(pub T);
}

#[reflect_remote(external_crate::TheirOuter<T>)]
struct MyOuter<T: FromReflect + Typed + GetTypeRegistration> {
    #[reflect(remote = MyInner<T>)]
    pub inner: external_crate::TheirInner<T>,
}

#[reflect_remote(external_crate::TheirInner<T>)]
struct MyInner<T: Reflect>(pub T);

fn main() {}
