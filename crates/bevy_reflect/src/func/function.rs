use crate::{
    func::{
        args::{ArgCount, ArgList},
        DynamicFunction, FunctionInfo, FunctionResult,
    },
    PartialReflect,
};
use alloc::borrow::Cow;
use core::fmt::Debug;

/// A trait used to power [function-like] operations via [reflection].
///
/// This trait allows types to be called like regular functions
/// with [`Reflect`]-based [arguments] and return values.
///
/// By default, this trait is currently only implemented for [`DynamicFunction`],
/// however, it is possible to implement this trait for custom function-like types.
///
/// # Example
///
/// ```
/// # use bevy_reflect::func::{IntoFunction, ArgList, Function};
/// fn add(a: i32, b: i32) -> i32 {
///    a + b
/// }
///
/// let func: Box<dyn Function> = Box::new(add.into_function());
/// let args = ArgList::new().with_owned(25_i32).with_owned(75_i32);
/// let value = func.reflect_call(args).unwrap().unwrap_owned();
/// assert_eq!(value.try_take::<i32>().unwrap(), 100);
/// ```
///
/// [function-like]: crate::func
/// [reflection]: crate::Reflect
/// [`Reflect`]: crate::Reflect
/// [arguments]: crate::func::args
/// [`DynamicFunction`]: crate::func::DynamicFunction
pub trait Function: PartialReflect + Debug {
    /// The name of the function, if any.
    ///
    /// For [`DynamicFunctions`] created using [`IntoFunction`],
    /// the default name will always be the full path to the function as returned by [`core::any::type_name`],
    /// unless the function is a closure, anonymous function, or function pointer,
    /// in which case the name will be `None`.
    ///
    /// [`DynamicFunctions`]: crate::func::DynamicFunction
    /// [`IntoFunction`]: crate::func::IntoFunction
    fn name(&self) -> Option<&Cow<'static, str>>;

    /// Returns the number of arguments the function expects.
    ///
    /// For [overloaded] functions that can have a variable number of arguments,
    /// this will contain the full set of counts for all signatures.
    ///
    /// [overloaded]: crate::func#overloading-functions
    fn arg_count(&self) -> ArgCount {
        self.info().arg_count()
    }

    /// The [`FunctionInfo`] for this function.
    fn info(&self) -> &FunctionInfo;

    /// Call this function with the given arguments.
    fn reflect_call<'a>(&self, args: ArgList<'a>) -> FunctionResult<'a>;

    /// Creates a new [`DynamicFunction`] from this function.
    fn to_dynamic_function(&self) -> DynamicFunction<'static>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::func::IntoFunction;
    use alloc::boxed::Box;

    #[test]
    fn should_call_dyn_function() {
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        let func: Box<dyn Function> = Box::new(add.into_function());
        let args = ArgList::new().with_owned(25_i32).with_owned(75_i32);
        let value = func.reflect_call(args).unwrap().unwrap_owned();
        assert_eq!(value.try_take::<i32>().unwrap(), 100);
    }
}
