mod external_crate {
    pub struct TheirOuter<T> {
        pub inner: TheirInner<T>,
    }
    pub struct TheirInner<T>(pub T);
}

mod missing_attribute {
    use bevy_reflect::{FromReflect, GetTypeRegistration, reflect_remote};

    #[reflect_remote(super::external_crate::TheirOuter<T>)]
    struct MyOuter<T: FromReflect + GetTypeRegistration> {
        // Reason: Missing `#[reflect(remote = ...)]` attribute
        pub inner: super::external_crate::TheirInner<T>,
    }

    #[reflect_remote(super::external_crate::TheirInner<T>)]
    struct MyInner<T>(pub T);
}

mod incorrect_inner_type {
    use bevy_reflect::{FromReflect, GetTypeRegistration, reflect_remote};

    #[reflect_remote(super::external_crate::TheirOuter<T>)]
    //~^ ERROR: `TheirInner<T>` does not implement `PartialReflect` so cannot be introspected
    //~| ERROR: `TheirInner<T>` does not implement `PartialReflect` so cannot be introspected
    //~| ERROR: `TheirInner<T>` does not implement `PartialReflect` so cannot be introspected
    //~| ERROR: `TheirInner<T>` does not implement `TypePath` so cannot provide dynamic type path information
    //~| ERROR: `?` operator has incompatible types
    //~| ERROR: mismatched types
    struct MyOuter<T: FromReflect + GetTypeRegistration> {
        // Reason: Should not use `MyInner<T>` directly
        pub inner: MyInner<T>,
        //~^ ERROR: mismatched types
    }

    #[reflect_remote(super::external_crate::TheirInner<T>)]
    struct MyInner<T>(pub T);
}

mod mismatched_remote_type {
    use bevy_reflect::{FromReflect, GetTypeRegistration, reflect_remote};

    #[reflect_remote(super::external_crate::TheirOuter<T>)]
    //~^ ERROR: mismatched types
    //~| ERROR: mismatched types
    struct MyOuter<T: FromReflect + GetTypeRegistration> {
        // Reason: Should be `MyInner<T>`
        #[reflect(remote = MyOuter<T>)]
        //~^ ERROR: mismatched types
        pub inner: super::external_crate::TheirInner<T>,
    }

    #[reflect_remote(super::external_crate::TheirInner<T>)]
    struct MyInner<T>(pub T);
}

mod mismatched_remote_generic {
    use bevy_reflect::{FromReflect, GetTypeRegistration, reflect_remote};

    #[reflect_remote(super::external_crate::TheirOuter<T>)]
    //~^ ERROR: `?` operator has incompatible types
    //~| ERROR: mismatched types
    //~| ERROR: mismatched types
    struct MyOuter<T: FromReflect + GetTypeRegistration> {
        // Reason: `TheirOuter::inner` is not defined as `TheirInner<bool>`
        #[reflect(remote = MyInner<bool>)]
        pub inner: super::external_crate::TheirInner<bool>,
        //~^ ERROR: mismatched types
    }

    #[reflect_remote(super::external_crate::TheirInner<T>)]
    struct MyInner<T>(pub T);
}

fn main() {}
