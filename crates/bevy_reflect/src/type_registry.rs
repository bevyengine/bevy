use crate::{serde::Serializable, Reflect, TypeInfo, Typed};
use bevy_ptr::{Ptr, PtrMut};
use bevy_utils::tracing::warn;
use bevy_utils::{HashMap, HashSet};
use downcast_rs::{impl_downcast, Downcast};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use serde::Deserialize;
use std::borrow::Cow;
use std::{any::TypeId, fmt::Debug, sync::Arc};

/// A registry of reflected types.
pub struct TypeRegistry {
    registrations: HashMap<TypeId, TypeRegistration>,
    short_name_to_id: HashMap<String, TypeId>,
    full_name_to_id: HashMap<String, TypeId>,
    alias_to_id: HashMap<Cow<'static, str>, AliasData>,
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

    /// Returns the static set of aliases that can be used to refer to this type.
    ///
    /// Note that these are the _default_ aliases used specifically for type registration.
    /// For a given [registry], the actual set of aliases for a registered type may differ from the
    /// ones listed here.
    ///
    /// If you need the list of aliases for this type, please use [`TypeRegistration::aliases`].
    ///
    /// [registry]: TypeRegistry
    fn aliases() -> &'static [&'static str] {
        &[]
    }

    /// Returns the static set of _deprecated_ aliases that can be used to refer to this type.
    ///
    /// For the list of _current_ aliases, try using [`aliases`] instead.
    ///
    /// Note that, like [`aliases`], this is the _default_ set used specifically for type registration.
    /// For a given [registry], the actual set of deprecated aliases for a registered type may differ from the
    /// ones listed here.
    ///
    /// If you need the list of aliases for this type, please use [`TypeRegistration::deprecated_aliases`].
    ///
    /// [`aliases`]: GetTypeRegistration::aliases
    /// [registry]: TypeRegistry
    fn deprecated_aliases() -> &'static [&'static str] {
        &[]
    }
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
            alias_to_id: Default::default(),
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

    /// Registers the type `T`, adding reflect data as specified in the [`Reflect`] derive:
    /// ```rust,ignore
    /// #[derive(Reflect)]
    /// #[reflect(Component, Serialize, Deserialize)] // will register ReflectComponent, ReflectSerialize, ReflectDeserialize
    /// ```
    pub fn register<T>(&mut self)
    where
        T: GetTypeRegistration,
    {
        self.add_registration(T::get_type_registration());
    }

    /// Registers the type described by `registration`.
    pub fn add_registration(&mut self, registration: TypeRegistration) {
        let type_id = registration.type_id();
        let type_name = registration.type_name();
        let short_name = registration.short_name.to_string();
        if self.short_name_to_id.contains_key(&short_name)
            || self.ambiguous_names.contains(&short_name)
        {
            // name is ambiguous. fall back to long names for all ambiguous types
            self.short_name_to_id.remove(&short_name);
            self.ambiguous_names.insert(short_name);
        } else {
            self.short_name_to_id.insert(short_name, type_id);
        }

        for alias in &registration.aliases {
            self.register_alias_internal(alias.clone(), type_name, type_id, false, true);
        }
        for alias in &registration.deprecated_aliases {
            self.register_alias_internal(alias.clone(), type_name, type_id, true, true);
        }

        self.full_name_to_id.insert(type_name.to_string(), type_id);
        self.registrations.insert(type_id, registration);
    }

    /// Registers the type data `D` for type `T`.
    ///
    /// Most of the time [`TypeRegistry::register`] can be used instead to register a type you derived [`Reflect`] for.
    /// However, in cases where you want to add a piece of type data that was not included in the list of `#[reflect(...)]` type data in the derive,
    /// or where the type is generic and cannot register e.g. `ReflectSerialize` unconditionally without knowing the specific type parameters,
    /// this method can be used to insert additional type data.
    ///
    /// # Example
    /// ```rust
    /// use bevy_reflect::{TypeRegistry, ReflectSerialize, ReflectDeserialize};
    ///
    /// let mut type_registry = TypeRegistry::default();
    /// type_registry.register::<Option<String>>();
    /// type_registry.register_type_data::<Option<String>, ReflectSerialize>();
    /// type_registry.register_type_data::<Option<String>, ReflectDeserialize>();
    /// ```
    pub fn register_type_data<T: Reflect + 'static, D: TypeData + FromType<T>>(&mut self) {
        let data = self.get_mut(TypeId::of::<T>()).unwrap_or_else(|| {
            panic!(
                "attempted to call `TypeRegistry::register_type_data` for type `{T}` with data `{D}` without registering `{T}` first",
                T = std::any::type_name::<T>(),
                D = std::any::type_name::<D>(),
            )
        });
        data.insert(D::from_type());
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

    /// Returns a reference to the [`TypeRegistration`] of the type with the
    /// given alias.
    ///
    /// If no type with the given alias has been registered, returns `None`.
    pub fn get_with_alias(&self, alias: &str) -> Option<&TypeRegistration> {
        let alias_data = self.alias_to_id.get(alias)?;
        let registration = self.get(alias_data.type_id)?;

        if alias_data.is_deprecated {
            Self::warn_alias_deprecation(alias, registration);
        }

        Some(registration)
    }

    /// Returns a mutable reference to the [`TypeRegistration`] of the type with
    /// the given alias.
    ///
    /// If no type with the given alias has been registered, returns `None`.
    pub fn get_with_alias_mut(&mut self, alias: &str) -> Option<&mut TypeRegistration> {
        let alias_data = *self.alias_to_id.get(alias)?;
        let registration = self.get_mut(alias_data.type_id)?;

        if alias_data.is_deprecated {
            Self::warn_alias_deprecation(alias, registration);
        }

        Some(registration)
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

    /// Register an alias for the given type, `T`.
    ///
    /// This will implicitly overwrite existing usages of the given alias
    /// and print a warning to the console if it does so.
    ///
    /// To register the alias only if it isn't already in use, try using [`try_register_alias`](Self::try_register_alias).
    /// Otherwise, to explicitly overwrite existing aliases without the warning, try using [`overwrite_alias`](Self::overwrite_alias).
    ///
    /// If an alias was overwritten, then the [`TypeId`] of the previous type is returned.
    pub fn register_alias<T: Reflect + 'static>(
        &mut self,
        alias: impl Into<Cow<'static, str>>,
    ) -> Option<TypeId> {
        let registerer = AliasRegisterer::implicit_overwrite(self, "TypeRegistry::register_alias");
        registerer.register::<T>(alias, false)
    }

    /// Attempts to register an alias for the given type, `T`, if it isn't already in use.
    ///
    /// To register the alias whether or not it exists, try using either [`register_alias`](Self::register_alias) or
    /// [`overwrite_alias`](Self::overwrite_alias).
    ///
    /// If the given alias is already in use, then the [`TypeId`] of that type is returned.
    pub fn try_register_alias<T: Reflect + 'static>(
        &mut self,
        alias: impl Into<Cow<'static, str>>,
    ) -> Option<TypeId> {
        let registerer = AliasRegisterer::no_overwrite(self, "TypeRegistry::try_register_alias");
        registerer.register::<T>(alias, false)
    }

    /// Register an alias for the given type, `T`, explicitly overwriting existing aliases.
    ///
    /// Unlike, [`register_alias`](Self::register_alias), this does not print a warning when overwriting existing aliases.
    ///
    /// To register the alias only if it isn't already in use, try using [`try_register_alias`](Self::try_register_alias).
    ///
    /// If an alias was overwritten, then the [`TypeId`] of the previous type is returned.
    pub fn overwrite_alias<T: Reflect + 'static>(
        &mut self,
        alias: impl Into<Cow<'static, str>>,
    ) -> Option<TypeId> {
        let registerer = AliasRegisterer::explicit_overwrite(self, "TypeRegistry::overwrite_alias");
        registerer.register::<T>(alias, false)
    }

    /// Registers an alias for the given type.
    fn register_alias_internal(
        &mut self,
        alias: Cow<'static, str>,
        type_name: &'static str,
        type_id: TypeId,
        is_deprecated: bool,
        should_warn: bool,
    ) -> Option<TypeId> {
        let existing = self.alias_to_id.insert(
            alias.clone(),
            AliasData {
                type_id,
                is_deprecated,
            },
        );

        if let Some(existing) = existing.and_then(|existing| self.get_mut(existing.type_id)) {
            existing.aliases.remove(&alias);
            if should_warn {
                warn!(
                     "overwrote alias `{alias}` — was assigned to type `{}` ({:?}), but is now assigned to type `{}` ({:?})",
                     existing.type_name(),
                     existing.type_id(),
                     type_name,
                     type_id
                 );
            }

            Some(existing.type_id())
        } else {
            None
        }
    }

    /// Prints a warning stating that the given alias has been deprecated for the given registration.
    fn warn_alias_deprecation(alias: &str, registration: &TypeRegistration) {
        warn!(
            "the alias `{}` has been deprecated for the type `{}` ({:?}) and may be removed in the future. \
            Consider using the full type name or one of the current aliases: {:?}",
            alias,
            registration.type_name(),
            registration.type_id(),
            registration.aliases(),
        );
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
/// This contains the [`TypeInfo`] of the type, as well as its [short name].
///
/// For each trait specified by the [`#[reflect(_)]`][0] attribute of
/// [`#[derive(Reflect)]`][1] on the registered type, this record also contains
/// a [`TypeData`] which can be used to downcast [`Reflect`] trait objects of
/// this type to trait objects of the relevant trait.
///
/// [short name]: bevy_utils::get_short_name
/// [`TypeInfo`]: crate::TypeInfo
/// [0]: crate::Reflect
/// [1]: crate::Reflect
pub struct TypeRegistration {
    short_name: String,
    data: HashMap<TypeId, Box<dyn TypeData>>,
    type_info: &'static TypeInfo,
    aliases: HashSet<Cow<'static, str>>,
    deprecated_aliases: HashSet<Cow<'static, str>>,
}

impl TypeRegistration {
    /// Returns the [`TypeId`] of the type.
    ///
    /// [`TypeId`]: std::any::TypeId
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
    pub fn of<T: Reflect + Typed + GetTypeRegistration>() -> Self {
        let type_name = std::any::type_name::<T>();
        Self {
            data: HashMap::default(),
            short_name: bevy_utils::get_short_name(type_name),
            type_info: T::type_info(),
            aliases: HashSet::from_iter(T::aliases().iter().map(|&alias| Cow::Borrowed(alias))),
            deprecated_aliases: HashSet::from_iter(
                T::deprecated_aliases()
                    .iter()
                    .map(|&alias| Cow::Borrowed(alias)),
            ),
        }
    }

    /// Returns the [short name] of the type.
    ///
    /// [short name]: bevy_utils::get_short_name
    pub fn short_name(&self) -> &str {
        &self.short_name
    }

    /// Returns the [name] of the type.
    ///
    /// [name]: std::any::type_name
    pub fn type_name(&self) -> &'static str {
        self.type_info.type_name()
    }

    /// Returns the default set of aliases for the type.
    ///
    /// For the set of _deprecated_ aliases, try [`deprecated_aliases`](Self::deprecated_aliases).
    pub fn aliases(&self) -> &HashSet<Cow<'static, str>> {
        &self.aliases
    }

    /// Returns the default set of _deprecated_ aliases for the type.
    ///
    /// For the set of _current_ aliases, try [`aliases`](Self::aliases).
    pub fn deprecated_aliases(&self) -> &HashSet<Cow<'static, str>> {
        &self.deprecated_aliases
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
            short_name: self.short_name.clone(),
            type_info: self.type_info,
            aliases: self.aliases.clone(),
            deprecated_aliases: self.deprecated_aliases.clone(),
        }
    }
}

