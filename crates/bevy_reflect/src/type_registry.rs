use crate::{serde::Serializable, FromReflect, Reflect, TypeInfo, TypePath, Typed};
use bevy_ptr::{Ptr, PtrMut};
use bevy_utils::{HashMap, HashSet, TypeIdMap};
use downcast_rs::{impl_downcast, Downcast};
use serde::Deserialize;
use std::{
    any::TypeId,
    fmt::Debug,
    sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

/// A registry of [reflected] types.
///
/// This struct is used as the central store for type information.
/// [Registering] a type will generate a new [`TypeRegistration`] entry in this store
/// using a type's [`GetTypeRegistration`] implementation
/// (which is automatically implemented when using [`#[derive(Reflect)]`](derive@crate::Reflect)).
///
/// See the [crate-level documentation] for more information.
///
/// [reflected]: crate
/// [Registering]: TypeRegistry::register
/// [crate-level documentation]: crate
pub struct TypeRegistry {
    registrations: TypeIdMap<TypeRegistration>,
    short_path_to_id: HashMap<&'static str, TypeId>,
    type_path_to_id: HashMap<&'static str, TypeId>,
    ambiguous_names: HashSet<&'static str>,
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
        self.internal
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .type_path_to_id
            .keys()
            .fmt(f)
    }
}

