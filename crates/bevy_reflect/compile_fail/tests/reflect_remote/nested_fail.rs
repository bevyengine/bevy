mod external_crate {
    pub struct TheirOuter<T> {
        pub inner: TheirInner<T>,
    }
    pub struct TheirInner<T>(pub T);
}

mod missing_attribute {
    use bevy_reflect::{reflect_remote, Reflect};

    #[reflect_remote(super::external_crate::TheirOuter<T>)]
    struct MyOuter<T: Reflect> {
        // Reason: Missing `#[reflect(remote = "...")]` attribute
        pub inner: super::external_crate::TheirInner<T>,
    }

    #[reflect_remote(super::external_crate::TheirInner<T>)]
    struct MyInner<T: Reflect>(pub T);
}

mod incorrect_inner_type {
    use bevy_reflect::{reflect_remote, Reflect};

    #[reflect_remote(super::external_crate::TheirOuter<T>)]
    //~^ ERROR: `TheirInner<T>` can not be reflected
    //~| ERROR: `TheirInner<T>` can not be reflected
    //~| ERROR: `TheirInner<T>` can not be reflected
    //~| ERROR: `TheirInner<T>` can not be used as a dynamic type path
    //~| ERROR: `?` operator has incompatible types
    struct MyOuter<T: Reflect> {
        // Reason: Should not use `MyInner<T>` directly
        pub inner: MyInner<T>,
    }

    #[reflect_remote(super::external_crate::TheirInner<T>)]
    struct MyInner<T: Reflect>(pub T);
}

fn main() {}