#[derive(Copy, Clone)]
struct AliasData {
    pub type_id: TypeId,
    pub is_deprecated: bool,
}

/// A simple helper struct for registering type aliases.
struct AliasRegisterer<'a> {
    registry: &'a mut TypeRegistry,
    func_name: &'static str,
    allow_overwrite: bool,
    should_warn: bool,
}

impl<'a> AliasRegisterer<'a> {
    /// Configure the registerer to register aliases with an implicit overwrite (produces a warning).
    fn implicit_overwrite(registry: &'a mut TypeRegistry, func_name: &'static str) -> Self {
        Self {
            registry,
            func_name,
            allow_overwrite: true,
            should_warn: true,
        }
    }

    /// Configure the registerer to register aliases with an explicit overwrite (does not produce a warning).
    fn explicit_overwrite(registry: &'a mut TypeRegistry, func_name: &'static str) -> Self {
        Self {
            registry,
            func_name,
            allow_overwrite: true,
            should_warn: false,
        }
    }

    /// Configure the registerer to register aliases as long as they are not already in use.
    fn no_overwrite(registry: &'a mut TypeRegistry, func_name: &'static str) -> Self {
        Self {
            registry,
            func_name,
            allow_overwrite: false,
            should_warn: false,
        }
    }

    /// Register the given alias for type, `T`.
    fn register<T: Reflect + 'static>(
        self,
        alias: impl Into<Cow<'static, str>>,
        is_deprecated: bool,
    ) -> Option<TypeId> {
        let Self {
            registry,
            func_name,
            allow_overwrite,
            should_warn,
        } = self;

        let alias = alias.into();

        if !allow_overwrite {
            if let Some(data) = registry.alias_to_id.get(&alias) {
                return Some(data.type_id);
            }
        }

        let registration = registry.get_mut(TypeId::of::<T>()).unwrap_or_else(|| {
            panic!(
                "attempted to call `{func_name}` for type `{T}` with alias `{alias}` without registering `{T}` first",
                T = std::any::type_name::<T>(),
            )
        });

        registration.aliases.insert(alias.clone());

        let type_name = registration.type_name();
        let type_id = registration.type_id();

        registry.register_alias_internal(alias, type_name, type_id, is_deprecated, should_warn)
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

/// A struct used to serialize reflected instances of a type.
///
/// A `ReflectSerialize` for type `T` can be obtained via
/// [`FromType::from_type`].
#[derive(Clone)]
pub struct ReflectSerialize {
    get_serializable: for<'a> fn(value: &'a dyn Reflect) -> Serializable,
}