/// A trait which allows a type to generate its [`TypeRegistration`]
/// for registration into the [`TypeRegistry`].
///
/// This trait is automatically implemented for items using [`#[derive(Reflect)]`](derive@crate::Reflect).
/// The macro also allows [`TypeData`] to be more easily registered.
///
/// See the [crate-level documentation] for more information on type registration.
///
/// [crate-level documentation]: crate
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not provide type registration information",
    note = "consider annotating `{Self}` with `#[derive(Reflect)]`"
)]
pub trait GetTypeRegistration: 'static {
    /// Returns the default [`TypeRegistration`] for this type.
    fn get_type_registration() -> TypeRegistration;
    /// Registers other types needed by this type.
    ///
    /// This method is called by [`TypeRegistry::register`] to register any other required types.
    /// Often, this is done for fields of structs and enum variants to ensure all types are properly registered.
    #[allow(unused_variables)]
    fn register_type_dependencies(registry: &mut TypeRegistry) {}
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
            short_path_to_id: Default::default(),
            type_path_to_id: Default::default(),
            ambiguous_names: Default::default(),
        }
    }

    /// Create a type registry with default registrations for primitive types.
    pub fn new() -> Self {
        let mut registry = Self::empty();
        registry.register::<bool>();
        registry.register::<char>();
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
        registry.register::<String>();
        registry
    }

    /// Attempts to register the type `T` if it has not yet been registered already.
    ///
    /// This will also recursively register any type dependencies as specified by [`GetTypeRegistration::register_type_dependencies`].
    /// When deriving `Reflect`, this will generally be all the fields of the struct or enum variant.
    /// As with any type registration, these type dependencies will not be registered more than once.
    ///
    /// If the registration for type `T` already exists, it will not be registered again and neither will its type dependencies.
    /// To register the type, overwriting any existing registration, use [register](Self::overwrite_registration) instead.
    ///
    /// Additionally, this will add any reflect [type data](TypeData) as specified in the [`Reflect`] derive.
    ///
    /// # Example
    ///
    /// ```
    /// # use std::any::TypeId;
    /// # use bevy_reflect::{Reflect, TypeRegistry, std_traits::ReflectDefault};
    /// #[derive(Reflect, Default)]
    /// #[reflect(Default)]
    /// struct Foo {
    ///   name: Option<String>,
    ///   value: i32
    /// }
    ///
    /// let mut type_registry = TypeRegistry::default();
    ///
    /// type_registry.register::<Foo>();
    ///
    /// // The main type
    /// assert!(type_registry.contains(TypeId::of::<Foo>()));
    ///
    /// // Its type dependencies
    /// assert!(type_registry.contains(TypeId::of::<Option<String>>()));
    /// assert!(type_registry.contains(TypeId::of::<i32>()));
    ///
    /// // Its type data
    /// assert!(type_registry.get_type_data::<ReflectDefault>(TypeId::of::<Foo>()).is_some());
    /// ```
    pub fn register<T>(&mut self)
    where
        T: GetTypeRegistration,
    {
        if self.register_internal(TypeId::of::<T>(), T::get_type_registration) {
            T::register_type_dependencies(self);
        }
    }

    /// Attempts to register the type described by `registration`.
    ///
    /// If the registration for the type already exists, it will not be registered again.
    ///
    /// To forcibly register the type, overwriting any existing registration, use the
    /// [`overwrite_registration`](Self::overwrite_registration) method instead.
    ///
    /// This method will _not_ register type dependencies.
    /// Use [`register`](Self::register) to register a type with its dependencies.
    ///
    /// Returns `true` if the registration was added and `false` if it already exists.
    pub fn add_registration(&mut self, registration: TypeRegistration) -> bool {
        let type_id = registration.type_id();
        self.register_internal(type_id, || registration)
    }

    /// Registers the type described by `registration`.
    ///
    /// If the registration for the type already exists, it will be overwritten.
    ///
    /// To avoid overwriting existing registrations, it's recommended to use the
    /// [`register`](Self::register) or [`add_registration`](Self::add_registration) methods instead.
    ///
    /// This method will _not_ register type dependencies.
    /// Use [`register`](Self::register) to register a type with its dependencies.
    pub fn overwrite_registration(&mut self, registration: TypeRegistration) {
        Self::update_registration_indices(
            &registration,
            &mut self.short_path_to_id,
            &mut self.type_path_to_id,
            &mut self.ambiguous_names,
        );
        self.registrations
            .insert(registration.type_id(), registration);
    }

    /// Internal method to register a type with a given [`TypeId`] and [`TypeRegistration`].
    ///
    /// By using this method, we are able to reduce the number of `TypeId` hashes and lookups needed
    /// to register a type.
    ///
    /// This method is internal to prevent users from accidentally registering a type with a `TypeId`
    /// that does not match the type in the `TypeRegistration`.
    fn register_internal(
        &mut self,
        type_id: TypeId,
        get_registration: impl FnOnce() -> TypeRegistration,
    ) -> bool {
        match self.registrations.entry(type_id) {
            bevy_utils::Entry::Occupied(_) => false,
            bevy_utils::Entry::Vacant(entry) => {
                let registration = get_registration();
                Self::update_registration_indices(
                    &registration,
                    &mut self.short_path_to_id,
                    &mut self.type_path_to_id,
                    &mut self.ambiguous_names,
                );
                entry.insert(registration);
                true
            }
        }
    }

    /// Internal method to register additional lookups for a given [`TypeRegistration`].
    fn update_registration_indices(
        registration: &TypeRegistration,
        short_path_to_id: &mut HashMap<&'static str, TypeId>,
        type_path_to_id: &mut HashMap<&'static str, TypeId>,
        ambiguous_names: &mut HashSet<&'static str>,
    ) {
        let short_name = registration.type_info().type_path_table().short_path();
        if short_path_to_id.contains_key(short_name) || ambiguous_names.contains(short_name) {
            // name is ambiguous. fall back to long names for all ambiguous types
            short_path_to_id.remove(short_name);
            ambiguous_names.insert(short_name);
        } else {
            short_path_to_id.insert(short_name, registration.type_id());
        }
        type_path_to_id.insert(registration.type_info().type_path(), registration.type_id());
    }

    /// Registers the type data `D` for type `T`.
    ///
    /// Most of the time [`TypeRegistry::register`] can be used instead to register a type you derived [`Reflect`] for.
    /// However, in cases where you want to add a piece of type data that was not included in the list of `#[reflect(...)]` type data in the derive,
    /// or where the type is generic and cannot register e.g. [`ReflectSerialize`] unconditionally without knowing the specific type parameters,
    /// this method can be used to insert additional type data.
    ///
    /// # Example
    /// ```
    /// use bevy_reflect::{TypeRegistry, ReflectSerialize, ReflectDeserialize};
    ///
    /// let mut type_registry = TypeRegistry::default();
    /// type_registry.register::<Option<String>>();
    /// type_registry.register_type_data::<Option<String>, ReflectSerialize>();
    /// type_registry.register_type_data::<Option<String>, ReflectDeserialize>();
    /// ```
    pub fn register_type_data<T: Reflect + TypePath, D: TypeData + FromType<T>>(&mut self) {
        let data = self.get_mut(TypeId::of::<T>()).unwrap_or_else(|| {
            panic!(
                "attempted to call `TypeRegistry::register_type_data` for type `{T}` with data `{D}` without registering `{T}` first",
                T = T::type_path(),
                D = std::any::type_name::<D>(),
            )
        });
        data.insert(D::from_type());
    }

    pub fn contains(&self, type_id: TypeId) -> bool {
        self.registrations.contains_key(&type_id)
    }

    /// Returns a reference to the [`TypeRegistration`] of the type with the
    /// given [`TypeId`].
    ///
    /// If the specified type has not been registered, returns `None`.
    ///
    pub fn get(&self, type_id: TypeId) -> Option<&TypeRegistration> {
        self.registrations.get(&type_id)
    }

    /// Returns a mutable reference to the [`TypeRegistration`] of the type with
    /// the given [`TypeId`].
    ///
    /// If the specified type has not been registered, returns `None`.
    ///
    pub fn get_mut(&mut self, type_id: TypeId) -> Option<&mut TypeRegistration> {
        self.registrations.get_mut(&type_id)
    }

    /// Returns a reference to the [`TypeRegistration`] of the type with the
    /// given [type path].
    ///
    /// If no type with the given path has been registered, returns `None`.
    ///
    /// [type path]: TypePath::type_path
    pub fn get_with_type_path(&self, type_path: &str) -> Option<&TypeRegistration> {
        self.type_path_to_id
            .get(type_path)
            .and_then(|id| self.get(*id))
    }

    /// Returns a mutable reference to the [`TypeRegistration`] of the type with
    /// the given [type path].
    ///
    /// If no type with the given type path has been registered, returns `None`.
    ///
    /// [type path]: TypePath::type_path
    pub fn get_with_type_path_mut(&mut self, type_path: &str) -> Option<&mut TypeRegistration> {
        self.type_path_to_id
            .get(type_path)
            .cloned()
            .and_then(move |id| self.get_mut(id))
    }

    /// Returns a reference to the [`TypeRegistration`] of the type with
    /// the given [short type path].
    ///
    /// If the short type path is ambiguous, or if no type with the given path
    /// has been registered, returns `None`.
    ///
    /// [short type path]: TypePath::short_type_path
    pub fn get_with_short_type_path(&self, short_type_path: &str) -> Option<&TypeRegistration> {
        self.short_path_to_id
            .get(short_type_path)
            .and_then(|id| self.registrations.get(id))
    }

    /// Returns a mutable reference to the [`TypeRegistration`] of the type with
    /// the given [short type path].
    ///
    /// If the short type path is ambiguous, or if no type with the given path
    /// has been registered, returns `None`.
    ///
    /// [short type path]: TypePath::short_type_path
    pub fn get_with_short_type_path_mut(
        &mut self,
        short_type_path: &str,
    ) -> Option<&mut TypeRegistration> {
        self.short_path_to_id
            .get(short_type_path)
            .and_then(|id| self.registrations.get_mut(id))
    }

    /// Returns `true` if the given [short type path] is ambiguous, that is, it matches multiple registered types.
    ///
    /// # Example
    /// ```
    /// # use bevy_reflect::TypeRegistry;
    /// # mod foo {
    /// #     use bevy_reflect::Reflect;
    /// #     #[derive(Reflect)]
    /// #     pub struct MyType;
    /// # }
    /// # mod bar {
    /// #     use bevy_reflect::Reflect;
    /// #     #[derive(Reflect)]
    /// #     pub struct MyType;
    /// # }
    /// let mut type_registry = TypeRegistry::default();
    /// type_registry.register::<foo::MyType>();
    /// type_registry.register::<bar::MyType>();
    /// assert_eq!(type_registry.is_ambiguous("MyType"), true);
    /// ```
    ///
    /// [short type path]: TypePath::short_type_path
    pub fn is_ambiguous(&self, short_type_path: &str) -> bool {
        self.ambiguous_names.contains(short_type_path)
    }

    /// Returns a reference to the [`TypeData`] of type `T` associated with the given [`TypeId`].
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

    /// Returns a mutable reference to the [`TypeData`] of type `T` associated with the given [`TypeId`].
    ///
    /// If the specified type has not been registered, or if `T` is not present
    /// in its type registration, returns `None`.
    pub fn get_type_data_mut<T: TypeData>(&mut self, type_id: TypeId) -> Option<&mut T> {
        self.get_mut(type_id)
            .and_then(|registration| registration.data_mut::<T>())
    }

    /// Returns the [`TypeInfo`] associated with the given [`TypeId`].
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

    /// Checks to see if the [`TypeData`] of type `T` is associated with each registered type,
    /// returning a ([`TypeRegistration`], [`TypeData`]) iterator for all entries where data of that type was found.
    pub fn iter_with_data<T: TypeData>(&self) -> impl Iterator<Item = (&TypeRegistration, &T)> {
        self.registrations.values().filter_map(|item| {
            let type_data = item.data::<T>();
            type_data.map(|data| (item, data))
        })
    }
}

