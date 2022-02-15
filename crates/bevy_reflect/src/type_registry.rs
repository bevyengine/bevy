use crate::Reflect;
use bevy_utils::{HashMap, HashSet};
use downcast_rs::{impl_downcast, Downcast};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use serde::Deserialize;
use std::{any::TypeId, fmt::Debug, sync::Arc};

/// A registry of reflected types.
#[derive(Default)]
pub struct TypeRegistry {
    registrations: HashMap<TypeId, TypeRegistration>,
    short_name_to_id: HashMap<String, TypeId>,
    full_name_to_id: HashMap<String, TypeId>,
    ambiguous_names: HashSet<String>,
}

// TODO:  remove this wrapper once we migrate to Atelier Assets and the Scene AssetLoader doesn't
// need a TypeRegistry ref
/// A synchronized wrapper around a [`TypeRegistry`].
#[derive(Clone, Default)]
pub struct TypeRegistryArc {
    pub internal: Arc<RwLock<TypeRegistry>>,
}

impl Debug for TypeRegistryArc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.internal.read().full_name_to_id.keys().fmt(f)
    }
}

/// A trait which allows a type to generate its [`TypeRegistration`].
///
/// This trait is automatically implemented for types which derive [`Reflect`].
pub trait GetTypeRegistration {
    fn get_type_registration() -> TypeRegistration;
}

impl TypeRegistry {
    /// Registers the type `T`.
    pub fn register<T>(&mut self)
    where
        T: GetTypeRegistration,
    {
        self.add_registration(T::get_type_registration());
    }

    /// Registers the type described by `registration`.
    pub fn add_registration(&mut self, registration: TypeRegistration) {
        let short_name = registration.short_name.to_string();
        if self.short_name_to_id.contains_key(&short_name)
            || self.ambiguous_names.contains(&short_name)
        {
            // name is ambiguous. fall back to long names for all ambiguous types
            self.short_name_to_id.remove(&short_name);
            self.ambiguous_names.insert(short_name);
        } else {
            self.short_name_to_id
                .insert(short_name, registration.type_id);
        }
        self.full_name_to_id
            .insert(registration.name.to_string(), registration.type_id);
        self.registrations
            .insert(registration.type_id, registration);
    }

    /// Returns a reference to the [`TypeRegistration`] of the type with the
    /// given [`TypeId`].
    ///
    /// If the specified type has not been registered, returns `None`.
    ///
    /// [`TypeId`]: std::any::TypeId
    pub fn get(&self, type_id: TypeId) -> Option<&TypeRegistration> {
        self.registrations.get(&type_id)
    }

    /// Returns a mutable reference to the [`TypeRegistration`] of the type with
    /// the given [`TypeId`].
    ///
    /// If the specified type has not been registered, returns `None`.
    ///
    /// [`TypeId`]: std::any::TypeId
    pub fn get_mut(&mut self, type_id: TypeId) -> Option<&mut TypeRegistration> {
        self.registrations.get_mut(&type_id)
    }

    /// Returns a reference to the [`TypeRegistration`] of the type with the
    /// given name.
    ///
    /// If no type with the given name has been registered, returns `None`.
    pub fn get_with_name(&self, type_name: &str) -> Option<&TypeRegistration> {
        self.full_name_to_id
            .get(type_name)
            .and_then(|id| self.get(*id))
    }

    /// Returns a mutable reference to the [`TypeRegistration`] of the type with
    /// the given name.
    ///
    /// If no type with the given name has been registered, returns `None`.
    pub fn get_with_name_mut(&mut self, type_name: &str) -> Option<&mut TypeRegistration> {
        self.full_name_to_id
            .get(type_name)
            .cloned()
            .and_then(move |id| self.get_mut(id))
    }

    /// Returns a mutable reference to the [`TypeRegistration`] of the type with
    /// the given short name.
    ///
    /// If the short name is ambiguous, or if no type with the given short name
    /// has been registered, returns `None`.
    pub fn get_with_short_name(&self, short_type_name: &str) -> Option<&TypeRegistration> {
        self.short_name_to_id
            .get(short_type_name)
            .and_then(|id| self.registrations.get(id))
    }

    /// Returns the [`TypeData`] of type `T` associated with the given `TypeId`.
    ///
    /// The returned value may be used to downcast [`Reflect`] trait objects to
    /// trait objects of the trait used to generate `T`, provided that the
    /// underlying reflected type has the proper `#[reflect(DoThing)]`
    /// attribute.
    ///
    /// If the specified type has not been registered, or if `T` is not present
    /// in its type registration, returns `None`.
    pub fn get_type_data<T: TypeData>(&self, type_id: TypeId) -> Option<&T> {
        self.get(type_id)
            .and_then(|registration| registration.data::<T>())
    }

