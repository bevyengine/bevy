macro_rules! impl_reflect_for_veclike {
    ($ty:ty, $insert:expr, $remove:expr, $push:expr, $pop:expr, $sub:ty) => {
        const _: () = {
            impl<T: $crate::from_reflect::FromReflect + $crate::type_info::MaybeTyped + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration> $crate::list::List for $ty {
                #[inline]
                fn get(&self, index: usize) -> Option<&dyn $crate::reflect::PartialReflect> {
                    <$sub>::get(self, index).map(|value| value as &dyn $crate::reflect::PartialReflect)
                }

                #[inline]
                fn get_mut(&mut self, index: usize) -> Option<&mut dyn $crate::reflect::PartialReflect> {
                    <$sub>::get_mut(self, index).map(|value| value as &mut dyn $crate::reflect::PartialReflect)
                }

                fn insert(&mut self, index: usize, value: bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>) {
                    let value = value.try_take::<T>().unwrap_or_else(|value| {
                        T::from_reflect(&*value).unwrap_or_else(|| {
                            panic!(
                                "Attempted to insert invalid value of type {}.",
                                value.reflect_type_path()
                            )
                        })
                    });
                    $insert(self, index, value);
                }

                fn remove(&mut self, index: usize) -> bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect> {
                    bevy_platform::prelude::Box::new($remove(self, index))
                }

                fn push(&mut self, value: bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>) {
                    let value = T::take_from_reflect(value).unwrap_or_else(|value| {
                        panic!(
                            "Attempted to push invalid value of type {}.",
                            value.reflect_type_path()
                        )
                    });
                    $push(self, value);
                }

                fn pop(&mut self) -> Option<bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>> {
                    $pop(self).map(|value| bevy_platform::prelude::Box::new(value) as bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>)
                }

                #[inline]
                fn len(&self) -> usize {
                    <$sub>::len(self)
                }

                #[inline]
                fn iter(&self) -> $crate::list::ListIter {
                    $crate::list::ListIter::new(self)
                }

                #[inline]
                fn drain(&mut self) -> alloc::vec::Vec<bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>> {
                    self.drain(..)
                        .map(|value| bevy_platform::prelude::Box::new(value) as bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>)
                        .collect()
                }
            }

            impl<T: $crate::from_reflect::FromReflect + $crate::type_info::MaybeTyped + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration> $crate::reflect::PartialReflect for $ty {
                #[inline]
                fn get_represented_type_info(&self) -> Option<&'static $crate::type_info::TypeInfo> {
                    Some(<Self as $crate::type_info::Typed>::type_info())
                }

                fn into_partial_reflect(self: bevy_platform::prelude::Box<Self>) -> bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect> {
                    self
                }

                #[inline]
                fn as_partial_reflect(&self) -> &dyn $crate::reflect::PartialReflect {
                    self
                }

                #[inline]
                fn as_partial_reflect_mut(&mut self) -> &mut dyn $crate::reflect::PartialReflect {
                    self
                }

                fn try_into_reflect(
                    self: bevy_platform::prelude::Box<Self>,
                ) -> Result<bevy_platform::prelude::Box<dyn $crate::reflect::Reflect>, bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>> {
                    Ok(self)
                }

                fn try_as_reflect(&self) -> Option<&dyn $crate::reflect::Reflect> {
                    Some(self)
                }

                fn try_as_reflect_mut(&mut self) -> Option<&mut dyn $crate::reflect::Reflect> {
                    Some(self)
                }

                fn reflect_kind(&self) -> $crate::kind::ReflectKind {
                    $crate::kind::ReflectKind::List
                }

                fn reflect_ref(&self) -> $crate::kind::ReflectRef {
                    $crate::kind::ReflectRef::List(self)
                }

                fn reflect_mut(&mut self) -> $crate::kind::ReflectMut {
                    $crate::kind::ReflectMut::List(self)
                }

                fn reflect_owned(self: bevy_platform::prelude::Box<Self>) -> $crate::kind::ReflectOwned {
                    $crate::kind::ReflectOwned::List(self)
                }

                fn reflect_clone(&self) -> Result<bevy_platform::prelude::Box<dyn $crate::reflect::Reflect>, $crate::error::ReflectCloneError> {
                    Ok(bevy_platform::prelude::Box::new(
                        self.iter()
                            .map(|value| value.reflect_clone_and_take())
                            .collect::<Result<Self, $crate::error::ReflectCloneError>>()?,
                    ))
                }

                fn reflect_hash(&self) -> Option<u64> {
                    $crate::list::list_hash(self)
                }

                fn reflect_partial_eq(&self, value: &dyn $crate::reflect::PartialReflect) -> Option<bool> {
                    $crate::list::list_partial_eq(self, value)
                }

                fn apply(&mut self, value: &dyn $crate::reflect::PartialReflect) {
                    $crate::list::list_apply(self, value);
                }

                fn try_apply(&mut self, value: &dyn $crate::reflect::PartialReflect) -> Result<(), $crate::reflect::ApplyError> {
                    $crate::list::list_try_apply(self, value)
                }
            }

            $crate::impl_full_reflect!(<T> for $ty where T: $crate::from_reflect::FromReflect + $crate::type_info::MaybeTyped + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration);

            impl<T: $crate::from_reflect::FromReflect + $crate::type_info::MaybeTyped + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration> $crate::type_info::Typed for $ty {
                fn type_info() -> &'static $crate::type_info::TypeInfo {
                    static CELL: $crate::utility::GenericTypeInfoCell = $crate::utility::GenericTypeInfoCell::new();
                    CELL.get_or_insert::<Self, _>(|| {
                        $crate::type_info::TypeInfo::List(
                            $crate::list::ListInfo::new::<Self, T>().with_generics($crate::generics::Generics::from_iter([
                                $crate::generics::TypeParamInfo::new::<T>("T")
                            ]))
                        )
                    })
                }
            }

            impl<T: $crate::from_reflect::FromReflect + $crate::type_info::MaybeTyped + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration> $crate::type_registry::GetTypeRegistration
                for $ty
            {
                fn get_type_registration() -> $crate::type_registry::TypeRegistration {
                    let mut registration = $crate::type_registry::TypeRegistration::of::<$ty>();
                    registration.insert::<$crate::type_registry::ReflectFromPtr>($crate::type_registry::FromType::<$ty>::from_type());
                    registration.insert::<$crate::from_reflect::ReflectFromReflect>($crate::type_registry::FromType::<$ty>::from_type());
                    registration
                }

                fn register_type_dependencies(registry: &mut $crate::type_registry::TypeRegistry) {
                    registry.register::<T>();
                }
            }

            impl<T: $crate::from_reflect::FromReflect + $crate::type_info::MaybeTyped + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration> $crate::from_reflect::FromReflect for $ty {
                fn from_reflect(reflect: &dyn $crate::reflect::PartialReflect) -> Option<Self> {
                    let ref_list = reflect.reflect_ref().as_list().ok()?;

                    let mut new_list = Self::with_capacity(ref_list.len());

                    for field in ref_list.iter() {
                        $push(&mut new_list, T::from_reflect(field)?);
                    }

                    Some(new_list)
                }
            }
        };
    };
}

pub(crate) use impl_reflect_for_veclike;