impl TypeRegistryArc {
    /// Takes a read lock on the underlying [`TypeRegistry`].
    pub fn read(&self) -> RwLockReadGuard<'_, TypeRegistry> {
        self.internal.read().unwrap_or_else(PoisonError::into_inner)
    }

    /// Takes a write lock on the underlying [`TypeRegistry`].
    pub fn write(&self) -> RwLockWriteGuard<'_, TypeRegistry> {
        self.internal
            .write()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// Runtime storage for type metadata, registered into the [`TypeRegistry`].
///
/// An instance of `TypeRegistration` can be created using the [`TypeRegistration::of`] method,
/// but is more often automatically generated using [`#[derive(Reflect)]`](derive@crate::Reflect) which itself generates
/// an implementation of the [`GetTypeRegistration`] trait.
///
/// Along with the type's [`TypeInfo`],
/// this struct also contains a type's registered [`TypeData`].
///
/// See the [crate-level documentation] for more information on type registration.
///
/// # Example
///
/// ```
/// # use bevy_reflect::{TypeRegistration, std_traits::ReflectDefault, FromType};
/// let mut registration = TypeRegistration::of::<Option<String>>();
///
/// assert_eq!("core::option::Option<alloc::string::String>", registration.type_info().type_path());
/// assert_eq!("Option<String>", registration.type_info().type_path_table().short_path());
///
/// registration.insert::<ReflectDefault>(FromType::<Option<String>>::from_type());
/// assert!(registration.data::<ReflectDefault>().is_some())
/// ```
///
/// [crate-level documentation]: crate
pub struct TypeRegistration {
    data: TypeIdMap<Box<dyn TypeData>>,
    type_info: &'static TypeInfo,
}

