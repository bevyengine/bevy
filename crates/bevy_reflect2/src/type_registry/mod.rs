mod type_data;

pub use type_data::*;

use crate::{Type, Typed};
use alloc::{boxed::Box, string::String};
use bevy_platform::{
    collections::{HashMap, HashSet},
    sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard},
};
use bevy_utils::TypeIdHashMap;
use core::{
    any::TypeId,
    fmt::Debug,
    ops::{Deref, DerefMut},
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
    registrations: TypeIdHashMap<TypeRegistration>,
    short_path_to_id: HashMap<&'static str, TypeId>,
    type_path_to_id: HashMap<&'static str, TypeId>,
    ambiguous_names: HashSet<&'static str>,
}

/// A synchronized wrapper around a [`TypeRegistry`].
#[derive(Clone, Default)]
pub struct TypeRegistryArc {
    /// The wrapped [`TypeRegistry`].
    pub internal: Arc<RwLock<TypeRegistry>>,
}

impl Debug for TypeRegistryArc {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
    message = "`{Self}` does not implement `GetTypeRegistration` so cannot provide type registration information",
    note = "consider annotating `{Self}` with `#[derive(Reflect)]`"
)]
pub trait GetTypeRegistration: Typed {
    /// Returns the default [`TypeRegistration`] for this type.
    fn get_type_registration() -> TypeRegistration;
    /// Registers other types needed by this type.
    ///
    /// This method is called by [`TypeRegistry::register`] to register any other required types.
    /// Often, this is done for fields of structs and enum variants to ensure all types are properly registered.
    fn register_type_dependencies(_registry: &mut TypeRegistry) {}
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
        registry.register::<()>();
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

    /// Register all non-generic types annotated with `#[derive(Reflect)]`.
    ///
    /// Calling this method is equivalent to calling [`register`](Self::register) on all types without generic parameters
    /// that derive [`Reflect`](derive@crate::Reflect).
    ///
    /// With the `auto_register_inventory` feature, this method is supported on Linux, macOS, Windows, iOS, Android,
    /// and Web via the `inventory` crate. On other platforms, use the `auto_register_static` feature along with
    /// [`load_type_registrations`](crate::load_type_registrations).
    ///
    /// Types can opt out of automatic registration using `#[reflect(no_auto_register)]`.
    ///
    /// # Example
    ///
    /// ```
    /// # use core::any::TypeId;
    /// # use bevy_reflect2::{Reflect, TypeRegistry};
    /// #[derive(Reflect)]
    /// struct Foo {
    ///   value: i32
    /// }
    ///
    /// let mut type_registry = TypeRegistry::empty();
    /// type_registry.register_derived_types();
    ///
    /// assert!(type_registry.contains(TypeId::of::<Foo>()));
    /// ```
    #[cfg(feature = "auto_register")]
    pub fn register_derived_types(&mut self) {
        crate::auto_register::register_types(self);
    }

    /// Attempts to register the type `T` if it has not yet been registered already.
    ///
    /// This will also recursively register any type dependencies as specified by [`GetTypeRegistration::register_type_dependencies`].
    /// When deriving `Reflect`, this will generally be all the fields of the struct or enum variant.
    /// As with any type registration, these type dependencies will not be registered more than once.
    ///
    /// If the registration for type `T` already exists, it will not be registered again and neither will its type dependencies.
    /// To register the type, overwriting any existing registration, use [`overwrite_registration`](Self::overwrite_registration) instead.
    ///
    /// Additionally, this will add any reflect [type data](TypeData) as specified in the [`Reflect`](derive@crate::Reflect) derive.
    ///
    /// # Example
    ///
    /// ```
    /// # use core::any::TypeId;
    /// # use bevy_reflect2::{CreateTypeData, Reflect, TypeRegistry};
    /// #[derive(Clone)]
    /// struct ReflectMarker;
    ///
    /// impl<T> CreateTypeData<T> for ReflectMarker {
    ///   fn create_type_data(_input: ()) -> Self {
    ///     Self
    ///   }
    /// }
    ///
    /// #[derive(Reflect)]
    /// #[reflect(Marker)]
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
    /// assert!(type_registry.get_type_data::<ReflectMarker>(TypeId::of::<Foo>()).is_some());
    /// ```
    pub fn register<T>(&mut self)
    where
        T: GetTypeRegistration,
    {
        if self.register_internal(T::TYPE.type_id(), T::get_type_registration) {
            T::register_type_dependencies(self);
        }
    }