impl<T: Reflect + erased_serde::Serialize> FromType<T> for ReflectSerialize {
    fn from_type() -> Self {
        ReflectSerialize {
            get_serializable: |value| {
                let value = value.downcast_ref::<T>().unwrap_or_else(|| {
                    panic!("ReflectSerialize::get_serialize called with type `{}`, even though it was created for `{}`", value.type_name(), std::any::type_name::<T>())
                });
                Serializable::Borrowed(value)
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
/// ```rust
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
/// let value = unsafe { Ptr::new(NonNull::from(&mut value).cast()) };
///
/// let reflect_data = type_registry.get(std::any::TypeId::of::<Reflected>()).unwrap();
/// let reflect_from_ptr = reflect_data.data::<ReflectFromPtr>().unwrap();
/// // SAFE: `value` is of type `Reflected`, which the `ReflectFromPtr` was created for
/// let value = unsafe { reflect_from_ptr.as_reflect_ptr(value) };
///
/// assert_eq!(value.downcast_ref::<Reflected>().unwrap().0, "Hello world!");
/// ```
#[derive(Clone)]
pub struct ReflectFromPtr {
    type_id: TypeId,
    to_reflect: for<'a> unsafe fn(Ptr<'a>) -> &'a dyn Reflect,
    to_reflect_mut: for<'a> unsafe fn(PtrMut<'a>) -> &'a mut dyn Reflect,
}

impl ReflectFromPtr {
    /// Returns the [`TypeId`] that the [`ReflectFromPtr`] was constructed for
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// # Safety
    ///
    /// `val` must be a pointer to value of the type that the [`ReflectFromPtr`] was constructed for.
    /// This can be verified by checking that the type id returned by [`ReflectFromPtr::type_id`] is the expected one.
    pub unsafe fn as_reflect_ptr<'a>(&self, val: Ptr<'a>) -> &'a dyn Reflect {
        (self.to_reflect)(val)
    }

