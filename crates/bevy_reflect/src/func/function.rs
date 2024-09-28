use crate::{
    func::{ArgList, DynamicFunction, FunctionInfo, FunctionResult},
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
/// let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
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
    /// the default name will always be the full path to the function as returned by [`std::any::type_name`],
    /// unless the function is a closure, anonymous function, or function pointer,
    /// in which case the name will be `None`.
    ///
    /// [`DynamicFunctions`]: crate::func::DynamicFunction
    /// [`IntoFunction`]: crate::func::IntoFunction
    fn name(&self) -> Option<&Cow<'static, str>> {
        self.info().name()
    }

    /// The number of arguments this function accepts.
    fn arg_count(&self) -> usize {
        self.info().arg_count()
    }

    /// The [`FunctionInfo`] for this function.
    fn info(&self) -> &FunctionInfo;

    /// Call this function with the given arguments.
    fn reflect_call<'a>(&self, args: ArgList<'a>) -> FunctionResult<'a>;

    /// Clone this function into a [`DynamicFunction`].
    fn clone_dynamic(&self) -> DynamicFunction<'static>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::func::IntoFunction;

    #[test]
    fn should_call_dyn_function() {
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        let func: Box<dyn Function> = Box::new(add.into_function());
        let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
        let value = func.reflect_call(args).unwrap().unwrap_owned();
        assert_eq!(value.try_take::<i32>().unwrap(), 100);
    }
}
