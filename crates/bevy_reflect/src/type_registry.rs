use crate::{Reflect, TypeInfo, Typed};
use bevy_utils::{HashMap, HashSet};
use downcast_rs::{impl_downcast, Downcast};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use serde::Deserialize;
use std::{any::TypeId, fmt::Debug, mem::MaybeUninit, ptr::NonNull, sync::Arc};

/// A registry of reflected types.
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

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeRegistry {
    /// Create a type registry with *no* registered types.
    pub fn empty() -> Self {
        Self {
            registrations: Default::default(),
            short_name_to_id: Default::default(),
            full_name_to_id: Default::default(),
            ambiguous_names: Default::default(),
        }
    }

    /// Create a type registry with default registrations for primitive types.
    pub fn new() -> Self {
        let mut registry = Self::empty();
        registry.register::<bool>();
        registry.register::<u8>();
        registry.register::<u16>();
        registry.register::<u32>();
        registry.register::<u64>();
        registry.register::<u128>();
        registry.register::<usize>();
        registry.register::<i8>();
        registry.register::<i16>();
        registry.register::<i32>();
        registry.register::<i64>();
        registry.register::<i128>();
        registry.register::<isize>();
        registry.register::<f32>();
        registry.register::<f64>();
        registry
    }

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
                .insert(short_name, registration.type_id());
        }
        self.full_name_to_id
            .insert(registration.type_name().to_string(), registration.type_id());
        self.registrations
            .insert(registration.type_id(), registration);
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

    /// Returns a reference to the [`TypeRegistration`] of the type with
    /// the given short name.
    ///
    /// If the short name is ambiguous, or if no type with the given short name
    /// has been registered, returns `None`.
    pub fn get_with_short_name(&self, short_type_name: &str) -> Option<&TypeRegistration> {
        self.short_name_to_id
            .get(short_type_name)
            .and_then(|id| self.registrations.get(id))
    }

    /// Returns a mutable reference to the [`TypeRegistration`] of the type with
    /// the given short name.
    ///
    /// If the short name is ambiguous, or if no type with the given short name
    /// has been registered, returns `None`.
    pub fn get_with_short_name_mut(
        &mut self,
        short_type_name: &str,
    ) -> Option<&mut TypeRegistration> {
        self.short_name_to_id
            .get(short_type_name)
            .and_then(|id| self.registrations.get_mut(id))
    }

    /// Returns a reference to the [`TypeData`] of type `T` associated with the given `TypeId`.
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

    /// Returns a mutable reference to the [`TypeData`] of type `T` associated with the given `TypeId`.
    ///
    /// If the specified type has not been registered, or if `T` is not present
    /// in its type registration, returns `None`.
    pub fn get_type_data_mut<T: TypeData>(&mut self, type_id: TypeId) -> Option<&mut T> {
        self.get_mut(type_id)
            .and_then(|registration| registration.data_mut::<T>())
    }

    /// Returns the [`TypeInfo`] associated with the given `TypeId`.
    ///
    /// If the specified type has not been registered, returns `None`.
    pub fn get_type_info(&self, type_id: TypeId) -> Option<&'static TypeInfo> {
        self.get(type_id)
            .map(|registration| registration.type_info())
    }

    /// Returns an iterator over the [`TypeRegistration`]s of the registered
    /// types.
    pub fn iter(&self) -> impl Iterator<Item = &TypeRegistration> {
        self.registrations.values()
    }

    /// Returns a mutable iterator over the [`TypeRegistration`]s of the registered
    /// types.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut TypeRegistration> {
        self.registrations.values_mut()
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

#[doc(hidden)]
pub struct ErasedNonNull {
    storage: MaybeUninit<[NonNull<()>; 2]>,
    ty: TypeId,
}

impl ErasedNonNull {
    /// Creates a new type-erased [`ErasedNonNull`] instance for the given value.
    pub fn new<T: ?Sized + 'static>(val: &T) -> Self {
        let size = core::mem::size_of::<*const T>();
        assert!(size <= core::mem::size_of::<[NonNull<()>; 2]>());
        let val = val as *const T;
        let mut storage = MaybeUninit::uninit();
        // SAFE: Size of reference pointer guaranteed to fit within storage
        unsafe {
            core::ptr::copy(
                &val as *const *const T as *const u8,
                storage.as_mut_ptr() as *mut u8,
                size,
            )
        };
        Self {
            storage,
            ty: TypeId::of::<T>(),
        }
    }

    /// Converts this type-erased value into a typed one.
    ///
    /// # Panics
    ///
    /// Panics if the type `T` does not match the underlying type.
    ///
    /// # Safety
    ///
    /// Since the type `T` _must_ match the underlying type (or else panics), this method is
    /// guaranteed to be safe.
    pub unsafe fn into_ref<'a, T: ?Sized + 'static>(self) -> &'a T {
        assert_eq!(self.ty, TypeId::of::<T>());
        let size = core::mem::size_of::<*const T>();
        let mut r: MaybeUninit<*const T> = MaybeUninit::uninit();
        core::ptr::copy(
            self.storage.as_ptr() as *mut u8,
            r.as_mut_ptr() as *mut u8,
            size,
        );
        &*r.assume_init()
    }
}

