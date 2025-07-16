macro_rules! impl_reflect_for_hashmap {
    ($ty:path) => {
        const _: () = {
            impl<K, V, S> $crate::map::Map for $ty
            where
                K: $crate::from_reflect::FromReflect + $crate::type_info::MaybeTyped + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration + Eq + core::hash::Hash,
                V: $crate::from_reflect::FromReflect + $crate::type_info::MaybeTyped + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration,
                S: $crate::type_path::TypePath + core::hash::BuildHasher + Default + Send + Sync,
            {
                fn get(&self, key: &dyn $crate::reflect::PartialReflect) -> Option<&dyn $crate::reflect::PartialReflect> {
                    key.try_downcast_ref::<K>()
                        .and_then(|key| Self::get(self, key))
                        .map(|value| value as &dyn $crate::reflect::PartialReflect)
                }

                fn get_mut(&mut self, key: &dyn $crate::reflect::PartialReflect) -> Option<&mut dyn $crate::reflect::PartialReflect> {
                    key.try_downcast_ref::<K>()
                        .and_then(move |key| Self::get_mut(self, key))
                        .map(|value| value as &mut dyn $crate::reflect::PartialReflect)
                }

                fn len(&self) -> usize {
                    Self::len(self)
                }

                fn iter(&self) -> bevy_platform::prelude::Box<dyn Iterator<Item = (&dyn $crate::reflect::PartialReflect, &dyn $crate::reflect::PartialReflect)> + '_> {
                    bevy_platform::prelude::Box::new(self.iter().map(|(k, v)| (k as &dyn $crate::reflect::PartialReflect, v as &dyn $crate::reflect::PartialReflect)))
                }

                fn drain(&mut self) -> bevy_platform::prelude::Vec<(bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>, bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>)> {
                    self.drain()
                        .map(|(key, value)| {
                            (
                                bevy_platform::prelude::Box::new(key) as bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>,
                                bevy_platform::prelude::Box::new(value) as bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>,
                            )
                        })
                        .collect()
                }

                fn retain(&mut self, f: &mut dyn FnMut(&dyn $crate::reflect::PartialReflect, &mut dyn $crate::reflect::PartialReflect) -> bool) {
                    self.retain(move |key, value| f(key, value));
                }

                fn to_dynamic_map(&self) -> $crate::map::DynamicMap {
                    let mut dynamic_map = $crate::map::DynamicMap::default();
                    dynamic_map.set_represented_type($crate::reflect::PartialReflect::get_represented_type_info(self));
                    for (k, v) in self {
                        let key = K::from_reflect(k).unwrap_or_else(|| {
                            panic!(
                                "Attempted to clone invalid key of type {}.",
                                k.reflect_type_path()
                            )
                        });
                        dynamic_map.insert_boxed(bevy_platform::prelude::Box::new(key), v.to_dynamic());
                    }
                    dynamic_map
                }

                fn insert_boxed(
                    &mut self,
                    key: bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>,
                    value: bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>,
                ) -> Option<bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>> {
                    let key = K::take_from_reflect(key).unwrap_or_else(|key| {
                        panic!(
                            "Attempted to insert invalid key of type {}.",
                            key.reflect_type_path()
                        )
                    });
                    let value = V::take_from_reflect(value).unwrap_or_else(|value| {
                        panic!(
                            "Attempted to insert invalid value of type {}.",
                            value.reflect_type_path()
                        )
                    });
                    self.insert(key, value)
                        .map(|old_value| bevy_platform::prelude::Box::new(old_value) as bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>)
                }

                fn remove(&mut self, key: &dyn $crate::reflect::PartialReflect) -> Option<bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>> {
                    let mut from_reflect = None;
                    key.try_downcast_ref::<K>()
                        .or_else(|| {
                            from_reflect = K::from_reflect(key);
                            from_reflect.as_ref()
                        })
                        .and_then(|key| self.remove(key))
                        .map(|value| bevy_platform::prelude::Box::new(value) as bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>)
                }
            }

            impl<K, V, S> $crate::reflect::PartialReflect for $ty
            where
                K: $crate::from_reflect::FromReflect + $crate::type_info::MaybeTyped + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration + Eq + core::hash::Hash,
                V: $crate::from_reflect::FromReflect + $crate::type_info::MaybeTyped + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration,
                S: $crate::type_path::TypePath + core::hash::BuildHasher + Default + Send + Sync,
            {
                fn get_represented_type_info(&self) -> Option<&'static $crate::type_info::TypeInfo> {
                    Some(<Self as $crate::type_info::Typed>::type_info())
                }

                #[inline]
                fn into_partial_reflect(self: bevy_platform::prelude::Box<Self>) -> bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect> {
                    self
                }

                fn as_partial_reflect(&self) -> &dyn $crate::reflect::PartialReflect {
                    self
                }

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
                    $crate::kind::ReflectKind::Map
                }

                fn reflect_ref(&self) -> $crate::kind::ReflectRef {
                    $crate::kind::ReflectRef::Map(self)
                }

                fn reflect_mut(&mut self) -> $crate::kind::ReflectMut {
                    $crate::kind::ReflectMut::Map(self)
                }

                fn reflect_owned(self: bevy_platform::prelude::Box<Self>) -> $crate::kind::ReflectOwned {
                    $crate::kind::ReflectOwned::Map(self)
                }

                fn reflect_clone(&self) -> Result<bevy_platform::prelude::Box<dyn $crate::reflect::Reflect>, $crate::error::ReflectCloneError> {
                    let mut map = Self::with_capacity_and_hasher(self.len(), S::default());
                    for (key, value) in self.iter() {
                        let key = key.reflect_clone_and_take()?;
                        let value = value.reflect_clone_and_take()?;
                        map.insert(key, value);
                    }

                    Ok(bevy_platform::prelude::Box::new(map))
                }

                fn reflect_partial_eq(&self, value: &dyn $crate::reflect::PartialReflect) -> Option<bool> {
                    $crate::map::map_partial_eq(self, value)
                }

                fn apply(&mut self, value: &dyn $crate::reflect::PartialReflect) {
                    $crate::map::map_apply(self, value);
                }

                fn try_apply(&mut self, value: &dyn $crate::reflect::PartialReflect) -> Result<(), $crate::reflect::ApplyError> {
                    $crate::map::map_try_apply(self, value)
                }
            }

            $crate::impl_full_reflect!(
                <K, V, S> for $ty
                where
                    K: $crate::from_reflect::FromReflect + $crate::type_info::MaybeTyped + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration + Eq + core::hash::Hash,
                    V: $crate::from_reflect::FromReflect + $crate::type_info::MaybeTyped + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration,
                    S: $crate::type_path::TypePath + core::hash::BuildHasher + Default + Send + Sync,
            );

            impl<K, V, S> $crate::type_info::Typed for $ty
            where
                K: $crate::from_reflect::FromReflect + $crate::type_info::MaybeTyped + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration + Eq + core::hash::Hash,
                V: $crate::from_reflect::FromReflect + $crate::type_info::MaybeTyped + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration,
                S: $crate::type_path::TypePath + core::hash::BuildHasher + Default + Send + Sync,
            {
                fn type_info() -> &'static $crate::type_info::TypeInfo {
                    static CELL: $crate::utility::GenericTypeInfoCell = $crate::utility::GenericTypeInfoCell::new();
                    CELL.get_or_insert::<Self, _>(|| {
                        $crate::type_info::TypeInfo::Map(
                            $crate::map::MapInfo::new::<Self, K, V>().with_generics($crate::generics::Generics::from_iter([
                                $crate::generics::TypeParamInfo::new::<K>("K"),
                                $crate::generics::TypeParamInfo::new::<V>("V"),
                            ])),
                        )
                    })
                }
            }

            impl<K, V, S> $crate::type_registry::GetTypeRegistration for $ty
            where
                K: $crate::from_reflect::FromReflect + $crate::type_info::MaybeTyped + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration + Eq + core::hash::Hash,
                V: $crate::from_reflect::FromReflect + $crate::type_info::MaybeTyped + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration,
                S: $crate::type_path::TypePath + core::hash::BuildHasher + Default + Send + Sync + Default,
            {
                fn get_type_registration() -> $crate::type_registry::TypeRegistration {
                    let mut registration = $crate::type_registry::TypeRegistration::of::<Self>();
                    registration.insert::<$crate::type_registry::ReflectFromPtr>($crate::type_registry::FromType::<Self>::from_type());
                    registration.insert::<$crate::from_reflect::ReflectFromReflect>($crate::type_registry::FromType::<Self>::from_type());
                    registration
                }

                fn register_type_dependencies(registry: &mut $crate::type_registry::TypeRegistry) {
                    registry.register::<K>();
                    registry.register::<V>();
                }
            }

            impl<K, V, S> $crate::from_reflect::FromReflect for $ty
            where
                K: $crate::from_reflect::FromReflect + $crate::type_info::MaybeTyped + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration + Eq + core::hash::Hash,
                V: $crate::from_reflect::FromReflect + $crate::type_info::MaybeTyped + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration,
                S: $crate::type_path::TypePath + core::hash::BuildHasher + Default + Send + Sync,
            {
                fn from_reflect(reflect: &dyn $crate::reflect::PartialReflect) -> Option<Self> {
                    let ref_map = reflect.reflect_ref().as_map().ok()?;

                    let mut new_map = Self::with_capacity_and_hasher(ref_map.len(), S::default());

                    for (key, value) in ref_map.iter() {
                        let new_key = K::from_reflect(key)?;
                        let new_value = V::from_reflect(value)?;
                        new_map.insert(new_key, new_value);
                    }

                    Some(new_map)
                }
            }
        };
    };
}

pub(crate) use impl_reflect_for_hashmap;