impl Debug for TypeRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypeRegistration")
            .field("type_info", &self.type_info)
            .finish()
    }
}

impl TypeRegistration {
    /// Returns the [`TypeId`] of the type.
    ///
    #[inline]
    pub fn type_id(&self) -> TypeId {
        self.type_info.type_id()
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
    pub fn of<T: Reflect + Typed + TypePath>() -> Self {
        Self {
            data: Default::default(),
            type_info: T::type_info(),
        }
    }
}

impl Clone for TypeRegistration {
    fn clone(&self) -> Self {
        let mut data = TypeIdMap::default();
        for (id, type_data) in &self.data {
            data.insert(*id, (*type_data).clone_type_data());
        }

        TypeRegistration {
            data,
            type_info: self.type_info,
        }
    }
}

/// A trait used to type-erase type metadata.
///
/// Type data can be registered to the [`TypeRegistry`] and stored on a type's [`TypeRegistration`].
///
/// While type data is often generated using the [`#[reflect_trait]`](crate::reflect_trait) macro,
/// almost any type that implements [`Clone`] can be considered "type data".
/// This is because it has a blanket implementation over all `T` where `T: Clone + Send + Sync + 'static`.
///
/// See the [crate-level documentation] for more information on type data and type registration.
///
/// [crate-level documentation]: crate
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

/// A struct used to serialize reflected instances of a type.
///
/// A `ReflectSerialize` for type `T` can be obtained via
/// [`FromType::from_type`].
#[derive(Clone)]
pub struct ReflectSerialize {
    get_serializable: for<'a> fn(value: &'a dyn Reflect) -> Serializable,
}

