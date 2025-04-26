use bevy_reflect_derive::impl_type_path;

use crate::impls::macros::impl_reflect_for_veclike;
#[cfg(feature = "functions")]
use crate::{
    from_reflect::FromReflect, type_info::MaybeTyped, type_path::TypePath,
    type_registry::GetTypeRegistration,
};

impl_reflect_for_veclike!(
    ::alloc::collections::VecDeque<T>,
    ::alloc::collections::VecDeque::insert,
    ::alloc::collections::VecDeque::remove,
    ::alloc::collections::VecDeque::push_back,
    ::alloc::collections::VecDeque::pop_back,
    ::alloc::collections::VecDeque::<T>
);
impl_type_path!(::alloc::collections::VecDeque<T>);
#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!(::alloc::collections::VecDeque<T>; <T: FromReflect + MaybeTyped + TypePath + GetTypeRegistration>);
