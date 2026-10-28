macro_rules! impl_reflect_for_hashset {
    ($ty:path) => {
        const _: () = {
            impl<V, S> $crate::set::Set for $ty
            where
                V: $crate::Reflect
                    + $crate::from_dynamic::FromDynamic
                    + $crate::type_path::TypePath
                    + $crate::type_registry::GetTypeRegistration
                    + Eq
                    + core::hash::Hash,
                S: $crate::type_path::TypePath + core::hash::BuildHasher + Default + Send + Sync,
            {
                fn value_from_dynamic(
                    &self,
                    dynamic: $crate::dynamic::DynamicValue,
                ) -> Option<bevy_platform::prelude::Box<dyn $crate::reflect::Reflect>> {
                    V::from_dynamic(dynamic)
                        .map(|value| bevy_platform::prelude::Box::new(value) as _)
                }

                fn get(
                    &self,
                    value: &dyn $crate::reflect::Reflect,
                ) -> Option<&dyn $crate::reflect::Reflect> {
                    value
                        .downcast_ref::<V>()
                        .and_then(|value| Self::get(self, value))
                        .map(|value| value as &dyn $crate::reflect::Reflect)
                }

                fn len(&self) -> usize {
                    Self::len(self)
                }

                fn iter(
                    &self,
                ) -> bevy_platform::prelude::Box<
                    dyn Iterator<Item = &dyn $crate::reflect::Reflect> + '_,
                > {
                    let iter = self.iter().map(|v| v as &dyn $crate::reflect::Reflect);
                    bevy_platform::prelude::Box::new(iter)
                }

                fn drain(
                    &mut self,
                ) -> bevy_platform::prelude::Vec<
                    bevy_platform::prelude::Box<dyn $crate::reflect::Reflect>,
                > {
                    self.drain()
                        .map(|value| {
                            bevy_platform::prelude::Box::new(value)
                                as bevy_platform::prelude::Box<dyn $crate::reflect::Reflect>
                        })
                        .collect()
                }

                fn retain(&mut self, f: &mut dyn FnMut(&dyn $crate::reflect::Reflect) -> bool) {
                    self.retain(move |value| f(value));
                }

                fn insert_boxed(
                    &mut self,
                    value: bevy_platform::prelude::Box<dyn $crate::reflect::Reflect>,
                ) -> bool {
                    let value = *value.downcast::<V>().unwrap_or_else(|value| {
                        panic!(
                            "Attempted to insert invalid value of type {}.",
                            value.reflect_type_path()
                        )
                    });
                    self.insert(value)
                }

                fn remove(&mut self, value: &dyn $crate::reflect::Reflect) -> bool {
                    value
                        .downcast_ref::<V>()
                        .is_some_and(|value| self.remove(value))
                }

                fn contains(&self, value: &dyn $crate::reflect::Reflect) -> bool {
                    value
                        .downcast_ref::<V>()
                        .is_some_and(|value| self.contains(value))
                }
            }

            impl<V, S> $crate::reflect::Reflect for $ty
            where
                V: $crate::Reflect
                    + $crate::from_dynamic::FromDynamic
                    + $crate::type_path::TypePath
                    + $crate::type_registry::GetTypeRegistration
                    + Eq
                    + core::hash::Hash,
                S: $crate::type_path::TypePath + core::hash::BuildHasher + Default + Send + Sync,
            {
                fn reflect_kind(&self) -> $crate::kind::ReflectKind {
                    $crate::kind::ReflectKind::Set
                }

                fn reflect_ref(&self) -> $crate::kind::ReflectRef<'_> {
                    $crate::kind::ReflectRef::Set(self)
                }

                fn reflect_mut(&mut self) -> $crate::kind::ReflectMut<'_> {
                    $crate::kind::ReflectMut::Set(self)
                }

                fn reflect_owned(
                    self: bevy_platform::prelude::Box<Self>,
                ) -> $crate::kind::ReflectOwned {
                    $crate::kind::ReflectOwned::Set(self)
                }

                fn reflect_clone(
                    &self,
                ) -> Result<
                    bevy_platform::prelude::Box<dyn $crate::reflect::Reflect>,
                    $crate::error::ReflectCloneError,
                > {
                    let mut set = Self::with_capacity_and_hasher(self.len(), S::default());
                    for value in self.iter() {
                        let value = value.reflect_clone_and_take()?;
                        set.insert(value);
                    }

                    Ok(bevy_platform::prelude::Box::new(set))
                }

                fn reflect_partial_eq(&self, value: &dyn $crate::reflect::Reflect) -> Option<bool> {
                    $crate::set::set_partial_eq(self, value)
                }

                $crate::impl_reflect_casts! {}
            }

            impl<V, S> $crate::info::Typed for $ty
            where
                V: $crate::Reflect
                    + $crate::from_dynamic::FromDynamic
                    + $crate::type_path::TypePath
                    + $crate::type_registry::GetTypeRegistration
                    + Eq
                    + core::hash::Hash,
                S: $crate::type_path::TypePath + core::hash::BuildHasher + Default + Send + Sync,
            {
                fn type_info() -> &'static $crate::info::TypeInfo {
                    static CELL: $crate::utility::GenericTypeInfoCell =
                        $crate::utility::GenericTypeInfoCell::new();
                    CELL.get_or_insert::<Self, _>(|| {
                        $crate::info::TypeInfo::Set(
                            $crate::set::SetInfo::new::<Self, V>().with_generics(
                                $crate::generics::Generics::from_iter([
                                    $crate::generics::TypeParamInfo::new::<V>("V"),
                                ]),
                            ),
                        )
                    })
                }
            }

            impl<V, S> $crate::type_registry::GetTypeRegistration for $ty
            where
                V: $crate::Reflect
                    + $crate::from_dynamic::FromDynamic
                    + $crate::type_path::TypePath
                    + $crate::type_registry::GetTypeRegistration
                    + Eq
                    + core::hash::Hash,
                S: $crate::type_path::TypePath
                    + core::hash::BuildHasher
                    + Default
                    + Send
                    + Sync
                    + Default,
            {
                fn get_type_registration() -> $crate::type_registry::TypeRegistration {
                    let mut registration = $crate::type_registry::TypeRegistration::of::<Self>();
                    registration
                        .register_type_data::<$crate::type_registry::ReflectFromPtr, Self>();
                    registration
                        .register_type_data::<$crate::from_dynamic::ReflectFromDynamic, Self>();
                    registration
                }

                fn register_type_dependencies(registry: &mut $crate::type_registry::TypeRegistry) {
                    registry.register::<V>();
                }
            }

            impl<V, S> $crate::from_dynamic::FromDynamic for $ty
            where
                V: $crate::from_dynamic::FromDynamic
                    + $crate::type_path::TypePath
                    + $crate::type_registry::GetTypeRegistration
                    + Eq
                    + core::hash::Hash,
                S: $crate::type_path::TypePath + core::hash::BuildHasher + Default + Send + Sync,
            {
                fn from_dynamic(dynamic: $crate::dynamic::DynamicValue) -> Option<Self> {
                    let dynamic = match dynamic {
                        $crate::dynamic::DynamicValue::Set(dynamic) => dynamic,
                        $crate::dynamic::DynamicValue::Concrete(concrete) => {
                            return concrete.take().ok()
                        }
                        _ => return None,
                    };

                    let mut new_set = Self::with_capacity_and_hasher(dynamic.len(), S::default());

                    for value in dynamic.into_iter() {
                        let new_value = V::from_dynamic(value)?;
                        new_set.insert(new_value);
                    }

                    Some(new_set)
                }
            }
        };
    };
}

pub(crate) use impl_reflect_for_hashset;
