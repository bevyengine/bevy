#![expect(
    unused_qualifications,
    reason = "the macro uses `MyEnum::Variant` which is generally unnecessary for `Option`"
)]

use bevy_reflect_derive::impl_reflect;

impl_reflect! {
    #[type_path = "core::option"]
    enum Option<T> {
        None,
        Some(T),
    }
}
