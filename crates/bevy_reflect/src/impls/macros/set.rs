macro_rules! impl_reflect_for_hashset {
    ($ty:path) => {
        const _: () = {
            impl<V, S> $crate::set::Set for $ty
            where
                V: $crate::from_reflect::FromReflect + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration + Eq + core::hash::Hash,
                S: $crate::type_path::TypePath + core::hash::BuildHasher + Default + Send + Sync,
            {
                fn get(&self, value: &dyn $crate::reflect::PartialReflect) -> Option<&dyn $crate::reflect::PartialReflect> {
                    value
                        .try_downcast_ref::<V>()
                        .and_then(|value| Self::get(self, value))
                        .map(|value| value as &dyn $crate::reflect::PartialReflect)
                }

                fn len(&self) -> usize {
                    Self::len(self)
                }

                fn iter(&self) -> bevy_platform::prelude::Box<dyn Iterator<Item = &dyn $crate::reflect::PartialReflect> + '_> {
                    let iter = self.iter().map(|v| v as &dyn $crate::reflect::PartialReflect);
                    bevy_platform::prelude::Box::new(iter)
                }

                fn drain(&mut self) -> bevy_platform::prelude::Vec<bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>> {
                    self.drain()
                        .map(|value| bevy_platform::prelude::Box::new(value) as bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>)
                        .collect()
                }

                fn retain(&mut self, f: &mut dyn FnMut(&dyn $crate::reflect::PartialReflect) -> bool) {
                    self.retain(move |value| f(value));
                }

                fn insert_boxed(&mut self, value: bevy_platform::prelude::Box<dyn $crate::reflect::PartialReflect>) -> bool {
                    let value = V::take_from_reflect(value).unwrap_or_else(|value| {
                        panic!(
                            "Attempted to insert invalid value of type {}.",
                            value.reflect_type_path()
                        )
                    });
                    self.insert(value)
                }

                fn remove(&mut self, value: &dyn $crate::reflect::PartialReflect) -> bool {
                    let mut from_reflect = None;
                    value
                        .try_downcast_ref::<V>()
                        .or_else(|| {
                            from_reflect = V::from_reflect(value);
                            from_reflect.as_ref()
                        })
                        .is_some_and(|value| self.remove(value))
                }

                fn contains(&self, value: &dyn $crate::reflect::PartialReflect) -> bool {
                    let mut from_reflect = None;
                    value
                        .try_downcast_ref::<V>()
                        .or_else(|| {
                            from_reflect = V::from_reflect(value);
                            from_reflect.as_ref()
                        })
                        .is_some_and(|value| self.contains(value))
                }
            }

            impl<V, S> $crate::reflect::PartialReflect for $ty
            where
                V: $crate::from_reflect::FromReflect + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration + Eq + core::hash::Hash,
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

                #[inline]
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

                fn apply(&mut self, value: &dyn $crate::reflect::PartialReflect) {
                   $crate::set::set_apply(self, value);
                }

                fn try_apply(&mut self, value: &dyn $crate::reflect::PartialReflect) -> Result<(), $crate::reflect::ApplyError> {
                    $crate::set::set_try_apply(self, value)
                }

                fn reflect_kind(&self) -> $crate::kind::ReflectKind {
                    $crate::kind::ReflectKind::Set
                }

                fn reflect_ref(&self) -> $crate::kind::ReflectRef {
                    $crate::kind::ReflectRef::Set(self)
                }

                fn reflect_mut(&mut self) -> $crate::kind::ReflectMut {
                    $crate::kind::ReflectMut::Set(self)
                }

                fn reflect_owned(self: bevy_platform::prelude::Box<Self>) -> $crate::kind::ReflectOwned {
                    $crate::kind::ReflectOwned::Set(self)
                }

                fn reflect_clone(&self) -> Result<bevy_platform::prelude::Box<dyn $crate::reflect::Reflect>, $crate::error::ReflectCloneError> {
                    let mut set = Self::with_capacity_and_hasher(self.len(), S::default());
                    for value in self.iter() {
                        let value = value.reflect_clone_and_take()?;
                        set.insert(value);
                    }

                    Ok(bevy_platform::prelude::Box::new(set))
                }

                fn reflect_partial_eq(&self, value: &dyn $crate::reflect::PartialReflect) -> Option<bool> {
                    $crate::set::set_partial_eq(self, value)
                }
            }

            impl<V, S> $crate::type_info::Typed for $ty
            where
                V: $crate::from_reflect::FromReflect + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration + Eq + core::hash::Hash,
                S: $crate::type_path::TypePath + core::hash::BuildHasher + Default + Send + Sync,
            {
                fn type_info() -> &'static $crate::type_info::TypeInfo {
                    static CELL: $crate::utility::GenericTypeInfoCell = $crate::utility::GenericTypeInfoCell::new();
                    CELL.get_or_insert::<Self, _>(|| {
                        $crate::type_info::TypeInfo::Set(
                            $crate::set::SetInfo::new::<Self, V>().with_generics($crate::generics::Generics::from_iter([
                                $crate::generics::TypeParamInfo::new::<V>("V")
                            ]))
                        )
                    })
                }
            }

            impl<V, S> $crate::type_registry::GetTypeRegistration for $ty
            where
                V: $crate::from_reflect::FromReflect + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration + Eq + core::hash::Hash,
                S: $crate::type_path::TypePath + core::hash::BuildHasher + Default + Send + Sync + Default,
            {
                fn get_type_registration() -> $crate::type_registry::TypeRegistration {
                    let mut registration = $crate::type_registry::TypeRegistration::of::<Self>();
                    registration.insert::<$crate::type_registry::ReflectFromPtr>($crate::type_registry::FromType::<Self>::from_type());
                    registration.insert::<$crate::from_reflect::ReflectFromReflect>($crate::type_registry::FromType::<Self>::from_type());
                    registration
                }

                fn register_type_dependencies(registry: &mut $crate::type_registry::TypeRegistry) {
                    registry.register::<V>();
                }
            }

            $crate::impl_full_reflect!(
                <V, S> for $ty
                where
                    V: $crate::from_reflect::FromReflect + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration + Eq + core::hash::Hash,
                    S: $crate::type_path::TypePath + core::hash::BuildHasher + Default + Send + Sync,
            );

            impl<V, S> $crate::from_reflect::FromReflect for $ty
            where
                V: $crate::from_reflect::FromReflect + $crate::type_path::TypePath + $crate::type_registry::GetTypeRegistration + Eq + core::hash::Hash,
                S: $crate::type_path::TypePath + core::hash::BuildHasher + Default + Send + Sync,
            {
                fn from_reflect(reflect: &dyn $crate::reflect::PartialReflect) -> Option<Self> {
                    let ref_set = reflect.reflect_ref().as_set().ok()?;

                    let mut new_set = Self::with_capacity_and_hasher(ref_set.len(), S::default());

                    for value in ref_set.iter() {
                        let new_value = V::from_reflect(value)?;
                        new_set.insert(new_value);
                    }

                    Some(new_set)
                }
            }
        };
    };
}

pub(crate) use impl_reflect_for_hashset;