    /// Attempts to register the referenced type `T` if it has not yet been registered.
    ///
    /// See [`register`] for more details.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect2::{Reflect, TypeRegistry};
    /// # use core::any::TypeId;
    /// #
    /// # let mut type_registry = TypeRegistry::default();
    /// #
    /// #[derive(Reflect)]
    /// struct Foo {
    ///   bar: Bar,
    /// }
    ///
    /// #[derive(Reflect)]
    /// struct Bar;
    ///
    /// let foo = Foo { bar: Bar };
    ///
    /// // Equivalent to `type_registry.register::<Foo>()`
    /// type_registry.register_by_val(&foo);
    ///
    /// assert!(type_registry.contains(TypeId::of::<Foo>()));
    /// assert!(type_registry.contains(TypeId::of::<Bar>()));
    /// ```
    ///
    /// [`register`]: Self::register
    pub fn register_by_val<T>(&mut self, _: &T)
    where
        T: GetTypeRegistration,
    {
        self.register::<T>();
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
        if self.registrations.contains_key(&type_id) {
            return false;
        }
        let registration = get_registration();
        Self::update_registration_indices(
            &registration,
            &mut self.short_path_to_id,
            &mut self.type_path_to_id,
            &mut self.ambiguous_names,
        );
        self.registrations.insert(type_id, registration);
        true
    }

    /// Internal method to register additional lookups for a given [`TypeRegistration`].
    fn update_registration_indices(
        registration: &TypeRegistration,
        short_path_to_id: &mut HashMap<&'static str, TypeId>,
        type_path_to_id: &mut HashMap<&'static str, TypeId>,
        ambiguous_names: &mut HashSet<&'static str>,
    ) {
        let short_name = registration.ty().type_path().short();
        if short_path_to_id.contains_key(short_name) || ambiguous_names.contains(short_name) {
            // name is ambiguous. fall back to long names for all ambiguous types
            short_path_to_id.remove(short_name);
            ambiguous_names.insert(short_name);
        } else {
            short_path_to_id.insert(short_name, registration.type_id());
        }
        type_path_to_id.insert(registration.ty().type_path().full(), registration.type_id());
    }

    /// Registers the type data `D` for type `T`.
    ///
    /// Most of the time [`TypeRegistry::register`] can be used instead to register a type you derived [`Reflect`](derive@crate::Reflect) for.
    /// However, in cases where you want to add a piece of type data that was not included in the list of `#[reflect(...)]` type data in the derive,
    /// or where the type is generic and cannot register the type data unconditionally without knowing the specific type parameters,
    /// this method can be used to insert additional type data.
    ///
    /// # Example
    /// ```
    /// # use core::any::TypeId;
    /// use bevy_reflect2::{CreateTypeData, TypeRegistry};
    ///
    /// #[derive(Clone)]
    /// struct ReflectMarker;
    ///
    /// impl<T> CreateTypeData<T> for ReflectMarker {
    ///   fn create_type_data(_input: ()) -> Self {
    ///     Self
    ///   }
    /// }
    ///
    /// let mut type_registry = TypeRegistry::default();
    /// type_registry.register::<Option<String>>();
    /// type_registry.register_type_data::<Option<String>, ReflectMarker>();
    ///
    /// assert!(type_registry.get_type_data::<ReflectMarker>(TypeId::of::<Option<String>>()).is_some());
    /// ```
    pub fn register_type_data<T: Typed, D: CreateTypeData<T>>(&mut self) {
        let data = self.get_mut(T::TYPE.type_id()).unwrap_or_else(|| {
            panic!(
                "attempted to call `TypeRegistry::register_type_data` for type `{T}` with data `{D}` without registering `{T}` first",
                T = T::TYPE.type_path().short(),
                D = core::any::type_name::<D>(),
            )
        });
        data.insert(D::create_type_data(()));
    }

