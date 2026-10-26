use crate::{
    array::{Array, ArrayInfo, ArrayIter},
    dynamic::DynamicValue,
    error::ReflectCloneError,
    info::{MaybeTyped, OpaqueInfo, TypeInfo, Typed},
    kind::{ReflectKind, ReflectMut, ReflectOwned, ReflectRef},
    prelude::*,
    type_registry::{
        GetTypeRegistration, ReflectDeserialize, ReflectFromPtr, ReflectSerialize,
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
    PartialOrd,
    Serialize,
    Deserialize,
    Default,
));
impl_reflect_opaque!(char(
    Clone,
    Debug,
    Hash,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Default,
));
impl_reflect_opaque!(u8(
    Clone,
    Debug,
    Hash,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Default,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Mul,
    MulAssign,
    Div,
    DivAssign,
    Rem,
    RemAssign,
));
impl_reflect_opaque!(u16(
    Clone,
    Debug,
    Hash,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Default,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Mul,
    MulAssign,
    Div,
    DivAssign,
    Rem,
    RemAssign,
));
impl_reflect_opaque!(u32(
    Clone,
    Debug,
    Hash,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Default,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Mul,
    MulAssign,
    Div,
    DivAssign,
    Rem,
    RemAssign,
));
impl_reflect_opaque!(u64(
    Clone,
    Debug,
    Hash,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Default,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Mul,
    MulAssign,
    Div,
    DivAssign,
    Rem,
    RemAssign,
));
impl_reflect_opaque!(u128(
    Clone,
    Debug,
    Hash,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Default,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Mul,
    MulAssign,
    Div,
    DivAssign,
    Rem,
    RemAssign,
));
impl_reflect_opaque!(usize(
    Clone,
    Debug,
    Hash,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Default,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Mul,
    MulAssign,
    Div,
    DivAssign,
    Rem,
    RemAssign,
));
impl_reflect_opaque!(i8(
    Clone,
    Debug,
    Hash,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Default,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Mul,
    MulAssign,
    Div,
    DivAssign,
    Rem,
    RemAssign,
));
impl_reflect_opaque!(i16(
    Clone,
    Debug,
    Hash,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Default,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Mul,
    MulAssign,
    Div,
    DivAssign,
    Rem,
    RemAssign,
));
impl_reflect_opaque!(i32(
    Clone,
    Debug,
    Hash,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Default,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Mul,
    MulAssign,
    Div,
    DivAssign,
    Rem,
    RemAssign,
));
impl_reflect_opaque!(i64(
    Clone,
    Debug,
    Hash,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Default,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Mul,
    MulAssign,
    Div,
    DivAssign,
    Rem,
    RemAssign,
));
impl_reflect_opaque!(i128(
    Clone,
    Debug,
    Hash,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Default,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Mul,
    MulAssign,
    Div,
    DivAssign,
    Rem,
    RemAssign,
));
impl_reflect_opaque!(isize(
    Clone,
    Debug,
    Hash,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Default,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Mul,
    MulAssign,
    Div,
    DivAssign,
    Rem,
    RemAssign,
));
impl_reflect_opaque!(f32(
    Clone,
    Debug,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Default,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Mul,
    MulAssign,
    Div,
    DivAssign,
    Rem,
    RemAssign,
));
impl_reflect_opaque!(f64(
    Clone,
    Debug,
    PartialEq,
    PartialOrd,
    Serialize,
    Deserialize,
    Default,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Mul,
    MulAssign,
    Div,
    DivAssign,
    Rem,
    RemAssign,
));
impl_type_path!(str);

