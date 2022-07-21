use crate::{serde::Serializable, Reflect, TypeInfo, Typed};
use bevy_ptr::{Ptr, PtrMut};
use bevy_utils::{HashMap, HashSet};
use downcast_rs::{impl_downcast, Downcast};
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use serde::Deserialize;
use std::{any::TypeId, fmt::Debug, sync::Arc};

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
    pub fn of<T: Reflect + Typed>() -> Self {
        let type_name = std::any::type_name::<T>();
        Self {
            data: HashMap::default(),
            short_name: bevy_utils::get_short_name(type_name),
            type_info: T::type_info(),
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
mod test {
    use std::ptr::NonNull;

    use crate::{GetTypeRegistration, ReflectFromPtr, TypeRegistration};
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
