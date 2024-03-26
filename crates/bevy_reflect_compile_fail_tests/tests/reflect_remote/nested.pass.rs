use bevy_reflect::{reflect_remote, Reflect, FromReflect, GetTypeRegistration};

mod external_crate {
    pub struct TheirOuter<T> {
        pub inner: TheirInner<T>,
    }
    pub struct TheirInner<T>(pub T);
}

#[reflect_remote(external_crate::TheirOuter<T>)]
struct MyOuter<T: FromReflect + GetTypeRegistration> {
    #[reflect(remote = "MyInner<T>")]
    pub inner: external_crate::TheirInner<T>,
}

#[reflect_remote(external_crate::TheirInner<T>)]
struct MyInner<T: Reflect>(pub T);

fn main() {}