impl<T: TypePath + FromReflect + erased_serde::Serialize> FromType<T> for ReflectSerialize {
    fn from_type() -> Self {
        ReflectSerialize {
            get_serializable: |value| {
                value
                    .downcast_ref::<T>()
                    .map(|value| Serializable::Borrowed(value))
                    .or_else(|| T::from_reflect(value).map(|value| Serializable::Owned(Box::new(value))))
                    .unwrap_or_else(|| {
                        panic!(
                            "FromReflect::from_reflect failed when called on type `{}` with this value: {value:?}",
                            T::type_path(),
                        );
                    })
            },
        }
    }
}

impl ReflectSerialize {
    /// Turn the value into a serializable representation
    pub fn get_serializable<'a>(&self, value: &'a dyn Reflect) -> Serializable<'a> {
        (self.get_serializable)(value)
    }
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

/// [`Reflect`] values are commonly used in situations where the actual types of values
/// are not known at runtime. In such situations you might have access to a `*const ()` pointer
/// that you know implements [`Reflect`], but have no way of turning it into a `&dyn Reflect`.
///
/// This is where [`ReflectFromPtr`] comes in, when creating a [`ReflectFromPtr`] for a given type `T: Reflect`.
/// Internally, this saves a concrete function `*const T -> const dyn Reflect` which lets you create a trait object of [`Reflect`]
/// from a pointer.
///
/// # Example
/// ```
/// use bevy_reflect::{TypeRegistry, Reflect, ReflectFromPtr};
/// use bevy_ptr::Ptr;
/// use std::ptr::NonNull;
///
/// #[derive(Reflect)]
/// struct Reflected(String);
///
/// let mut type_registry = TypeRegistry::default();
/// type_registry.register::<Reflected>();
///
/// let mut value = Reflected("Hello world!".to_string());
/// let value = Ptr::from(&value);
///
/// let reflect_data = type_registry.get(std::any::TypeId::of::<Reflected>()).unwrap();
/// let reflect_from_ptr = reflect_data.data::<ReflectFromPtr>().unwrap();
/// // SAFE: `value` is of type `Reflected`, which the `ReflectFromPtr` was created for
/// let value = unsafe { reflect_from_ptr.as_reflect(value) };
///
/// assert_eq!(value.downcast_ref::<Reflected>().unwrap().0, "Hello world!");
/// ```
#[derive(Clone)]
pub struct ReflectFromPtr {
    type_id: TypeId,
    from_ptr: unsafe fn(Ptr) -> &dyn Reflect,
    from_ptr_mut: unsafe fn(PtrMut) -> &mut dyn Reflect,
}