    /// Returns an iterator overed the [`TypeRegistration`]s of the registered
    /// types.
    pub fn iter(&self) -> impl Iterator<Item = &TypeRegistration> {
        self.registrations.values()
    }
}

impl TypeRegistryArc {
    /// Takes a read lock on the underlying [`TypeRegistry`].
    pub fn read(&self) -> RwLockReadGuard<'_, TypeRegistry> {
        self.internal.read()
    }

    /// Takes a write lock on the underlying [`TypeRegistry`].
    pub fn write(&self) -> RwLockWriteGuard<'_, TypeRegistry> {
        self.internal.write()
    }
}

/// A record of data about a type.
///
/// This contains the [`TypeId`], [name], and [short name] of the type.
///
/// For each trait specified by the [`#[reflect(_)]`][0] attribute of
/// [`#[derive(Reflect)]`][1] on the registered type, this record also contains
/// a [`TypeData`] which can be used to downcast [`Reflect`] trait objects of
/// this type to trait objects of the relevant trait.
///
/// [`TypeId`]: std::any::TypeId
/// [name]: std::any::type_name
/// [short name]: TypeRegistration::get_short_name
/// [0]: crate::Reflect
/// [1]: crate::Reflect
pub struct TypeRegistration {
    type_id: TypeId,
    short_name: String,
    name: &'static str,
    data: HashMap<TypeId, Box<dyn TypeData>>,
}

impl TypeRegistration {
    /// Returns the [`TypeId`] of the type.
    ///
    /// [`TypeId`]: std::any::TypeId
    #[inline]
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Returns a reference to the value of type `T` in this registration's type
    /// data.
    ///
    /// Returns `None` if no such value exists.
    pub fn data<T: TypeData>(&self) -> Option<&T> {
        self.data
            .get(&TypeId::of::<T>())
            .and_then(|value| value.downcast_ref())
    }

    /// Returns a mutable reference to the value of type `T` in this
    /// registration's type data.
    ///
    /// Returns `None` if no such value exists.
    pub fn data_mut<T: TypeData>(&mut self) -> Option<&mut T> {
        self.data
            .get_mut(&TypeId::of::<T>())
            .and_then(|value| value.downcast_mut())
    }

    /// Inserts an instance of `T` into this registration's type data.
    ///
    /// If another instance of `T` was previously inserted, it is replaced.
    pub fn insert<T: TypeData>(&mut self, data: T) {
        self.data.insert(TypeId::of::<T>(), Box::new(data));
    }

    /// Creates type registration information for `T`.
    pub fn of<T: Reflect>() -> Self {
        let ty = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();
        Self {
            type_id: ty,
            data: HashMap::default(),
            name: type_name,
            short_name: Self::get_short_name(type_name),
        }
    }

    /// Returns the [short name] of the type.
    ///
    /// [short name]: TypeRegistration::get_short_name
    pub fn short_name(&self) -> &str {
        &self.short_name
    }

    /// Returns the name of the type.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Calculates the short name of a type.
    ///
    /// The short name of a type is its full name as returned by
    /// [`std::any::type_name`], but with the prefix of all paths removed. For
    /// example, the short name of `alloc::vec::Vec<core::option::Option<u32>>`
    /// would be `Vec<Option<u32>>`.
    pub fn get_short_name(full_name: &str) -> String {
        let mut short_name = String::new();

        {
            // A typename may be a composition of several other type names (e.g. generic parameters)
            // separated by the characters that we try to find below.
            // Then, each individual typename is shortened to its last path component.
            //
            // Note: Instead of `find`, `split_inclusive` would be nice but it's still unstable...
            let mut remainder = full_name;
            while let Some(index) = remainder.find(&['<', '>', '(', ')', '[', ']', ',', ';'][..]) {
                let (path, new_remainder) = remainder.split_at(index);
                // Push the shortened path in front of the found character
                short_name.push_str(path.rsplit(':').next().unwrap());
                // Push the character that was found
                let character = new_remainder.chars().next().unwrap();
                short_name.push(character);
                // Advance the remainder
                if character == ',' || character == ';' {
                    // A comma or semicolon is always followed by a space
                    short_name.push(' ');
                    remainder = &new_remainder[2..];
                } else {
                    remainder = &new_remainder[1..];
                }
            }

            // The remainder will only be non-empty if there were no matches at all
            if !remainder.is_empty() {
                // Then, the full typename is a path that has to be shortened
                short_name.push_str(remainder.rsplit(':').next().unwrap());
            }
        }

        short_name
    }
}

impl Clone for TypeRegistration {
    fn clone(&self) -> Self {
        let mut data = HashMap::default();
        for (id, type_data) in &self.data {
            data.insert(*id, (*type_data).clone_type_data());
        }

        TypeRegistration {
            data,
            name: self.name,
            short_name: self.short_name.clone(),
            type_id: self.type_id,
        }
    }
}

