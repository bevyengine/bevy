//! Module containing the [`ReflectDefault`] type.

use alloc::boxed::Box;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Sub, SubAssign};

use crate::{CreateTypeData, PartialReflect, Reflect};

/// A struct used to provide the default value of a type.
///
/// A [`ReflectDefault`] for type `T` can be obtained via [`CreateTypeData::create_type_data`].
#[derive(Clone)]
pub struct ReflectDefault {
    default: fn() -> Box<dyn Reflect>,
}

impl ReflectDefault {
    /// Returns the default value for a type.
    pub fn default(&self) -> Box<dyn Reflect> {
        (self.default)()
    }
}

impl<T: Reflect + Default> CreateTypeData<T> for ReflectDefault {
    fn create_type_data(_input: ()) -> Self {
        ReflectDefault {
            default: || Box::<T>::default(),
        }
    }
}

/// A struct used to perform addition on reflected values.
///
/// A [`ReflectAdd`] for type `T` can be obtained via [`CreateTypeData::create_type_data`].
#[derive(Clone)]
pub struct ReflectAdd {
    /// Function pointer implementing [`ReflectAdd::add()`].
    pub add: fn(
        Box<dyn PartialReflect>,
        Box<dyn PartialReflect>,
    )
        -> Result<Box<dyn Reflect>, (Box<dyn PartialReflect>, Box<dyn PartialReflect>)>,
}

impl ReflectAdd {
    /// Adds two reflected values together, returning the result as a new reflected value.
    ///
    /// # Errors
    ///
    /// Returns `Err((a, b))` if the types are incompatible.
    pub fn add(
        &self,
        a: Box<dyn PartialReflect>,
        b: Box<dyn PartialReflect>,
    ) -> Result<Box<dyn Reflect>, (Box<dyn PartialReflect>, Box<dyn PartialReflect>)> {
        (self.add)(a, b)
    }
}

impl<T: Reflect + Add<Output: Reflect>> CreateTypeData<T> for ReflectAdd {
    fn create_type_data(_input: ()) -> Self {
        ReflectAdd {
            add: |a: Box<dyn PartialReflect>, b: Box<dyn PartialReflect>| {
                let (a, b) = match (a.try_downcast::<T>(), b.try_downcast::<T>()) {
                    (Ok(a), Ok(b)) => (a, b),
                    (a, b) => {
                        let a = a
                            .map(|a| a as Box<dyn PartialReflect>)
                            .unwrap_or_else(|e| e);
                        let b = b
                            .map(|b| b as Box<dyn PartialReflect>)
                            .unwrap_or_else(|e| e);
                        return Err((a, b));
                    }
                };
                Ok(Box::new(*a + *b))
            },
        }
    }
}

/// A struct used to perform subtraction on reflected values.
///
/// A [`ReflectSub`] for type `T` can be obtained via [`CreateTypeData::create_type_data`].
#[derive(Clone)]
pub struct ReflectSub {
    /// Function pointer implementing [`ReflectSub::sub()`].
    pub sub: fn(
        Box<dyn PartialReflect>,
        Box<dyn PartialReflect>,
    )
        -> Result<Box<dyn Reflect>, (Box<dyn PartialReflect>, Box<dyn PartialReflect>)>,
}

impl ReflectSub {
    /// Subtracts two reflected values, returning the result as a new reflected value.
    ///
    /// # Errors
    ///
    /// Returns `Err((a, b))` if the types are incompatible.
    pub fn sub(
        &self,
        a: Box<dyn PartialReflect>,
        b: Box<dyn PartialReflect>,
    ) -> Result<Box<dyn Reflect>, (Box<dyn PartialReflect>, Box<dyn PartialReflect>)> {
        (self.sub)(a, b)
    }
}

impl<T: Reflect + Sub<Output: Reflect>> CreateTypeData<T> for ReflectSub {
    fn create_type_data(_input: ()) -> Self {
        ReflectSub {
            sub: |a: Box<dyn PartialReflect>, b: Box<dyn PartialReflect>| {
                let (a, b) = match (a.try_downcast::<T>(), b.try_downcast::<T>()) {
                    (Ok(a), Ok(b)) => (a, b),
                    (a, b) => {
                        let a = a
                            .map(|a| a as Box<dyn PartialReflect>)
                            .unwrap_or_else(|e| e);
                        let b = b
                            .map(|b| b as Box<dyn PartialReflect>)
                            .unwrap_or_else(|e| e);
                        return Err((a, b));
                    }
                };
                Ok(Box::new(*a - *b))
            },
        }
    }
}

