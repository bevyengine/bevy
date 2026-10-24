use crate::{
    dynamic::DynamicValue,
    error::ReflectCloneError,
    info::{OpaqueInfo, TypeInfo, Typed},
    kind::{ReflectKind, ReflectMut, ReflectOwned, ReflectRef},
    prelude::*,
    reflect::impl_reflect_casts,
    type_registry::{GetTypeRegistration, ReflectFromPtr, TypeRegistration},
    utility::NonGenericTypeInfoCell,
};
use bevy_platform::prelude::*;
use bevy_reflect_derive::impl_type_path;
use core::fmt;

macro_rules! impl_reflect_for_atomic {
    ($ty:ty, $ordering:expr) => {
        impl_type_path!($ty);

        const _: () = {
            #[cfg(feature = "functions")]
            crate::func::macros::impl_function_traits!($ty);

            impl GetTypeRegistration for $ty {
                fn get_type_registration() -> TypeRegistration {
                    let mut registration = TypeRegistration::of::<Self>();
                    registration.register_type_data::<ReflectFromPtr, Self>();
                    registration.register_type_data::<ReflectFromDynamic, Self>();
                    registration.register_type_data::<ReflectDefault, Self>();

                    // Serde only supports atomic types when the "std" feature is enabled
                    #[cfg(feature = "std")]
                    {
                        registration
                            .register_type_data::<crate::type_registry::ReflectSerialize, Self>();
                        registration
                            .register_type_data::<crate::type_registry::ReflectDeserialize, Self>();
                    }

                    registration
                }
            }

            impl Typed for $ty {
                fn type_info() -> &'static TypeInfo {
                    static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
                    CELL.get_or_set(|| {
                        let info = OpaqueInfo::new::<Self>();
                        TypeInfo::Opaque(info)
                    })
                }
            }

            impl Reflect for $ty {
                #[inline]
                fn reflect_clone(&self) -> Result<Box<dyn Reflect>, ReflectCloneError> {
                    Ok(Box::new(<$ty>::new(self.load($ordering))))
                }
                #[inline]
                fn reflect_kind(&self) -> ReflectKind {
                    ReflectKind::Opaque
                }
                #[inline]
                fn reflect_ref(&self) -> ReflectRef<'_> {
                    ReflectRef::Opaque(self)
                }
                #[inline]
                fn reflect_mut(&mut self) -> ReflectMut<'_> {
                    ReflectMut::Opaque(self)
                }
                #[inline]
                fn reflect_owned(self: Box<Self>) -> ReflectOwned {
                    ReflectOwned::Opaque(self)
                }
                fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    fmt::Debug::fmt(self, f)
                }

                impl_reflect_casts! {}
            }

            impl FromDynamic for $ty {
                fn from_dynamic(dynamic: DynamicValue) -> Option<Self> {
                    Some(<$ty>::new(
                        dynamic
                            .into_concrete()?
                            .downcast_ref::<$ty>()?
                            .load($ordering),
                    ))
                }
            }
        };
    };
}

impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicIsize,
    ::core::sync::atomic::Ordering::SeqCst
);
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicUsize,
    ::core::sync::atomic::Ordering::SeqCst
);
#[cfg(target_has_atomic = "64")]
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicI64,
    ::core::sync::atomic::Ordering::SeqCst
);
#[cfg(target_has_atomic = "64")]
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicU64,
    ::core::sync::atomic::Ordering::SeqCst
);
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicI32,
    ::core::sync::atomic::Ordering::SeqCst
);
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicU32,
    ::core::sync::atomic::Ordering::SeqCst
);
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicI16,
    ::core::sync::atomic::Ordering::SeqCst
);
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicU16,
    ::core::sync::atomic::Ordering::SeqCst
);
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicI8,
    ::core::sync::atomic::Ordering::SeqCst
);
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicU8,
    ::core::sync::atomic::Ordering::SeqCst
);
impl_reflect_for_atomic!(
    ::core::sync::atomic::AtomicBool,
    ::core::sync::atomic::Ordering::SeqCst
);