#[macro_export]
macro_rules! maybe_trait_cast {
    ($this_type:ty, $trait_type:path) => {{
        {
            trait NotTrait {
                const CAST_FN: Option<
                    for<'a> fn(&'a $this_type) -> &'a (dyn $trait_type + 'static),
                > = None;
            }
            impl<T> NotTrait for T {}
            struct IsImplemented<T>(core::marker::PhantomData<T>);

            impl<T: $trait_type + 'static> IsImplemented<T> {
                #[allow(dead_code)]
                const CAST_FN: Option<for<'a> fn(&'a T) -> &'a (dyn $trait_type + 'static)> =
                    Some(|a| a);
            }
            if IsImplemented::<$this_type>::CAST_FN.is_some() {
                let f: fn(&dyn $crate::Reflect) -> $crate::ErasedNonNull =
                    |val: &dyn $crate::Reflect| {
                        let cast_fn = IsImplemented::<$this_type>::CAST_FN.unwrap();
                        let static_val: &$this_type = val.downcast_ref::<$this_type>().unwrap();
                        let trait_val: &dyn $trait_type = (cast_fn)(static_val);
                        $crate::ErasedNonNull::new(trait_val)
                    };
                Some(f)
            } else {
                None
            }
        }
    }};
}

#[macro_export]
macro_rules! register_type {
    ($type_registry:ident, $this_type:ty, $($trait_type:path),* $(,)?) => {{
        let type_registration = match $type_registry.get_mut(::std::any::TypeId::of::<$this_type>()) {
            Some(registration) => registration,
            None => {
                $type_registry.register::<$this_type>();
                $type_registry.get_mut(::std::any::TypeId::of::<$this_type>()).unwrap()
            }
        };

        $(
            if let Some(cast_fn) = $crate::maybe_trait_cast!($this_type, $trait_type) {{
                type_registration.register_trait_cast::<dyn $trait_type>(cast_fn);
            }}
        )*
    }};
}

/// A record of data about a type.
///
/// This contains the [`TypeInfo`] of the type, as well as its [short name].
///
/// For each trait specified by the [`#[reflect(_)]`][0] attribute of
/// [`#[derive(Reflect)]`][1] on the registered type, this record also contains
/// a [`TypeData`] which can be used to downcast [`Reflect`] trait objects of
/// this type to trait objects of the relevant trait.
///
/// [short name]: TypeRegistration::get_short_name
/// [`TypeInfo`]: crate::TypeInfo
/// [0]: crate::Reflect
/// [1]: crate::Reflect
pub struct TypeRegistration {
    short_name: String,
    data: HashMap<TypeId, Box<dyn TypeData>>,
    type_info: &'static TypeInfo,
    trait_casts: HashMap<TypeId, fn(&dyn Reflect) -> ErasedNonNull>,
}

impl TypeRegistration {
    /// Returns the [`TypeId`] of the type.
    ///
    /// [`TypeId`]: std::any::TypeId
    #[inline]
    pub fn type_id(&self) -> TypeId {
        self.type_info.type_id()
    }

