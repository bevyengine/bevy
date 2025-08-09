use crate::{
    array::{Array, ArrayInfo, ArrayIter},
    error::ReflectCloneError,
    kind::{ReflectKind, ReflectMut, ReflectOwned, ReflectRef},
    prelude::*,
    reflect::ApplyError,
    type_info::{MaybeTyped, OpaqueInfo, TypeInfo, Typed},
    type_registry::{
        FromType, GetTypeRegistration, ReflectDeserialize, ReflectFromPtr, ReflectSerialize,
        TypeRegistration, TypeRegistry,
    },
    utility::{reflect_hasher, GenericTypeInfoCell, GenericTypePathCell, NonGenericTypeInfoCell},
};
use bevy_platform::prelude::*;
use bevy_reflect_derive::{impl_reflect_opaque, impl_type_path};
use core::any::Any;
use core::fmt;
use core::hash::{Hash, Hasher};

impl_reflect_opaque!(bool(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(char(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(u8(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(u16(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(u32(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(u64(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(u128(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(usize(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(i8(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(i16(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(i32(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(i64(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(i128(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(isize(
    Clone,
    Debug,
    Hash,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(f32(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_reflect_opaque!(f64(
    Clone,
    Debug,
    PartialEq,
    Serialize,
    Deserialize,
    Default
));
impl_type_path!(str);

impl PartialReflect for &'static str {
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        Some(<Self as Typed>::type_info())
    }

    #[inline]
    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }

    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self
    }

    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self
    }

    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        Ok(self)
    }

    fn try_as_reflect(&self) -> Option<&dyn Reflect> {
        Some(self)
    }

    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        Some(self)
    }

    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Opaque(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut<'_> {
        ReflectMut::Opaque(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Opaque(self)
    }

    fn reflect_clone(&self) -> Result<Box<dyn Reflect>, ReflectCloneError> {
        Ok(Box::new(*self))
    }

    fn reflect_hash(&self) -> Option<u64> {
        let mut hasher = reflect_hasher();
        Hash::hash(&Any::type_id(self), &mut hasher);
        Hash::hash(self, &mut hasher);
        Some(hasher.finish())
    }

    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        if let Some(value) = value.try_downcast_ref::<Self>() {
            Some(PartialEq::eq(self, value))
        } else {
            Some(false)
        }
    }

    fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        if let Some(value) = value.try_downcast_ref::<Self>() {
            self.clone_from(value);
        } else {
            return Err(ApplyError::MismatchedTypes {
                from_type: value.reflect_type_path().into(),
                to_type: Self::type_path().into(),
            });
        }
        Ok(())
    }
}

impl Reflect for &'static str {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }

    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }
}

impl Typed for &'static str {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| TypeInfo::Opaque(OpaqueInfo::new::<Self>()))
    }
}

impl GetTypeRegistration for &'static str {
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Self>();
        registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
        registration.insert::<ReflectFromReflect>(FromType::<Self>::from_type());
        registration.insert::<ReflectSerialize>(FromType::<Self>::from_type());
        registration
    }
}

impl FromReflect for &'static str {
    fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
        reflect.try_downcast_ref::<Self>().copied()
    }
}

impl<T: Reflect + MaybeTyped + TypePath + GetTypeRegistration, const N: usize> Array for [T; N] {
    #[inline]
    fn get(&self, index: usize) -> Option<&dyn PartialReflect> {
        <[T]>::get(self, index).map(|value| value as &dyn PartialReflect)
    }

    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        <[T]>::get_mut(self, index).map(|value| value as &mut dyn PartialReflect)
    }

    #[inline]
    fn len(&self) -> usize {
        N
    }

    #[inline]
    fn iter(&self) -> ArrayIter<'_> {
        ArrayIter::new(self)
    }

    #[inline]
    fn drain(self: Box<Self>) -> Vec<Box<dyn PartialReflect>> {
        self.into_iter()
            .map(|value| Box::new(value) as Box<dyn PartialReflect>)
            .collect()
    }
}

impl<T: Reflect + MaybeTyped + TypePath + GetTypeRegistration, const N: usize> PartialReflect
    for [T; N]
{
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        Some(<Self as Typed>::type_info())
    }

    #[inline]
    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }

    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self
    }

    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self
    }

    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        Ok(self)
    }

    fn try_as_reflect(&self) -> Option<&dyn Reflect> {
        Some(self)
    }

    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        Some(self)
    }

    #[inline]
    fn reflect_kind(&self) -> ReflectKind {
        ReflectKind::Array
    }

    #[inline]
    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Array(self)
    }

    #[inline]
    fn reflect_mut(&mut self) -> ReflectMut<'_> {
        ReflectMut::Array(self)
    }

    #[inline]
    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Array(self)
    }

    #[inline]
    fn reflect_hash(&self) -> Option<u64> {
        crate::array_hash(self)
    }

    #[inline]
    fn reflect_partial_eq(&self, value: &dyn PartialReflect) -> Option<bool> {
        crate::array_partial_eq(self, value)
    }

    fn apply(&mut self, value: &dyn PartialReflect) {
        crate::array_apply(self, value);
    }

    #[inline]
    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        crate::array_try_apply(self, value)
    }
}

impl<T: Reflect + MaybeTyped + TypePath + GetTypeRegistration, const N: usize> Reflect for [T; N] {
    #[inline]
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    #[inline]
    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }

    #[inline]
    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    #[inline]
    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    #[inline]
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }
}