    /// Registers the type data `D` with parameter `P` for type `T`.
    ///
    /// Most of the time [`TypeRegistry::register`] can be used instead to register a type you derived [`Reflect`](derive@crate::Reflect) for.
    /// However, in cases where you want to add a piece of type data that was not included in the list of `#[reflect(...)]` type data in the derive,
    /// or where the type is generic and cannot register the type data unconditionally without knowing the specific type parameters,
    /// this method can be used to insert additional type data.
    ///
    /// If no parameters are needed for the type data or the type data does not accept parameters,
    /// then [`TypeRegistry::register_type_data`] may be used instead.
    pub fn register_type_data_with<T: Typed, D: CreateTypeData<T, P>, P>(&mut self, params: P) {
        let data = self.get_mut(T::TYPE.type_id()).unwrap_or_else(|| {
            panic!(
                "attempted to call `TypeRegistry::register_type_data_with` for type `{T}` with data `{D}` and params `{P}` without registering `{T}` first",
                T = T::TYPE.type_path().short(),
                D = ::core::any::type_name::<D>(),
                P = ::core::any::type_name::<P>(),
            )
        });
        data.insert(D::create_type_data(params));
    }

    /// Whether the type with given [`TypeId`] has been registered in this registry.
    pub fn contains(&self, type_id: TypeId) -> bool {
        self.registrations.contains_key(&type_id)
    }

    /// Returns a reference to the [`TypeRegistration`] of the type with the
    /// given [`TypeId`].
    ///
    /// If the specified type has not been registered, returns `None`.
    #[inline]
    pub fn get(&self, type_id: TypeId) -> Option<&TypeRegistration> {
        self.registrations.get(&type_id)
    }

    /// Returns a mutable reference to the [`TypeRegistration`] of the type with
    /// the given [`TypeId`].
    ///
    /// If the specified type has not been registered, returns `None`.
    pub fn get_mut(&mut self, type_id: TypeId) -> Option<&mut TypeRegistration> {
        self.registrations.get_mut(&type_id)
    }

    /// Returns a reference to the [`TypeRegistration`] of the type with the
    /// given [type path].
    ///
    /// If no type with the given path has been registered, returns `None`.
    ///
    /// [type path]: crate::TypePath::full
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
    /// [type path]: crate::TypePath::full
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
    /// [short type path]: crate::TypePath::short
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
    /// [short type path]: crate::TypePath::short
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
    /// # use bevy_reflect2::TypeRegistry;
    /// # mod foo {
    /// #     use bevy_reflect2::Reflect;
    /// #     #[derive(Reflect)]
    /// #     pub struct MyType;
    /// # }
    /// # mod bar {
    /// #     use bevy_reflect2::Reflect;
    /// #     #[derive(Reflect)]
    /// #     pub struct MyType;
    /// # }
    /// let mut type_registry = TypeRegistry::default();
    /// type_registry.register::<foo::MyType>();
    /// type_registry.register::<bar::MyType>();
    /// assert_eq!(type_registry.is_ambiguous("MyType"), true);
    /// ```
    ///
    /// [short type path]: crate::TypePath::short
    pub fn is_ambiguous(&self, short_type_path: &str) -> bool {
        self.ambiguous_names.contains(short_type_path)
    }

    /// Returns a reference to the [`TypeData`] of type `T` associated with the given [`TypeId`].
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

    /// Returns the [`Type`] associated with the given [`TypeId`].
    ///
    /// If the specified type has not been registered, returns `None`.
    pub fn get_type(&self, type_id: TypeId) -> Option<&'static Type> {
        self.get(type_id).map(|r| r.ty)
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
/// Along with the type's [`Type`],
/// this struct also contains a type's registered [`TypeData`].
///
/// See the [crate-level documentation] for more information on type registration.
///
/// # Example
///
/// ```
/// # use bevy_reflect2::{CreateTypeData, TypeRegistration};
/// #[derive(Clone)]
/// struct ReflectMarker;
///
/// impl<T> CreateTypeData<T> for ReflectMarker {
///   fn create_type_data(_input: ()) -> Self {
///     Self
///   }
/// }
///
/// let mut registration = TypeRegistration::of::<Option<String>>();
///
/// assert_eq!("core::option::Option<alloc::string::String>", registration.ty().type_path().full());
/// assert_eq!("Option<String>", registration.ty().type_path().short());
///
/// registration.insert::<ReflectMarker>(CreateTypeData::<Option<String>>::create_type_data(()));
/// assert!(registration.data::<ReflectMarker>().is_some())
/// ```
///
/// [crate-level documentation]: crate
pub struct TypeRegistration {
    data: TypeIdHashMap<Box<dyn TypeData>>,
    ty: &'static Type,
}

impl Debug for TypeRegistration {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TypeRegistration")
            .field("ty", &self.ty)
            .finish()
    }
}

