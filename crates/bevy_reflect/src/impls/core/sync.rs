use crate::{
    error::ReflectCloneError,
    kind::{ReflectKind, ReflectMut, ReflectOwned, ReflectRef},
    prelude::*,
    reflect::{impl_full_reflect, ApplyError},
    type_info::{OpaqueInfo, TypeInfo, Typed},
    type_path::DynamicTypePath,
    type_registry::{FromType, GetTypeRegistration, ReflectFromPtr, TypeRegistration},
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
                    registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
                    registration.insert::<ReflectFromReflect>(FromType::<Self>::from_type());
                    registration.insert::<ReflectDefault>(FromType::<Self>::from_type());

                    // Serde only supports atomic types when the "std" feature is enabled
                    #[cfg(feature = "std")]
                    {
                        registration.insert::<crate::type_registry::ReflectSerialize>(FromType::<Self>::from_type());
                        registration.insert::<crate::type_registry::ReflectDeserialize>(FromType::<Self>::from_type());
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

            impl PartialReflect for $ty {
                #[inline]
                fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
                    Some(<Self as Typed>::type_info())
                }
                #[inline]
                fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
                    self
                }
                #[inline]
                fn as_partial_reflect(&self) -> &dyn PartialReflect {
                    self
                }
                #[inline]
                fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
                    self
                }
                #[inline]
                fn try_into_reflect(
                    self: Box<Self>,
                ) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
                    Ok(self)
                }
                #[inline]
                fn try_as_reflect(&self) -> Option<&dyn Reflect> {
                    Some(self)
                }
                #[inline]
                fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
                    Some(self)
                }

                #[inline]
                fn reflect_clone(&self) -> Result<Box<dyn Reflect>, ReflectCloneError> {
                    Ok(Box::new(<$ty>::new(self.load($ordering))))
                }

                #[inline]
                fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
                    if let Some(value) = value.try_downcast_ref::<Self>() {
                        *self = <$ty>::new(value.load($ordering));
                    } else {
                        return Err(ApplyError::MismatchedTypes {
                            from_type: Into::into(DynamicTypePath::reflect_type_path(value)),
                            to_type: Into::into(<Self as TypePath>::type_path()),
                        });
                    }
                    Ok(())
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
            }

            impl FromReflect for $ty {
                fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
                    Some(<$ty>::new(
                        reflect.try_downcast_ref::<$ty>()?.load($ordering),
                    ))
                }
            }
        };

        impl_full_reflect!(for $ty);
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
