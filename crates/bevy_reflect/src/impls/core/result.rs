// TODO: Depending on the result of https://github.com/rust-lang/rust/issues/165835, this may be
// either a bug or intentional. `expect` or remove based on the resolution.
#![allow(
    unused_qualifications,
    reason = "the macro uses `MyEnum::Variant` which is generally unnecessary for `Result`"
)]

use bevy_reflect_derive::impl_reflect;

impl_reflect! {
    #[type_path = "core::result"]
    enum Result<T, E> {
        Ok(T),
        Err(E),
    }
}