impl TypeRegistration {
    /// Creates type registration information for `T`.
    pub fn of<T: Typed>() -> Self {
        Self {
            data: Default::default(),
            ty: T::TYPE,
        }
    }

    /// Returns the [`TypeId`] of the type.
    #[inline]
    pub fn type_id(&self) -> TypeId {
        self.ty.type_id()
    }

    /// Returns a reference to the registration's [`Type`]
    pub fn ty(&self) -> &'static Type {
        self.ty
    }

    /// Inserts an instance of `T` into this registration's [type data].
    ///
    /// If another instance of `T` was previously inserted, it is replaced.
    ///
    /// Note that unlike [`TypeRegistration::register_type_data`], this will not
    /// automatically insert any type data dependencies for `T`.
    /// That will need to be done manually using [`CreateTypeData::insert_dependencies`].
    ///
    /// [type data]: TypeData
    pub fn insert<T: TypeData>(&mut self, data: T) {
        self.data.insert(TypeId::of::<T>(), Box::new(data));
    }

    /// Gets the instance of `T` into this registration's [type data], if it exists. If it does not
    /// exist, it will insert a new instance using `get_data` and then return it.
    ///
    /// [type data]: TypeData
    pub fn get_or_insert_data_with<T: TypeData>(&mut self, get_data: impl FnOnce() -> T) -> &mut T {
        let boxed_data = self
            .data
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(get_data()));
        boxed_data.downcast_mut::<T>().unwrap()
    }

    /// Inserts the [`TypeData`] instance of `T` created for `V`, and inserts any
    /// [`TypeData`] dependencies for that combination of `T` and `V`.
    ///
    /// To register [`TypeData`] that requires input (i.e. it doesn't take `()` as its input),
    /// see [`TypeRegistration::register_type_data_with`].
    #[inline]
    pub fn register_type_data<T: CreateTypeData<V>, V>(&mut self) {
        self.insert(T::create_type_data(()));
        T::insert_dependencies(self);
    }

    /// Inserts the [`TypeData`] instance of `T` created for `V` with some input `I`,
    /// and inserts any [`TypeData`] dependencies for that combination of `T`, `V`, and `I`.
    ///
    /// To register [`TypeData`] that doesn't require input (i.e. it expects `()`),
    /// see [`TypeRegistration::register_type_data`].
    #[inline]
    pub fn register_type_data_with<T: CreateTypeData<V, I>, V, I>(&mut self, input: I) {
        self.insert(T::create_type_data(input));
        T::insert_dependencies(self);
    }

    /// Returns a reference to the value of type `T` in this registration's
    /// [type data].
    ///
    /// Returns `None` if no such value exists.
    ///
    /// For a dynamic version of this method, see [`data_by_id`].
    ///
    /// [type data]: TypeData
    /// [`data_by_id`]: Self::data_by_id
    pub fn data<T: TypeData>(&self) -> Option<&T> {
        self.data
            .get(&TypeId::of::<T>())
            .and_then(|value| value.downcast_ref())
    }

    /// Returns a reference to the value with the given [`TypeId`] in this registration's
    /// [type data].
    ///
    /// Returns `None` if no such value exists.
    ///
    /// For a static version of this method, see [`data`].
    ///
    /// [type data]: TypeData
    /// [`data`]: Self::data
    pub fn data_by_id(&self, type_id: TypeId) -> Option<&dyn TypeData> {
        self.data.get(&type_id).map(Deref::deref)
    }

    /// Returns a mutable reference to the value of type `T` in this registration's
    /// [type data].
    ///
    /// Returns `None` if no such value exists.
    ///
    /// For a dynamic version of this method, see [`data_mut_by_id`].
    ///
    /// [type data]: TypeData
    /// [`data_mut_by_id`]: Self::data_mut_by_id
    pub fn data_mut<T: TypeData>(&mut self) -> Option<&mut T> {
        self.data
            .get_mut(&TypeId::of::<T>())
            .and_then(|value| value.downcast_mut())
    }

    /// Returns a mutable reference to the value with the given [`TypeId`] in this registration's
    /// [type data].
    ///
    /// Returns `None` if no such value exists.
    ///
    /// For a static version of this method, see [`data_mut`].
    ///
    /// [type data]: TypeData
    /// [`data_mut`]: Self::data_mut
    pub fn data_mut_by_id(&mut self, type_id: TypeId) -> Option<&mut dyn TypeData> {
        self.data.get_mut(&type_id).map(DerefMut::deref_mut)
    }

    /// Returns true if this registration contains the given [type data].
    ///
    /// For a dynamic version of this method, see [`contains_by_id`].
    ///
    /// [type data]: TypeData
    /// [`contains_by_id`]: Self::contains_by_id
    pub fn contains<T: TypeData>(&self) -> bool {
        self.data.contains_key(&TypeId::of::<T>())
    }

    /// Returns true if this registration contains the given [type data] with [`TypeId`].
    ///
    /// For a static version of this method, see [`contains`].
    ///
    /// [type data]: TypeData
    /// [`contains`]: Self::contains
    pub fn contains_by_id(&self, type_id: TypeId) -> bool {
        self.data.contains_key(&type_id)
    }

    /// The total count of [type data] in this registration.
    ///
    /// [type data]: TypeData
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if this registration has no [type data].
    ///
    /// [type data]: TypeData
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns an iterator over all [type data] in this registration.
    ///
    /// The iterator yields a tuple of the [`TypeId`] and its corresponding type data.
    ///
    /// [type data]: TypeData
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (TypeId, &dyn TypeData)> {
        self.data.iter().map(|(id, data)| (*id, data.deref()))
    }

    /// Returns a mutable iterator over all [type data] in this registration.
    ///
    /// The iterator yields a tuple of the [`TypeId`] and its corresponding type data.
    ///
    /// [type data]: TypeData
    pub fn iter_mut(&mut self) -> impl ExactSizeIterator<Item = (TypeId, &mut dyn TypeData)> {
        self.data
            .iter_mut()
            .map(|(id, data)| (*id, data.deref_mut()))
    }
}