impl Reflect for &'static str {
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

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        if let Some(value) = value.downcast_ref::<Self>() {
            Some(PartialEq::eq(self, value))
        } else {
            Some(false)
        }
    }

    fn reflect_partial_cmp(&self, value: &dyn Reflect) -> Option<core::cmp::Ordering> {
        if let Some(value) = value.downcast_ref::<Self>() {
            PartialOrd::partial_cmp(self, value)
        } else {
            None
        }
    }

    fn debug(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self, f)
    }

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
        registration.register_type_data::<ReflectFromPtr, Self>();
        registration.register_type_data::<ReflectFromDynamic, Self>();
        registration.register_type_data::<ReflectSerialize, Self>();
        registration
    }
}

impl FromDynamic for &'static str {
    fn from_dynamic(dynamic: DynamicValue) -> Option<Self> {
        dynamic.into_concrete()?.downcast_ref::<Self>().copied()
    }
}

impl<T: Reflect + MaybeTyped + TypePath + GetTypeRegistration, const N: usize> Array for [T; N] {
    #[inline]
    fn get(&self, index: usize) -> Option<&dyn Reflect> {
        <[T]>::get(self, index).map(|value| value as &dyn Reflect)
    }

    #[inline]
    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        <[T]>::get_mut(self, index).map(|value| value as &mut dyn Reflect)
    }

    #[inline]
    fn swap(&mut self, a: usize, b: usize) {
        <[T]>::swap(self, a, b);
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
    fn drain(self: Box<Self>) -> Vec<Box<dyn Reflect>> {
        self.into_iter()
            .map(|value| Box::new(value) as Box<dyn Reflect>)
            .collect()
    }
}

impl<T: Reflect + MaybeTyped + TypePath + GetTypeRegistration, const N: usize> Reflect for [T; N] {
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
        crate::array::array_hash(self)
    }

    #[inline]
    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        crate::array::array_partial_eq(self, value)
    }

    #[inline]
    fn reflect_partial_cmp(&self, value: &dyn Reflect) -> Option<::core::cmp::Ordering> {
        crate::array::array_partial_cmp(self, value)
    }

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