    #[doc(hidden)]
    pub fn register_trait_cast<T: ?Sized + 'static>(
        &mut self,
        f: fn(&dyn Reflect) -> ErasedNonNull,
    ) {
        self.trait_casts.insert(TypeId::of::<T>(), f);
    }

    pub fn has_trait_cast<T: ?Sized + 'static>(&self) -> bool {
        self.trait_casts.contains_key(&TypeId::of::<T>())
    }

    pub fn trait_cast<'a, T: ?Sized + 'static>(&self, val: &'a dyn Reflect) -> Option<&'a T> {
        if let Some(cast) = self.trait_casts.get(&TypeId::of::<T>()) {
            let raw = cast(val);
            // SAFE: Registered trait and type matches the call site
            Some(unsafe { raw.into_ref() })
        } else {
            None
        }
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

    /// Returns a reference to the registration's [`TypeInfo`]
    pub fn type_info(&self) -> &'static TypeInfo {
        self.type_info
    }

    /// Inserts an instance of `T` into this registration's type data.
    ///
    /// If another instance of `T` was previously inserted, it is replaced.
    pub fn insert<T: TypeData>(&mut self, data: T) {
        self.data.insert(TypeId::of::<T>(), Box::new(data));
    }

    /// Creates type registration information for `T`.
    pub fn of<T: Reflect + Typed>() -> Self {
        let type_name = std::any::type_name::<T>();
        Self {
            data: HashMap::default(),
            trait_casts: HashMap::default(),
            short_name: Self::get_short_name(type_name),
            type_info: T::type_info(),
        }
    }

    /// Returns the [short name] of the type.
    ///
    /// [short name]: TypeRegistration::get_short_name
    pub fn short_name(&self) -> &str {
        &self.short_name
    }

    /// Returns the [name] of the type.
    ///
    /// [name]: std::any::type_name
    pub fn type_name(&self) -> &'static str {
        self.type_info.type_name()
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
            trait_casts: self.trait_casts.clone(),
            short_name: self.short_name.clone(),
            type_info: self.type_info,
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
mod tests {
    use super::*;
    use std::any::Any;

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

    #[test]
    fn test_property_type_registration() {
        assert_eq!(
            TypeRegistration::of::<Option<f64>>().short_name,
            "Option<f64>"
        );
        assert_eq!(
            TypeRegistration::of::<HashMap<u32, String>>().short_name,
            "HashMap<u32, String>"
        );
        assert_eq!(
            TypeRegistration::of::<Option<HashMap<u32, String>>>().short_name,
            "Option<HashMap<u32, String>>"
        );
        assert_eq!(
            TypeRegistration::of::<Option<HashMap<u32, Option<String>>>>().short_name,
            "Option<HashMap<u32, Option<String>>>"
        );
        assert_eq!(
            TypeRegistration::of::<Option<HashMap<String, Option<String>>>>().short_name,
            "Option<HashMap<String, Option<String>>>"
        );
        assert_eq!(
            TypeRegistration::of::<Option<HashMap<Option<String>, Option<String>>>>().short_name,
            "Option<HashMap<Option<String>, Option<String>>>"
        );
        assert_eq!(
            TypeRegistration::of::<Option<HashMap<Option<String>, (String, Option<String>)>>>()
                .short_name,
            "Option<HashMap<Option<String>, (String, Option<String>)>>"
        );
    }

    trait Test {}

    impl Test for HashMap<u32, u32> {}

    trait TestNot {}

    // the user should specify all traits in a top-level crate.
    // all registration should be done through macros in a top-level crate.
    macro_rules! register_type_custom {
        ($type_registry:ident, $this_type:ty,$($trait_type:path),*) => {
            register_type!(
                $type_registry,
                $this_type,
                erased_serde::Serialize,
                $($trait_type,)*
            )
        };
    }
    #[test]
    fn test_trait_cast() {
        let mut type_registry = TypeRegistry::default();
        register_type_custom!(type_registry, HashMap<u32, u32>, Test, TestNot);
        register_type_custom!(type_registry, u32,);
        let val = HashMap::<u32, u32>::default();
        let ty = type_registry
            .get(TypeId::of::<HashMap<u32, u32>>())
            .unwrap();
        assert!(ty.trait_cast::<dyn erased_serde::Serialize>(&val).is_some());
        assert!(ty.trait_cast::<dyn Test>(&val).is_some());
        assert!(ty.trait_cast::<dyn TestNot>(&val).is_none());

        let ty = type_registry.get(TypeId::of::<u32>()).unwrap();
        assert!(ty
            .trait_cast::<dyn erased_serde::Serialize>(&3u32)
            .is_some());
        assert!(ty.trait_cast::<dyn Test>(&3u32).is_none());
    }

    #[test]
    fn erased_non_null_should_work() {
        // &str -> &str
        let input = "Hello, World!";
        let erased = ErasedNonNull::new(input);
        let output = unsafe { erased.into_ref::<str>() };
        assert_eq!(input, output);
        assert_eq!(input.type_id(), output.type_id());

        // &dyn Test -> &dyn Test
        let input: &dyn Test = &HashMap::<u32, u32>::default();
        let erased = ErasedNonNull::new(input);
        let output = unsafe { erased.into_ref::<dyn Test>() };
        assert_eq!(input.type_id(), output.type_id());

        // &() -> &()
        let input: () = ();
        let erased = ErasedNonNull::new(&input);
        let output = unsafe { erased.into_ref::<()>() };
        assert_eq!(input.type_id(), output.type_id());
    }

    #[test]
    #[should_panic]
    fn erased_non_null_should_panic_for_wrong_type() {
        let input = "Hello, World!";
        let erased = ErasedNonNull::new(input);
        let _ = unsafe { erased.into_ref::<i32>() };
    }
}
