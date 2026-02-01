//! Module containing the [`ReflectDefault`] type.

use alloc::boxed::Box;
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Sub, SubAssign};

use crate::{FromType, Reflect};

/// A struct used to provide the default value of a type.
///
/// A [`ReflectDefault`] for type `T` can be obtained via [`FromType::from_type`].
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

impl<T: Reflect + Default> FromType<T> for ReflectDefault {
    fn from_type() -> Self {
        ReflectDefault {
            default: || Box::<T>::default(),
        }
    }
}

/// A struct used to perform addition on reflected values.
///
/// A [`ReflectAdd`] for type `T` can be obtained via [`FromType::from_type`].
#[derive(Clone)]
pub struct ReflectAdd {
    /// Function pointer implementing [`ReflectAdd::add()`].
    pub add: fn(
        Box<dyn Reflect>,
        Box<dyn Reflect>,
    ) -> Result<Box<dyn Reflect>, (Box<dyn Reflect>, Box<dyn Reflect>)>,
}

impl ReflectAdd {
    /// Adds two reflected values together, returning the result as a new reflected value.
    ///
    /// # Errors
    ///
    /// Returns `Err((a, b))` if the types are incompatible.
    pub fn add(
        &self,
        a: Box<dyn Reflect>,
        b: Box<dyn Reflect>,
    ) -> Result<Box<dyn Reflect>, (Box<dyn Reflect>, Box<dyn Reflect>)> {
        (self.add)(a, b)
    }
}

