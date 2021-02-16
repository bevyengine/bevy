// Copyright 2019 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// modified by Bevy contributors

use crate::{Component, TypeInfo};
use std::{
    any::{type_name, TypeId},
    fmt, mem,
    ptr::NonNull,
};

/// A dynamically typed collection of components
///
/// See [Bundle]
pub trait DynamicBundle {
    /// Invoke a callback on the fields' type IDs, sorted by descending alignment then id
    #[doc(hidden)]
    fn with_ids<T>(&self, f: impl FnOnce(&[TypeId]) -> T) -> T;
    /// Obtain the fields' TypeInfos, sorted by descending alignment then id
    #[doc(hidden)]
    fn type_info(&self) -> &[TypeInfo];
    /// Allow a callback to move all components out of the bundle
    ///
    /// Must invoke `f` only with a valid pointer, its type, and the pointee's size. A `false`
    /// return value indicates that the value was not moved and should be dropped.
    #[doc(hidden)]
    unsafe fn put(self, f: impl FnMut(*mut u8, TypeId, usize) -> bool);
}

/// A statically typed collection of components
///
/// See [DynamicBundle]
pub trait Bundle: DynamicBundle {
    #[doc(hidden)]
    const TYPES: &'static [TypeInfo];

    #[doc(hidden)]
    const IDS: &'static [TypeId];

    /// Construct `Self` by moving components out of pointers fetched by `f`
    ///
    /// # Safety
    ///
    /// `f` must produce pointers to the expected fields. The implementation must not read from any
    /// pointers if any call to `f` returns `None`.
    #[doc(hidden)]
    unsafe fn get(
        f: impl FnMut(TypeId, usize) -> Option<NonNull<u8>>,
    ) -> Result<Self, MissingComponent>
    where
        Self: Sized;
}

/// Error indicating that an entity did not have a required component
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct MissingComponent(&'static str);

impl MissingComponent {
    /// Construct an error representing a missing `T`
    pub fn new<T: Component>() -> Self {
        Self(type_name::<T>())
    }
}

impl fmt::Display for MissingComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "missing {} component", self.0)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MissingComponent {}

macro_rules! tuple_impl {
    ($($name: ident),*) => {
        impl<$($name: Component),*> DynamicBundle for ($($name,)*) {
            fn with_ids<T>(&self, f: impl FnOnce(&[TypeId]) -> T) -> T {
                f(<Self as Bundle>::IDS)
            }

            fn type_info(&self) -> &[TypeInfo] {
                <Self as Bundle>::TYPES
            }

            #[allow(unused_variables, unused_mut)]
            unsafe fn put(self, mut f: impl FnMut(*mut u8, TypeId, usize) -> bool) {
                #[allow(non_snake_case)]
                let ($(mut $name,)*) = self;
                $(
                    if f(
                        (&mut $name as *mut $name).cast::<u8>(),
                        TypeId::of::<$name>(),
                        mem::size_of::<$name>()
                    ) {
                        mem::forget($name)
                    }
                )*
            }
        }

        impl<$($name: Component),*> Bundle for ($($name,)*) {
            const TYPES: &'static [TypeInfo] = &{
                const fn is_less(left: &TypeInfo, right: &TypeInfo) -> bool {
                    unsafe {
                        left.layout().align() > right.layout().align() ||
                        left.layout().align() == right.layout().align() &&
                        mem::transmute::<TypeId, u64>(left.id()) < mem::transmute::<TypeId, u64>(right.id())
                    }
                }
    
                let mut types = [$(TypeInfo::of::<$name>()),*];
                let mut i = 1;
                while i < types.len() {
                    let temp = types[i];
                    let mut j = i;
                    while j > 0 && is_less(&temp, &types[j - 1]) {
                        types[j] = types[j - 1];
                        j -= 1;
                    }
                    types[j] = temp;
                    i += 1;
                }    
                types
            };

            const IDS: &'static [TypeId] = &{
                let types = Self::TYPES;
                let mut ids = [TypeId::of::<()>(); count!($($name),*)];
                let mut i = 0;
                while i < ids.len() {
                    ids[i] = types[i].id();
                    i += 1;
                }
                ids
            };

            #[allow(unused_variables, unused_mut)]
            unsafe fn get(mut f: impl FnMut(TypeId, usize) -> Option<NonNull<u8>>) -> Result<Self, MissingComponent> {
                #[allow(non_snake_case)]
                let ($(mut $name,)*) = ($(
                    f(TypeId::of::<$name>(), mem::size_of::<$name>()).ok_or_else(MissingComponent::new::<$name>)?
                        .as_ptr()
                        .cast::<$name>(),)*
                );
                Ok(($($name.read(),)*))
            }
        }
    }
}

macro_rules! count {
    () => { 0 };
    ($x: ident $(, $rest: ident)*) => { 1 + count!($($rest),*) };
}

smaller_tuples_too!(tuple_impl, O, N, M, L, K, J, I, H, G, F, E, D, C, B, A);