    /// # Safety
    ///
    /// `val` must be a pointer to a value of the type that the [`ReflectFromPtr`] was constructed for
    /// This can be verified by checking that the type id returned by [`ReflectFromPtr::type_id`] is the expected one.
    pub unsafe fn as_reflect_ptr_mut<'a>(&self, val: PtrMut<'a>) -> &'a mut dyn Reflect {
        (self.to_reflect_mut)(val)
    }
}

impl<T: Reflect> FromType<T> for ReflectFromPtr {
    fn from_type() -> Self {
        ReflectFromPtr {
            type_id: std::any::TypeId::of::<T>(),
            to_reflect: |ptr| {
                // SAFE: only called from `as_reflect`, where the `ptr` is guaranteed to be of type `T`,
                // and `as_reflect_ptr`, where the caller promises to call it with type `T`
                unsafe { ptr.deref::<T>() as &dyn Reflect }
            },
            to_reflect_mut: |ptr| {
                // SAFE: only called from `as_reflect_mut`, where the `ptr` is guaranteed to be of type `T`,
                // and `as_reflect_ptr_mut`, where the caller promises to call it with type `T`
                unsafe { ptr.deref_mut::<T>() as &mut dyn Reflect }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;
    use std::ptr::NonNull;

    use crate::{GetTypeRegistration, ReflectFromPtr, TypeRegistration, TypeRegistry};
    use bevy_ptr::{Ptr, PtrMut};
    use bevy_utils::HashMap;

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
            // SAFETY: lifetime doesn't outlive original value, access is unique
            let value = unsafe { PtrMut::new(NonNull::from(&mut value).cast()) };
            // SAFETY: reflect_from_ptr was constructed for the correct type
            let dyn_reflect = unsafe { reflect_from_ptr.as_reflect_ptr_mut(value) };
            match dyn_reflect.reflect_mut() {
                bevy_reflect::ReflectMut::Struct(strukt) => {
                    strukt.field_mut("a").unwrap().apply(&2.0f32);
                }
                _ => panic!("invalid reflection"),
            }
        }

        {
            // SAFETY: lifetime doesn't outlive original value
            let value = unsafe { Ptr::new(NonNull::from(&mut value).cast()) };
            // SAFETY: reflect_from_ptr was constructed for the correct type
            let dyn_reflect = unsafe { reflect_from_ptr.as_reflect_ptr(value) };
            match dyn_reflect.reflect_ref() {
                bevy_reflect::ReflectRef::Struct(strukt) => {
                    let a = strukt.field("a").unwrap().downcast_ref::<f32>().unwrap();
                    assert_eq!(*a, 2.0);
                }
                _ => panic!("invalid reflection"),
            }
        }
    }

    #[test]
    fn should_register_new_alias() {
        #[derive(Reflect)]
        struct Foo;
        #[derive(Reflect)]
        struct Bar;

        let mut registry = TypeRegistry::empty();
        registry.register::<Foo>();
        registry.register::<Bar>();

        let previous = registry.register_alias::<Foo>("my_alias");
        assert_eq!(None, previous);

        let registration = registry.get_with_alias("my_alias").unwrap();
        assert_eq!(TypeId::of::<Foo>(), registration.type_id());
        assert!(registration.aliases().contains("my_alias"));

        let previous = registry.register_alias::<Bar>("my_alias");
        assert_eq!(Some(TypeId::of::<Foo>()), previous);

        let registration = registry.get_with_alias("my_alias").unwrap();
        assert_eq!(TypeId::of::<Bar>(), registration.type_id());
        assert!(registration.aliases().contains("my_alias"));

        // Confirm that the registrations' aliases have been updated
        let foo_registration = registry.get(TypeId::of::<Foo>()).unwrap();
        let bar_registration = registry.get(TypeId::of::<Bar>()).unwrap();
        assert!(!foo_registration.aliases().contains("my_alias"));
        assert!(bar_registration.aliases().contains("my_alias"));
    }

    #[test]
    fn should_not_register_existing_alias() {
        #[derive(Reflect)]
        struct Foo;
        #[derive(Reflect)]
        struct Bar;

        let mut registry = TypeRegistry::empty();
        registry.register::<Foo>();
        registry.register::<Bar>();

        registry.register_alias::<Foo>("my_alias");
        let current = registry.try_register_alias::<Bar>("my_alias");
        assert_eq!(Some(TypeId::of::<Foo>()), current);

        let registration = registry.get_with_alias("my_alias").unwrap();
        assert_eq!(TypeId::of::<Foo>(), registration.type_id());

        // Confirm that the registrations' aliases have been updated
        let foo_registration = registry.get(TypeId::of::<Foo>()).unwrap();
        let bar_registration = registry.get(TypeId::of::<Bar>()).unwrap();
        assert!(foo_registration.aliases().contains("my_alias"));
        assert!(!bar_registration.aliases().contains("my_alias"));
    }

    #[test]
    #[should_panic(expected = "attempted to call `TypeRegistry::register_alias` for type")]
    fn register_alias_should_panic_if_no_registration() {
        #[derive(Reflect)]
        struct Foo;

        let mut registry = TypeRegistry::empty();
        registry.register_alias::<Foo>("my_alias");
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
}