/// A struct used to perform multiplication on reflected values.
///
/// A [`ReflectMul`] for type `T` can be obtained via [`CreateTypeData::create_type_data`].
#[derive(Clone)]
pub struct ReflectMul {
    /// Function pointer implementing [`ReflectMul::mul()`].
    pub mul: fn(
        Box<dyn PartialReflect>,
        Box<dyn PartialReflect>,
    )
        -> Result<Box<dyn Reflect>, (Box<dyn PartialReflect>, Box<dyn PartialReflect>)>,
}

impl ReflectMul {
    /// Multiplies two reflected values, returning the result as a new reflected value.
    ///
    /// # Errors
    ///
    /// Returns `Err((a, b))` if the types are incompatible.
    pub fn mul(
        &self,
        a: Box<dyn PartialReflect>,
        b: Box<dyn PartialReflect>,
    ) -> Result<Box<dyn Reflect>, (Box<dyn PartialReflect>, Box<dyn PartialReflect>)> {
        (self.mul)(a, b)
    }
}

impl<T: Reflect + Mul<Output: Reflect>> CreateTypeData<T> for ReflectMul {
    fn create_type_data(_input: ()) -> Self {
        ReflectMul {
            mul: |a: Box<dyn PartialReflect>, b: Box<dyn PartialReflect>| {
                let (a, b) = match (a.try_downcast::<T>(), b.try_downcast::<T>()) {
                    (Ok(a), Ok(b)) => (a, b),
                    (a, b) => {
                        let a = a
                            .map(|a| a as Box<dyn PartialReflect>)
                            .unwrap_or_else(|e| e);
                        let b = b
                            .map(|b| b as Box<dyn PartialReflect>)
                            .unwrap_or_else(|e| e);
                        return Err((a, b));
                    }
                };
                Ok(Box::new(*a * *b))
            },
        }
    }
}

/// A struct used to perform division on reflected values.
///
/// A [`ReflectDiv`] for type `T` can be obtained via [`CreateTypeData::create_type_data`].
#[derive(Clone)]
pub struct ReflectDiv {
    /// Function pointer implementing [`ReflectDiv::div()`].
    pub div: fn(
        Box<dyn PartialReflect>,
        Box<dyn PartialReflect>,
    )
        -> Result<Box<dyn Reflect>, (Box<dyn PartialReflect>, Box<dyn PartialReflect>)>,
}

impl ReflectDiv {
    /// Divides two reflected values, returning the result as a new reflected value.
    ///
    /// # Errors
    ///
    /// Returns `Err((a, b))` if the types are incompatible.
    pub fn div(
        &self,
        a: Box<dyn PartialReflect>,
        b: Box<dyn PartialReflect>,
    ) -> Result<Box<dyn Reflect>, (Box<dyn PartialReflect>, Box<dyn PartialReflect>)> {
        (self.div)(a, b)
    }
}

impl<T: Reflect + Div<Output: Reflect>> CreateTypeData<T> for ReflectDiv {
    fn create_type_data(_input: ()) -> Self {
        ReflectDiv {
            div: |a: Box<dyn PartialReflect>, b: Box<dyn PartialReflect>| {
                let (a, b) = match (a.try_downcast::<T>(), b.try_downcast::<T>()) {
                    (Ok(a), Ok(b)) => (a, b),
                    (a, b) => {
                        let a = a
                            .map(|a| a as Box<dyn PartialReflect>)
                            .unwrap_or_else(|e| e);
                        let b = b
                            .map(|b| b as Box<dyn PartialReflect>)
                            .unwrap_or_else(|e| e);
                        return Err((a, b));
                    }
                };
                Ok(Box::new(*a / *b))
            },
        }
    }
}

/// A struct used to perform remainder on reflected values.
///
/// A [`ReflectRem`] for type `T` can be obtained via [`CreateTypeData::create_type_data`].
#[derive(Clone)]
pub struct ReflectRem {
    /// Function pointer implementing [`ReflectRem::rem()`].
    pub rem: fn(
        Box<dyn PartialReflect>,
        Box<dyn PartialReflect>,
    )
        -> Result<Box<dyn Reflect>, (Box<dyn PartialReflect>, Box<dyn PartialReflect>)>,
}

impl ReflectRem {
    /// Computes the remainder of two reflected values, returning the result as
    /// a new reflected value.
    ///
    /// # Errors
    ///
    /// Returns `Err((a, b))` if the types are incompatible.
    pub fn rem(
        &self,
        a: Box<dyn PartialReflect>,
        b: Box<dyn PartialReflect>,
    ) -> Result<Box<dyn Reflect>, (Box<dyn PartialReflect>, Box<dyn PartialReflect>)> {
        (self.rem)(a, b)
    }
}