impl<T: Reflect + Add<Output: Reflect>> FromType<T> for ReflectAdd {
    fn from_type() -> Self {
        ReflectAdd {
            add: |a: Box<dyn Reflect>, b: Box<dyn Reflect>| {
                let (a, b) = match (a.downcast::<T>(), b.downcast::<T>()) {
                    (Ok(a), Ok(b)) => (a, b),
                    (a, b) => {
                        let a = a.map(|a| a as Box<dyn Reflect>).unwrap_or_else(|e| e);
                        let b = b.map(|b| b as Box<dyn Reflect>).unwrap_or_else(|e| e);
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
/// A [`ReflectSub`] for type `T` can be obtained via [`FromType::from_type`].
#[derive(Clone)]
pub struct ReflectSub {
    /// Function pointer implementing [`ReflectSub::sub()`].
    pub sub: fn(
        Box<dyn Reflect>,
        Box<dyn Reflect>,
    ) -> Result<Box<dyn Reflect>, (Box<dyn Reflect>, Box<dyn Reflect>)>,
}

impl ReflectSub {
    /// Subtracts two reflected values, returning the result as a new reflected value.
    ///
    /// # Errors
    ///
    /// Returns `Err((a, b))` if the types are incompatible.
    pub fn sub(
        &self,
        a: Box<dyn Reflect>,
        b: Box<dyn Reflect>,
    ) -> Result<Box<dyn Reflect>, (Box<dyn Reflect>, Box<dyn Reflect>)> {
        (self.sub)(a, b)
    }
}

impl<T: Reflect + Sub<Output: Reflect>> FromType<T> for ReflectSub {
    fn from_type() -> Self {
        ReflectSub {
            sub: |a: Box<dyn Reflect>, b: Box<dyn Reflect>| {
                let (a, b) = match (a.downcast::<T>(), b.downcast::<T>()) {
                    (Ok(a), Ok(b)) => (a, b),
                    (a, b) => {
                        let a = a.map(|a| a as Box<dyn Reflect>).unwrap_or_else(|e| e);
                        let b = b.map(|b| b as Box<dyn Reflect>).unwrap_or_else(|e| e);
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
/// A [`ReflectMul`] for type `T` can be obtained via [`FromType::from_type`].
#[derive(Clone)]
pub struct ReflectMul {
    /// Function pointer implementing [`ReflectMul::mul()`].
    pub mul: fn(
        Box<dyn Reflect>,
        Box<dyn Reflect>,
    ) -> Result<Box<dyn Reflect>, (Box<dyn Reflect>, Box<dyn Reflect>)>,
}

impl ReflectMul {
    /// Multiplies two reflected values, returning the result as a new reflected value.
    ///
    /// # Errors
    ///
    /// Returns `Err((a, b))` if the types are incompatible.
    pub fn mul(
        &self,
        a: Box<dyn Reflect>,
        b: Box<dyn Reflect>,
    ) -> Result<Box<dyn Reflect>, (Box<dyn Reflect>, Box<dyn Reflect>)> {
        (self.mul)(a, b)
    }
}

impl<T: Reflect + Mul<Output: Reflect>> FromType<T> for ReflectMul {
    fn from_type() -> Self {
        ReflectMul {
            mul: |a: Box<dyn Reflect>, b: Box<dyn Reflect>| {
                let (a, b) = match (a.downcast::<T>(), b.downcast::<T>()) {
                    (Ok(a), Ok(b)) => (a, b),
                    (a, b) => {
                        let a = a.map(|a| a as Box<dyn Reflect>).unwrap_or_else(|e| e);
                        let b = b.map(|b| b as Box<dyn Reflect>).unwrap_or_else(|e| e);
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
/// A [`ReflectDiv`] for type `T` can be obtained via [`FromType::from_type`].
#[derive(Clone)]
pub struct ReflectDiv {
    /// Function pointer implementing [`ReflectDiv::div()`].
    pub div: fn(
        Box<dyn Reflect>,
        Box<dyn Reflect>,
    ) -> Result<Box<dyn Reflect>, (Box<dyn Reflect>, Box<dyn Reflect>)>,
}

impl ReflectDiv {
    /// Divides two reflected values, returning the result as a new reflected value.
    ///
    /// # Errors
    ///
    /// Returns `Err((a, b))` if the types are incompatible.
    pub fn div(
        &self,
        a: Box<dyn Reflect>,
        b: Box<dyn Reflect>,
    ) -> Result<Box<dyn Reflect>, (Box<dyn Reflect>, Box<dyn Reflect>)> {
        (self.div)(a, b)
    }
}

impl<T: Reflect + Div<Output: Reflect>> FromType<T> for ReflectDiv {
    fn from_type() -> Self {
        ReflectDiv {
            div: |a: Box<dyn Reflect>, b: Box<dyn Reflect>| {
                let (a, b) = match (a.downcast::<T>(), b.downcast::<T>()) {
                    (Ok(a), Ok(b)) => (a, b),
                    (a, b) => {
                        let a = a.map(|a| a as Box<dyn Reflect>).unwrap_or_else(|e| e);
                        let b = b.map(|b| b as Box<dyn Reflect>).unwrap_or_else(|e| e);
                        return Err((a, b));
                    }
                };
                Ok(Box::new(*a / *b))
            },
        }
    }
}

/// A struct used to perform division on reflected values.
///
/// A [`ReflectDiv`] for type `T` can be obtained via [`FromType::from_type`].
#[derive(Clone)]
pub struct ReflectRem {
    /// Function pointer implementing [`ReflectRem::rem()`].
    pub rem: fn(
        Box<dyn Reflect>,
        Box<dyn Reflect>,
    ) -> Result<Box<dyn Reflect>, (Box<dyn Reflect>, Box<dyn Reflect>)>,
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
        a: Box<dyn Reflect>,
        b: Box<dyn Reflect>,
    ) -> Result<Box<dyn Reflect>, (Box<dyn Reflect>, Box<dyn Reflect>)> {
        (self.rem)(a, b)
    }
}

impl<T: Reflect + Rem<Output: Reflect>> FromType<T> for ReflectRem {
    fn from_type() -> Self {
        ReflectRem {
            rem: |a: Box<dyn Reflect>, b: Box<dyn Reflect>| {
                let (a, b) = match (a.downcast::<T>(), b.downcast::<T>()) {
                    (Ok(a), Ok(b)) => (a, b),
                    (a, b) => {
                        let a = a.map(|a| a as Box<dyn Reflect>).unwrap_or_else(|e| e);
                        let b = b.map(|b| b as Box<dyn Reflect>).unwrap_or_else(|e| e);
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
/// A [`ReflectAddAssign`] for type `T` can be obtained via [`FromType::from_type`].
#[derive(Clone)]
pub struct ReflectAddAssign {
    /// Function pointer implementing [`ReflectAddAssign::add_assign()`].
    pub add_assign: fn(&mut dyn Reflect, Box<dyn Reflect>) -> Result<(), Option<Box<dyn Reflect>>>,
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
        b: Box<dyn Reflect>,
    ) -> Result<(), Option<Box<dyn Reflect>>> {
        (self.add_assign)(a, b)
    }
}

impl<T: Reflect + AddAssign> FromType<T> for ReflectAddAssign {
    fn from_type() -> Self {
        ReflectAddAssign {
            add_assign: |a: &mut dyn Reflect, b: Box<dyn Reflect>| {
                let Some(a) = a.downcast_mut::<T>() else {
                    return Err(None);
                };
                let b = match b.downcast::<T>() {
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
/// A [`ReflectSubAssign`] for type `T` can be obtained via [`FromType::from_type`].
#[derive(Clone)]
pub struct ReflectSubAssign {
    /// Function pointer implementing [`ReflectSubAssign::sub_assign()`].
    pub sub_assign: fn(&mut dyn Reflect, Box<dyn Reflect>) -> Result<(), Option<Box<dyn Reflect>>>,
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
        b: Box<dyn Reflect>,
    ) -> Result<(), Option<Box<dyn Reflect>>> {
        (self.sub_assign)(a, b)
    }
}

impl<T: Reflect + SubAssign> FromType<T> for ReflectSubAssign {
    fn from_type() -> Self {
        ReflectSubAssign {
            sub_assign: |a: &mut dyn Reflect, b: Box<dyn Reflect>| {
                let Some(a) = a.downcast_mut::<T>() else {
                    return Err(None);
                };
                let b = match b.downcast::<T>() {
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
/// A [`ReflectMulAssign`] for type `T` can be obtained via [`FromType::from_type`].
#[derive(Clone)]
pub struct ReflectMulAssign {
    /// Function pointer implementing [`ReflectMulAssign::mul_assign()`].
    pub mul_assign: fn(&mut dyn Reflect, Box<dyn Reflect>) -> Result<(), Option<Box<dyn Reflect>>>,
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
        b: Box<dyn Reflect>,
    ) -> Result<(), Option<Box<dyn Reflect>>> {
        (self.mul_assign)(a, b)
    }
}

impl<T: Reflect + MulAssign> FromType<T> for ReflectMulAssign {
    fn from_type() -> Self {
        ReflectMulAssign {
            mul_assign: |a: &mut dyn Reflect, b: Box<dyn Reflect>| {
                let Some(a) = a.downcast_mut::<T>() else {
                    return Err(None);
                };
                let b = match b.downcast::<T>() {
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
/// A [`ReflectDivAssign`] for type `T` can be obtained via [`FromType::from_type`].
#[derive(Clone)]
pub struct ReflectDivAssign {
    /// Function pointer implementing [`ReflectDivAssign::div_assign()`].
    pub div_assign: fn(&mut dyn Reflect, Box<dyn Reflect>) -> Result<(), Option<Box<dyn Reflect>>>,
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
        b: Box<dyn Reflect>,
    ) -> Result<(), Option<Box<dyn Reflect>>> {
        (self.div_assign)(a, b)
    }
}

impl<T: Reflect + DivAssign> FromType<T> for ReflectDivAssign {
    fn from_type() -> Self {
        ReflectDivAssign {
            div_assign: |a: &mut dyn Reflect, b: Box<dyn Reflect>| {
                let Some(a) = a.downcast_mut::<T>() else {
                    return Err(None);
                };
                let b = match b.downcast::<T>() {
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
/// A [`ReflectRemAssign`] for type `T` can be obtained via [`FromType::from_type`].
#[derive(Clone)]
pub struct ReflectRemAssign {
    /// Function pointer implementing [`ReflectRemAssign::rem_assign()`].
    pub rem_assign: fn(&mut dyn Reflect, Box<dyn Reflect>) -> Result<(), Option<Box<dyn Reflect>>>,
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
        b: Box<dyn Reflect>,
    ) -> Result<(), Option<Box<dyn Reflect>>> {
        (self.rem_assign)(a, b)
    }
}

impl<T: Reflect + RemAssign> FromType<T> for ReflectRemAssign {
    fn from_type() -> Self {
        ReflectRemAssign {
            rem_assign: |a: &mut dyn Reflect, b: Box<dyn Reflect>| {
                let Some(a) = a.downcast_mut::<T>() else {
                    return Err(None);
                };
                let b = match b.downcast::<T>() {
                    Ok(b) => b,
                    Err(b) => return Err(Some(b)),
                };
                *a %= *b;
                Ok(())
            },
        }
    }
}