impl Clone for TypeRegistration {
    fn clone(&self) -> Self {
        let mut data = TypeIdHashMap::default();
        for (id, type_data) in &self.data {
            data.insert(*id, (*type_data).clone_type_data());
        }

        TypeRegistration { data, ty: self.ty }
    }
}

#[cfg(test)]
mod test {
    use crate::{self as bevy_reflect2, Reflect, TypeRegistration};
    use core::any::TypeId;

    #[test]
    fn type_data_iter() {
        #[derive(Reflect)]
        struct Foo;

        #[derive(Clone)]
        struct DataA(i32);

        let mut registration = TypeRegistration::of::<Foo>();
        registration.insert(DataA(123));

        let mut iter = registration.iter();

        let (id, data) = iter.next().unwrap();
        assert_eq!(id, TypeId::of::<DataA>());
        assert_eq!(data.downcast_ref::<DataA>().unwrap().0, 123);

        assert!(iter.next().is_none());
    }

    #[test]
    fn type_data_iter_mut() {
        #[derive(Reflect)]
        struct Foo;

        #[derive(Clone)]
        struct DataA(i32);

        let mut registration = TypeRegistration::of::<Foo>();
        registration.insert(DataA(123));

        {
            let mut iter = registration.iter_mut();

            let (_, data) = iter.next().unwrap();
            data.downcast_mut::<DataA>().unwrap().0 = 456;

            assert!(iter.next().is_none());
        }

        let data = registration.data::<DataA>().unwrap();
        assert_eq!(data.0, 456);
    }
}