impl<T: Reflect + Rem<Output: Reflect>> CreateTypeData<T> for ReflectRem {
    fn create_type_data(_input: ()) -> Self {
        ReflectRem {
            rem: |a: Box<dyn PartialReflect>, b: Box<dyn PartialReflect>| {
                let (a, b) = match (a.try_downcast::<T>(), b.try_downcast::<T>()) {
                    (Ok(a), Ok(b)) => (a, b),
                    (a, b) => {
                        let a = a
                            .map(|a| a as Box<dyn PartialReflect>)
                            .unwrap_or_else(|e| e);
                        let b = b
                            .map(|b| b as Box<dyn PartialReflect>)
                            .unwrap_or_else(|e| e);
                        return Err((a, b));
                    }
                };
                Ok(Box::new(*a % *b))
            },
        }
    }
}

/// A struct used to perform addition assignment on reflected values.
///
/// A [`ReflectAddAssign`] for type `T` can be obtained via [`CreateTypeData::create_type_data`].
#[derive(Clone)]
pub struct ReflectAddAssign {
    /// Function pointer implementing [`ReflectAddAssign::add_assign()`].
    pub add_assign: fn(
        &mut dyn Reflect,
        Box<dyn PartialReflect>,
    ) -> Result<(), Option<Box<dyn PartialReflect>>>,
}

impl ReflectAddAssign {
    /// Adds a reflected value to another reflected value in place.
    ///
    /// # Errors
    ///
    /// - Returns `Err(None)` if the first argument is of an incompatible type.
    /// - Returns `Err(Some(b))` if the second argument is of an incompatible type.
    pub fn add_assign(
        &self,
        a: &mut dyn Reflect,
        b: Box<dyn PartialReflect>,
    ) -> Result<(), Option<Box<dyn PartialReflect>>> {
        (self.add_assign)(a, b)
    }
}

impl<T: Reflect + AddAssign> CreateTypeData<T> for ReflectAddAssign {
    fn create_type_data(_input: ()) -> Self {
        ReflectAddAssign {
            add_assign: |a: &mut dyn Reflect, b: Box<dyn PartialReflect>| {
                let Some(a) = a.downcast_mut::<T>() else {
                    return Err(None);
                };
                let b = match b.try_downcast::<T>() {
                    Ok(b) => b,
                    Err(b) => return Err(Some(b)),
                };
                *a += *b;
                Ok(())
            },
        }
    }
}

/// A struct used to perform subtraction assignment on reflected values.
///
/// A [`ReflectSubAssign`] for type `T` can be obtained via [`CreateTypeData::create_type_data`].
#[derive(Clone)]
pub struct ReflectSubAssign {
    /// Function pointer implementing [`ReflectSubAssign::sub_assign()`].
    pub sub_assign: fn(
        &mut dyn Reflect,
        Box<dyn PartialReflect>,
    ) -> Result<(), Option<Box<dyn PartialReflect>>>,
}

impl ReflectSubAssign {
    /// Subtracts a reflected value from another reflected value in place.
    ///
    /// # Errors
    ///
    /// - Returns `Err(None)` if the first argument is of an incompatible type.
    /// - Returns `Err(Some(b))` if the second argument is of an incompatible type.
    pub fn sub_assign(
        &self,
        a: &mut dyn Reflect,
        b: Box<dyn PartialReflect>,
    ) -> Result<(), Option<Box<dyn PartialReflect>>> {
        (self.sub_assign)(a, b)
    }
}

impl<T: Reflect + SubAssign> CreateTypeData<T> for ReflectSubAssign {
    fn create_type_data(_input: ()) -> Self {
        ReflectSubAssign {
            sub_assign: |a: &mut dyn Reflect, b: Box<dyn PartialReflect>| {
                let Some(a) = a.downcast_mut::<T>() else {
                    return Err(None);
                };
                let b = match b.try_downcast::<T>() {
                    Ok(b) => b,
                    Err(b) => return Err(Some(b)),
                };
                *a -= *b;
                Ok(())
            },
        }
    }
}

/// A struct used to perform multiplication assignment on reflected values.
///
/// A [`ReflectMulAssign`] for type `T` can be obtained via [`CreateTypeData::create_type_data`].
#[derive(Clone)]
pub struct ReflectMulAssign {
    /// Function pointer implementing [`ReflectMulAssign::mul_assign()`].
    pub mul_assign: fn(
        &mut dyn Reflect,
        Box<dyn PartialReflect>,
    ) -> Result<(), Option<Box<dyn PartialReflect>>>,
}