impl<T: FromDynamic + MaybeTyped + TypePath + GetTypeRegistration, const N: usize> FromDynamic
    for [T; N]
{
    fn from_dynamic(dynamic: DynamicValue) -> Option<Self> {
        let dynamic = match dynamic {
            DynamicValue::Array(dynamic) => dynamic,
            DynamicValue::Concrete(concrete) => return concrete.take().ok(),
            _ => return None,
        };

        let mut temp_vec = Vec::with_capacity(dynamic.len());

        for field in dynamic.into_iter() {
            temp_vec.push(T::from_dynamic(field)?);
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
    use alloc::boxed::Box;
    use core::{
        any::TypeId,
        f32::consts::{PI, TAU},
        ops::{Add, Div, Mul, Rem, Sub},
    };

    use crate::{
        dynamic::DynamicValue,
        prelude::{
            ReflectAdd, ReflectDivAssign, ReflectMulAssign, ReflectRem, ReflectRemAssign,
            ReflectSubAssign,
        },
        std_traits::{ReflectAddAssign, ReflectDiv, ReflectMul, ReflectSub},
        FromDynamic, Reflect, TypeRegistry,
    };

    #[test]
    fn should_partial_eq_char() {
        let a: &dyn Reflect = &'x';
        let b: &dyn Reflect = &'x';
        let c: &dyn Reflect = &'o';
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_i32() {
        let a: &dyn Reflect = &123_i32;
        let b: &dyn Reflect = &123_i32;
        let c: &dyn Reflect = &321_i32;
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_eq_f32() {
        let a: &dyn Reflect = &PI;
        let b: &dyn Reflect = &PI;
        let c: &dyn Reflect = &TAU;
        assert!(a.reflect_partial_eq(b).unwrap_or_default());
        assert!(!a.reflect_partial_eq(c).unwrap_or_default());
    }

    #[test]
    fn should_partial_cmp_f32() {
        use core::cmp::Ordering;

        let a: &dyn Reflect = &1.0_f32;
        let b: &dyn Reflect = &2.0_f32;

        let ord = a.reflect_partial_cmp(b);
        assert_eq!(ord, Some(Ordering::Less));

        // mismatched type should return None
        let ord_mismatch = a.reflect_partial_cmp(&1i32);
        assert_eq!(ord_mismatch, None);
    }

    #[test]
    fn reflect_partial_cmp_f32_nan() {
        // NaN comparisons should return None
        let nan = f32::NAN;
        assert_eq!(Reflect::reflect_partial_cmp(&nan, &nan), None);
    }

    #[test]
    fn static_str_should_from_dynamic() {
        let expected = "Hello, World!";
        let output =
            <&'static str as FromDynamic>::from_dynamic(DynamicValue::Concrete(Box::new(expected)))
                .unwrap();
        assert_eq!(expected, output);
    }

    #[test]
    fn should_add() {
        fn check_add<T: Reflect + Add<Output = T> + Copy + 'static>(
            registry: &TypeRegistry,
            mut a: T,
            b: T,
            result: T,
        ) {
            let reflect_add = registry
                .get_type_data::<ReflectAdd>(TypeId::of::<T>())
                .unwrap();
            let reflect_add_assign = registry
                .get_type_data::<ReflectAddAssign>(TypeId::of::<T>())
                .unwrap();

            assert_eq!(
                reflect_add
                    .add(Box::new(a), Box::new(b))
                    .unwrap()
                    .reflect_partial_eq(&result),
                Some(true)
            );
            reflect_add_assign.add_assign(&mut a, Box::new(b)).unwrap();
            assert_eq!(a.reflect_partial_eq(&result), Some(true));

            assert!(reflect_add.add(Box::new(a), Box::new("not a T")).is_err());
            assert!(reflect_add.add(Box::new("not a T"), Box::new(b)).is_err());
            assert!(reflect_add_assign
                .add_assign(&mut a, Box::new("not a T"))
                .is_err());
            assert!(reflect_add_assign
                .add_assign(&mut "not a T", Box::new(b))
                .is_err());
        }

        let registry = TypeRegistry::new();

        check_add::<u8>(&registry, 10, 5, 15);
        check_add::<u16>(&registry, 10, 5, 15);
        check_add::<u32>(&registry, 10, 5, 15);
        check_add::<u64>(&registry, 10, 5, 15);
        check_add::<u128>(&registry, 10, 5, 15);
        check_add::<usize>(&registry, 10, 5, 15);
        check_add::<i8>(&registry, 10, 5, 15);
        check_add::<i16>(&registry, 10, 5, 15);
        check_add::<i32>(&registry, 10, 5, 15);
        check_add::<i64>(&registry, 10, 5, 15);
        check_add::<i128>(&registry, 10, 5, 15);
        check_add::<isize>(&registry, 10, 5, 15);
        check_add::<f32>(&registry, 1.5, 2.5, 4.0);
        check_add::<f64>(&registry, 1.5, 2.5, 4.0);
    }

    #[test]
    fn should_sub() {
        fn check_sub<T: Reflect + Sub<Output = T> + Copy + 'static>(
            registry: &TypeRegistry,
            mut a: T,
            b: T,
            result: T,
        ) {
            let reflect_sub = registry
                .get_type_data::<ReflectSub>(TypeId::of::<T>())
                .unwrap();
            let reflect_sub_assign = registry
                .get_type_data::<ReflectSubAssign>(TypeId::of::<T>())
                .unwrap();

            assert_eq!(
                reflect_sub
                    .sub(Box::new(a), Box::new(b))
                    .unwrap()
                    .reflect_partial_eq(&result),
                Some(true)
            );
            reflect_sub_assign.sub_assign(&mut a, Box::new(b)).unwrap();
            assert_eq!(a.reflect_partial_eq(&result), Some(true));

            assert!(reflect_sub.sub(Box::new(a), Box::new("not a T")).is_err());
            assert!(reflect_sub.sub(Box::new("not a T"), Box::new(b)).is_err());
            assert!(reflect_sub_assign
                .sub_assign(&mut a, Box::new("not a T"))
                .is_err());
            assert!(reflect_sub_assign
                .sub_assign(&mut "not a T", Box::new(b))
                .is_err());
        }

        let registry = TypeRegistry::new();

        check_sub::<u8>(&registry, 10, 5, 5);
        check_sub::<u16>(&registry, 10, 5, 5);
        check_sub::<u32>(&registry, 10, 5, 5);
        check_sub::<u64>(&registry, 10, 5, 5);
        check_sub::<u128>(&registry, 10, 5, 5);
        check_sub::<usize>(&registry, 10, 5, 5);
        check_sub::<i8>(&registry, 10, 5, 5);
        check_sub::<i16>(&registry, 10, 5, 5);
        check_sub::<i32>(&registry, 10, 5, 5);
        check_sub::<i64>(&registry, 10, 5, 5);
        check_sub::<i128>(&registry, 10, 5, 5);
        check_sub::<isize>(&registry, 10, 5, 5);
        check_sub::<f32>(&registry, 1.5, 2.5, -1.0);
        check_sub::<f64>(&registry, 1.5, 2.5, -1.0);
    }

    #[test]
    fn should_mul() {
        fn check_mul<T: Reflect + Mul<Output = T> + Copy + 'static>(
            registry: &TypeRegistry,
            mut a: T,
            b: T,
            result: T,
        ) {
            let reflect_mul = registry
                .get_type_data::<ReflectMul>(TypeId::of::<T>())
                .unwrap();
            let reflect_mul_assign = registry
                .get_type_data::<ReflectMulAssign>(TypeId::of::<T>())
                .unwrap();

            assert_eq!(
                reflect_mul
                    .mul(Box::new(a), Box::new(b))
                    .unwrap()
                    .reflect_partial_eq(&result),
                Some(true)
            );
            reflect_mul_assign.mul_assign(&mut a, Box::new(b)).unwrap();
            assert_eq!(a.reflect_partial_eq(&result), Some(true));

            assert!(reflect_mul.mul(Box::new(a), Box::new("not a T")).is_err());
            assert!(reflect_mul.mul(Box::new("not a T"), Box::new(b)).is_err());
            assert!(reflect_mul_assign
                .mul_assign(&mut a, Box::new("not a T"))
                .is_err());
            assert!(reflect_mul_assign
                .mul_assign(&mut "not a T", Box::new(b))
                .is_err());
        }

        let registry = TypeRegistry::new();

        check_mul::<u8>(&registry, 10, 5, 50);
        check_mul::<u16>(&registry, 10, 5, 50);
        check_mul::<u32>(&registry, 10, 5, 50);
        check_mul::<u64>(&registry, 10, 5, 50);
        check_mul::<u128>(&registry, 10, 5, 50);
        check_mul::<usize>(&registry, 10, 5, 50);
        check_mul::<i8>(&registry, 10, 5, 50);
        check_mul::<i16>(&registry, 10, 5, 50);
        check_mul::<i32>(&registry, 10, 5, 50);
        check_mul::<i64>(&registry, 10, 5, 50);
        check_mul::<i128>(&registry, 10, 5, 50);
        check_mul::<isize>(&registry, 10, 5, 50);
        check_mul::<f32>(&registry, 5., 2., 10.);
        check_mul::<f64>(&registry, 5., 2., 10.);
    }

    #[test]
    fn should_div() {
        fn check_div<T: Reflect + Div<Output = T> + Copy + 'static>(
            registry: &TypeRegistry,
            mut a: T,
            b: T,
            result: T,
        ) {
            let reflect_div = registry
                .get_type_data::<ReflectDiv>(TypeId::of::<T>())
                .unwrap();
            let reflect_div_assign = registry
                .get_type_data::<ReflectDivAssign>(TypeId::of::<T>())
                .unwrap();

            assert_eq!(
                reflect_div
                    .div(Box::new(a), Box::new(b))
                    .unwrap()
                    .reflect_partial_eq(&result),
                Some(true)
            );
            reflect_div_assign.div_assign(&mut a, Box::new(b)).unwrap();
            assert_eq!(a.reflect_partial_eq(&result), Some(true));

            assert!(reflect_div.div(Box::new(a), Box::new("not a T")).is_err());
            assert!(reflect_div.div(Box::new("not a T"), Box::new(b)).is_err());
            assert!(reflect_div_assign
                .div_assign(&mut a, Box::new("not a T"))
                .is_err());
            assert!(reflect_div_assign
                .div_assign(&mut "not a T", Box::new(b))
                .is_err());
        }

        let registry = TypeRegistry::new();

        check_div::<u8>(&registry, 10, 5, 2);
        check_div::<u16>(&registry, 10, 5, 2);
        check_div::<u32>(&registry, 10, 5, 2);
        check_div::<u64>(&registry, 10, 5, 2);
        check_div::<u128>(&registry, 10, 5, 2);
        check_div::<usize>(&registry, 10, 5, 2);
        check_div::<i8>(&registry, 10, 5, 2);
        check_div::<i16>(&registry, 10, 5, 2);
        check_div::<i32>(&registry, 10, 5, 2);
        check_div::<i64>(&registry, 10, 5, 2);
        check_div::<i128>(&registry, 10, 5, 2);
        check_div::<isize>(&registry, 10, 5, 2);
        check_div::<f32>(&registry, 10., 2., 5.);
        check_div::<f64>(&registry, 10., 2., 5.);
    }

    #[test]
    fn should_rem() {
        fn check_rem<T: Reflect + Rem<Output = T> + Copy + 'static>(
            registry: &TypeRegistry,
            mut a: T,
            b: T,
            result: T,
        ) {
            let reflect_rem = registry
                .get_type_data::<ReflectRem>(TypeId::of::<T>())
                .unwrap();
            let reflect_rem_assign = registry
                .get_type_data::<ReflectRemAssign>(TypeId::of::<T>())
                .unwrap();

            assert_eq!(
                reflect_rem
                    .rem(Box::new(a), Box::new(b))
                    .unwrap()
                    .reflect_partial_eq(&result),
                Some(true)
            );
            reflect_rem_assign.rem_assign(&mut a, Box::new(b)).unwrap();
            assert_eq!(a.reflect_partial_eq(&result), Some(true));

            assert!(reflect_rem.rem(Box::new(a), Box::new("not a T")).is_err());
            assert!(reflect_rem.rem(Box::new("not a T"), Box::new(b)).is_err());
            assert!(reflect_rem_assign
                .rem_assign(&mut a, Box::new("not a T"))
                .is_err());
            assert!(reflect_rem_assign
                .rem_assign(&mut "not a T", Box::new(b))
                .is_err());
        }

        let registry = TypeRegistry::new();

        check_rem::<u8>(&registry, 10, 5, 0);
        check_rem::<u16>(&registry, 10, 5, 0);
        check_rem::<u32>(&registry, 10, 5, 0);
        check_rem::<u64>(&registry, 10, 5, 0);
        check_rem::<u128>(&registry, 10, 5, 0);
        check_rem::<usize>(&registry, 10, 5, 0);
        check_rem::<i8>(&registry, 10, 5, 0);
        check_rem::<i16>(&registry, 10, 5, 0);
        check_rem::<i32>(&registry, 10, 5, 0);
        check_rem::<i64>(&registry, 10, 5, 0);
        check_rem::<i128>(&registry, 10, 5, 0);
        check_rem::<isize>(&registry, 10, 5, 0);
        check_rem::<f32>(&registry, 10., 5., 0.);
        check_rem::<f64>(&registry, 10., 5., 0.);
    }
}