#[allow(unsafe_code)]
impl ReflectFromPtr {
    /// Returns the [`TypeId`] that the [`ReflectFromPtr`] was constructed for.
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Convert `Ptr` into `&dyn Reflect`.
    ///
    /// # Safety
    ///
    /// `val` must be a pointer to value of the type that the [`ReflectFromPtr`] was constructed for.
    /// This can be verified by checking that the type id returned by [`ReflectFromPtr::type_id`] is the expected one.
    pub unsafe fn as_reflect<'a>(&self, val: Ptr<'a>) -> &'a dyn Reflect {
        // SAFETY: contract uphold by the caller.
        unsafe { (self.from_ptr)(val) }
    }

    /// Convert `PtrMut` into `&mut dyn Reflect`.
    ///
    /// # Safety
    ///
    /// `val` must be a pointer to a value of the type that the [`ReflectFromPtr`] was constructed for
    /// This can be verified by checking that the type id returned by [`ReflectFromPtr::type_id`] is the expected one.
    pub unsafe fn as_reflect_mut<'a>(&self, val: PtrMut<'a>) -> &'a mut dyn Reflect {
        // SAFETY: contract uphold by the caller.
        unsafe { (self.from_ptr_mut)(val) }
    }
    /// Get a function pointer to turn a `Ptr` into `&dyn Reflect` for
    /// the type this [`ReflectFromPtr`] was constructed for.
    ///
    /// # Safety
    ///
    /// When calling the unsafe function returned by this method you must ensure that:
    /// - The input `Ptr` points to the `Reflect` type this `ReflectFromPtr`
    ///   was constructed for.
    pub fn from_ptr(&self) -> unsafe fn(Ptr) -> &dyn Reflect {
        self.from_ptr
    }
    /// Get a function pointer to turn a `PtrMut` into `&mut dyn Reflect` for
    /// the type this [`ReflectFromPtr`] was constructed for.
    ///
    /// # Safety
    ///
    /// When calling the unsafe function returned by this method you must ensure that:
    /// - The input `PtrMut` points to the `Reflect` type this `ReflectFromPtr`
    ///   was constructed for.
    pub fn from_ptr_mut(&self) -> unsafe fn(PtrMut) -> &mut dyn Reflect {
        self.from_ptr_mut
    }
}

#[allow(unsafe_code)]
impl<T: Reflect> FromType<T> for ReflectFromPtr {
    fn from_type() -> Self {
        ReflectFromPtr {
            type_id: TypeId::of::<T>(),
            from_ptr: |ptr| {
                // SAFETY: `from_ptr_mut` is either called in `ReflectFromPtr::as_reflect`
                // or returned by `ReflectFromPtr::from_ptr`, both lay out the invariants
                // required by `deref`
                unsafe { ptr.deref::<T>() as &dyn Reflect }
            },
            from_ptr_mut: |ptr| {
                // SAFETY: same as above, but for `as_reflect_mut`, `from_ptr_mut` and `deref_mut`.
                unsafe { ptr.deref_mut::<T>() as &mut dyn Reflect }
            },
        }
    }
}

#[cfg(test)]
#[allow(unsafe_code)]
mod test {
    use crate::{GetTypeRegistration, ReflectFromPtr};
    use bevy_ptr::{Ptr, PtrMut};

    use crate as bevy_reflect;
    use crate::Reflect;

    #[test]
    fn test_reflect_from_ptr() {
        #[derive(Reflect)]
        struct Foo {
            a: f32,
        }

        let foo_registration = <Foo as GetTypeRegistration>::get_type_registration();
        let reflect_from_ptr = foo_registration.data::<ReflectFromPtr>().unwrap();

        // not required in this situation because we no nobody messed with the TypeRegistry,
        // but in the general case somebody could have replaced the ReflectFromPtr with an
        // instance for another type, so then we'd need to check that the type is the expected one
        assert_eq!(reflect_from_ptr.type_id(), std::any::TypeId::of::<Foo>());

        let mut value = Foo { a: 1.0 };
        {
            let value = PtrMut::from(&mut value);
            // SAFETY: reflect_from_ptr was constructed for the correct type
            let dyn_reflect = unsafe { reflect_from_ptr.as_reflect_mut(value) };
            match dyn_reflect.reflect_mut() {
                bevy_reflect::ReflectMut::Struct(strukt) => {
                    strukt.field_mut("a").unwrap().apply(&2.0f32);
                }
                _ => panic!("invalid reflection"),
            }
        }

        {
            // SAFETY: reflect_from_ptr was constructed for the correct type
            let dyn_reflect = unsafe { reflect_from_ptr.as_reflect(Ptr::from(&value)) };
            match dyn_reflect.reflect_ref() {
                bevy_reflect::ReflectRef::Struct(strukt) => {
                    let a = strukt.field("a").unwrap().downcast_ref::<f32>().unwrap();
                    assert_eq!(*a, 2.0);
                }
                _ => panic!("invalid reflection"),
            }
        }
    }
}