/// A trait for types generated by the [`#[reflect_trait]`][0] attribute macro.
///
/// [0]: crate::reflect_trait
pub trait TypeData: Downcast + Send + Sync {
    fn clone_type_data(&self) -> Box<dyn TypeData>;
}
impl_downcast!(TypeData);

impl<T: 'static + Send + Sync> TypeData for T
where
    T: Clone,
{
    fn clone_type_data(&self) -> Box<dyn TypeData> {
        Box::new(self.clone())
    }
}

/// Trait used to generate [`TypeData`] for trait reflection.
///
/// This is used by the `#[derive(Reflect)]` macro to generate an implementation
/// of [`TypeData`] to pass to [`TypeRegistration::insert`].
pub trait FromType<T> {
    fn from_type() -> Self;
}

/// A struct used to deserialize reflected instances of a type.
///
/// A `ReflectDeserialize` for type `T` can be obtained via
/// [`FromType::from_type`].
#[derive(Clone)]
pub struct ReflectDeserialize {
    #[allow(clippy::type_complexity)]
    pub func: fn(
        deserializer: &mut dyn erased_serde::Deserializer,
    ) -> Result<Box<dyn Reflect>, erased_serde::Error>,
}

impl ReflectDeserialize {
    /// Deserializes a reflected value.
    ///
    /// The underlying type of the reflected value, and thus the expected
    /// structure of the serialized data, is determined by the type used to
    /// construct this `ReflectDeserialize` value.
    pub fn deserialize<'de, D>(&self, deserializer: D) -> Result<Box<dyn Reflect>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut erased = <dyn erased_serde::Deserializer>::erase(deserializer);
        (self.func)(&mut erased)
            .map_err(<<D as serde::Deserializer<'de>>::Error as serde::de::Error>::custom)
    }
}

impl<T: for<'a> Deserialize<'a> + Reflect> FromType<T> for ReflectDeserialize {
    fn from_type() -> Self {
        ReflectDeserialize {
            func: |deserializer| Ok(Box::new(T::deserialize(deserializer)?)),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::TypeRegistration;

    #[test]
    fn test_get_short_name() {
        assert_eq!(
            TypeRegistration::get_short_name(std::any::type_name::<f64>()),
            "f64"
        );
        assert_eq!(
            TypeRegistration::get_short_name(std::any::type_name::<String>()),
            "String"
        );
        assert_eq!(
            TypeRegistration::get_short_name(std::any::type_name::<(u32, f64)>()),
            "(u32, f64)"
        );
        assert_eq!(
            TypeRegistration::get_short_name(std::any::type_name::<(String, String)>()),
            "(String, String)"
        );
        assert_eq!(
            TypeRegistration::get_short_name(std::any::type_name::<[f64]>()),
            "[f64]"
        );
        assert_eq!(
            TypeRegistration::get_short_name(std::any::type_name::<[String]>()),
            "[String]"
        );
        assert_eq!(
            TypeRegistration::get_short_name(std::any::type_name::<[f64; 16]>()),
            "[f64; 16]"
        );
        assert_eq!(
            TypeRegistration::get_short_name(std::any::type_name::<[String; 16]>()),
            "[String; 16]"
        );
    }

    // TODO: re-enable
    // #[test]
    // fn test_property_type_registration() {
    //     assert_eq!(
    //         TypeRegistration::of::<Option<f64>>().short_name,
    //         "Option<f64>"
    //     );
    //     assert_eq!(
    //         TypeRegistration::of::<HashMap<u32, String>>().short_name,
    //         "HashMap<u32, String>"
    //     );
    //     assert_eq!(
    //         TypeRegistration::of::<Option<HashMap<u32, String>>>().short_name,
    //         "Option<HashMap<u32, String>>"
    //     );
    //     assert_eq!(
    //         TypeRegistration::of::<Option<HashMap<u32, Option<String>>>>().short_name,
    //         "Option<HashMap<u32, Option<String>>>"
    //     );
    //     assert_eq!(
    //         TypeRegistration::of::<Option<HashMap<String, Option<String>>>>().short_name,
    //         "Option<HashMap<String, Option<String>>>"
    //     );
    //     assert_eq!(
    //         TypeRegistration::of::<Option<HashMap<Option<String>, Option<String>>>>().short_name,
    //         "Option<HashMap<Option<String>, Option<String>>>"
    //     );
    //     assert_eq!(
    //         TypeRegistration::of::<Option<HashMap<Option<String>, (String, Option<String>)>>>()
    //             .short_name,
    //         "Option<HashMap<Option<String>, (String, Option<String>)>>"
    //     );
    // }
}