impl ReflectMulAssign {
    /// Multiplies a reflected value by another reflected value in place.
    ///
    /// # Errors
    ///
    /// - Returns `Err(None)` if the first argument is of an incompatible type.
    /// - Returns `Err(Some(b))` if the second argument is of an incompatible type.
    pub fn mul_assign(
        &self,
        a: &mut dyn Reflect,
        b: Box<dyn PartialReflect>,
    ) -> Result<(), Option<Box<dyn PartialReflect>>> {
        (self.mul_assign)(a, b)
    }
}

impl<T: Reflect + MulAssign> CreateTypeData<T> for ReflectMulAssign {
    fn create_type_data(_input: ()) -> Self {
        ReflectMulAssign {
            mul_assign: |a: &mut dyn Reflect, b: Box<dyn PartialReflect>| {
                let Some(a) = a.downcast_mut::<T>() else {
                    return Err(None);
                };
                let b = match b.try_downcast::<T>() {
                    Ok(b) => b,
                    Err(b) => return Err(Some(b)),
                };
                *a *= *b;
                Ok(())
            },
        }
    }
}

/// A struct used to perform division assignment on reflected values.
///
/// A [`ReflectDivAssign`] for type `T` can be obtained via [`CreateTypeData::create_type_data`].
#[derive(Clone)]
pub struct ReflectDivAssign {
    /// Function pointer implementing [`ReflectDivAssign::div_assign()`].
    pub div_assign: fn(
        &mut dyn Reflect,
        Box<dyn PartialReflect>,
    ) -> Result<(), Option<Box<dyn PartialReflect>>>,
}

impl ReflectDivAssign {
    /// Divides a reflected value by another reflected value in place.
    ///
    /// # Errors
    ///
    /// - Returns `Err(None)` if the first argument is of an incompatible type.
    /// - Returns `Err(Some(b))` if the second argument is of an incompatible type.
    pub fn div_assign(
        &self,
        a: &mut dyn Reflect,
        b: Box<dyn PartialReflect>,
    ) -> Result<(), Option<Box<dyn PartialReflect>>> {
        (self.div_assign)(a, b)
    }
}

impl<T: Reflect + DivAssign> CreateTypeData<T> for ReflectDivAssign {
    fn create_type_data(_input: ()) -> Self {
        ReflectDivAssign {
            div_assign: |a: &mut dyn Reflect, b: Box<dyn PartialReflect>| {
                let Some(a) = a.downcast_mut::<T>() else {
                    return Err(None);
                };
                let b = match b.try_downcast::<T>() {
                    Ok(b) => b,
                    Err(b) => return Err(Some(b)),
                };
                *a /= *b;
                Ok(())
            },
        }
    }
}

/// A struct used to perform remainder assignment on reflected values.
///
/// A [`ReflectRemAssign`] for type `T` can be obtained via [`CreateTypeData::create_type_data`].
#[derive(Clone)]
pub struct ReflectRemAssign {
    /// Function pointer implementing [`ReflectRemAssign::rem_assign()`].
    pub rem_assign: fn(
        &mut dyn Reflect,
        Box<dyn PartialReflect>,
    ) -> Result<(), Option<Box<dyn PartialReflect>>>,
}

impl ReflectRemAssign {
    /// Computes the remainder of a reflected value by another reflected value in place.
    ///
    /// # Errors
    ///
    /// - Returns `Err(None)` if the first argument is of an incompatible type.
    /// - Returns `Err(Some(b))` if the second argument is of an incompatible type.
    pub fn rem_assign(
        &self,
        a: &mut dyn Reflect,
        b: Box<dyn PartialReflect>,
    ) -> Result<(), Option<Box<dyn PartialReflect>>> {
        (self.rem_assign)(a, b)
    }
}

impl<T: Reflect + RemAssign> CreateTypeData<T> for ReflectRemAssign {
    fn create_type_data(_input: ()) -> Self {
        ReflectRemAssign {
            rem_assign: |a: &mut dyn Reflect, b: Box<dyn PartialReflect>| {
                let Some(a) = a.downcast_mut::<T>() else {
                    return Err(None);
                };
                let b = match b.try_downcast::<T>() {
                    Ok(b) => b,
                    Err(b) => return Err(Some(b)),
                };
                *a %= *b;
                Ok(())
            },
        }
    }
}