impl<T: FromReflect + MaybeTyped + TypePath + GetTypeRegistration, const N: usize> FromReflect
    for [T; N]
{
    fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
        let ref_array = reflect.reflect_ref().as_array().ok()?;

        let mut temp_vec = Vec::with_capacity(ref_array.len());

        for field in ref_array.iter() {
            temp_vec.push(T::from_reflect(field)?);
        }

        temp_vec.try_into().ok()
    }
}

impl<T: Reflect + MaybeTyped + TypePath + GetTypeRegistration, const N: usize> Typed for [T; N] {
    fn type_info() -> &'static TypeInfo {
        static CELL: GenericTypeInfoCell = GenericTypeInfoCell::new();
        CELL.get_or_insert::<Self, _>(|| TypeInfo::Array(ArrayInfo::new::<Self, T>(N)))
    }
}

impl<T: TypePath, const N: usize> TypePath for [T; N] {
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| format!("[{t}; {N}]", t = T::type_path()))
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| format!("[{t}; {N}]", t = T::short_type_path()))
    }
}

impl<T: Reflect + MaybeTyped + TypePath + GetTypeRegistration, const N: usize> GetTypeRegistration
    for [T; N]
{
    fn get_type_registration() -> TypeRegistration {
        TypeRegistration::of::<[T; N]>()
    }

    fn register_type_dependencies(registry: &mut TypeRegistry) {
        registry.register::<T>();
    }
}

#[cfg(feature = "functions")]
crate::func::macros::impl_function_traits!([T; N]; <T: Reflect + MaybeTyped + TypePath + GetTypeRegistration> [const N: usize]);

impl<T: TypePath> TypePath for [T]
where
    [T]: ToOwned,
{
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| format!("[{}]", <T>::type_path()))
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| format!("[{}]", <T>::short_type_path()))
    }
}

impl<T: TypePath + ?Sized> TypePath for &'static T {
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| format!("&{}", T::type_path()))
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| format!("&{}", T::short_type_path()))
    }
}

impl<T: TypePath + ?Sized> TypePath for &'static mut T {
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| format!("&mut {}", T::type_path()))
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| format!("&mut {}", T::short_type_path()))
    }
}

#[cfg(test)]
mod tests {
    use bevy_reflect::{FromReflect, PartialReflect};
    use core::f32::consts::{PI, TAU};

    #[test]
    fn should_partial_eq_char() {
        let a: &dyn PartialReflect = &'x';
        let b: &dyn PartialReflect = &'x';
        let c: &dyn PartialReflect = &'o';
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_i32() {
        let a: &dyn PartialReflect = &123_i32;
        let b: &dyn PartialReflect = &123_i32;
        let c: &dyn PartialReflect = &321_i32;
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_f32() {
        let a: &dyn PartialReflect = &PI;
        let b: &dyn PartialReflect = &PI;
        let c: &dyn PartialReflect = &TAU;
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn static_str_should_from_reflect() {
        let expected = "Hello, World!";
        let output = <&'static str as FromReflect>::from_reflect(&expected).unwrap();
        assert_eq!(expected, output);
    }
}
