use crate::func::args::{ArgInfo, ArgList};
use crate::func::info::FunctionInfo;
use crate::func::{DynamicFunctionMut, FunctionResult, IntoFunction, IntoFunctionMut, ReturnInfo};
use alloc::borrow::Cow;
use core::fmt::{Debug, Formatter};
use std::sync::Arc;

/// A dynamic representation of a function.
///
/// This type can be used to represent any callable that satisfies [`Fn`]
/// (or the reflection-based equivalent, [`ReflectFn`]).
/// That is, any function or closure that does not mutably borrow data from its environment.
///
/// For functions that do need to capture their environment mutably (i.e. mutable closures),
/// see [`DynamicFunctionMut`].
///
/// See the [module-level documentation] for more information.
///
/// You will generally not need to construct this manually.
/// Instead, many functions and closures can be automatically converted using the [`IntoFunction`] trait.
///
/// # Example
///
/// Most of the time, a [`DynamicFunction`] can be created using the [`IntoFunction`] trait:
///
/// ```
/// # use bevy_reflect::func::{ArgList, DynamicFunction, FunctionInfo, IntoFunction};
/// #
/// fn add(a: i32, b: i32) -> i32 {
///   a + b
/// }
///
/// // Convert the function into a dynamic function using `IntoFunction::into_function`:
/// let mut func: DynamicFunction = add.into_function();
///
/// // Dynamically call it:
/// let args = ArgList::default().push_owned(25_i32).push_owned(75_i32);
/// let value = func.call(args).unwrap().unwrap_owned();
///
/// // Check the result:
/// assert_eq!(value.try_downcast_ref::<i32>(), Some(&100));
/// ```
///
/// [`ReflectFn`]: crate::func::ReflectFn
/// [module-level documentation]: crate::func
pub struct DynamicFunction<'env> {
    pub(super) info: FunctionInfo,
    pub(super) func: Arc<dyn for<'a> Fn(ArgList<'a>) -> FunctionResult<'a> + Send + Sync + 'env>,
}

impl<'env> DynamicFunction<'env> {
    /// Create a new [`DynamicFunction`].
    ///
    /// The given function can be used to call out to any other callable,
    /// including functions, closures, or methods.
    ///
    /// It's important that the function signature matches the provided [`FunctionInfo`].
    /// This info may be used by consumers of this function for validation and debugging.
    pub fn new<F: for<'a> Fn(ArgList<'a>) -> FunctionResult<'a> + Send + Sync + 'env>(
        func: F,
        info: FunctionInfo,
    ) -> Self {
        Self {
            info,
            func: Arc::new(func),
        }
    }

    /// Set the name of the function.
    ///
    /// For [`DynamicFunctions`] created using [`IntoFunction`],
    /// the default name will always be the full path to the function as returned by [`std::any::type_name`],
    /// unless the function is a closure, anonymous function, or function pointer,
    /// in which case the name will be `None`.
    ///
    /// [`DynamicFunctions`]: DynamicFunction
    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.info = self.info.with_name(name);
        self
    }

    /// Set the argument information of the function.
    ///
    /// It's important that the arguments match the intended function signature,
    /// as this can be used by consumers of this function for validation and debugging.
    pub fn with_args(mut self, args: Vec<ArgInfo>) -> Self {
        self.info = self.info.with_args(args);
        self
    }

    /// Set the return information of the function.
    pub fn with_return_info(mut self, return_info: ReturnInfo) -> Self {
        self.info = self.info.with_return_info(return_info);
        self
    }

    /// Call the function with the given arguments.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::{IntoFunction, ArgList};
    /// let c = 23;
    /// let add = |a: i32, b: i32| -> i32 {
    ///   a + b + c
    /// };
    ///
    /// let mut func = add.into_function().with_name("add");
    /// let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
    /// let result = func.call(args).unwrap().unwrap_owned();
    /// assert_eq!(result.try_take::<i32>().unwrap(), 123);
    /// ```
    pub fn call<'a>(&self, args: ArgList<'a>) -> FunctionResult<'a> {
        (self.func)(args)
    }

    /// Returns the function info.
    pub fn info(&self) -> &FunctionInfo {
        &self.info
    }

    /// The [name] of the function.
    ///
    /// For [`DynamicFunctions`] created using [`IntoFunction`],
    /// the default name will always be the full path to the function as returned by [`std::any::type_name`],
    /// unless the function is a closure, anonymous function, or function pointer,
    /// in which case the name will be `None`.
    ///
    /// This can be overridden using [`with_name`].
    ///
    /// [name]: FunctionInfo::name
    /// [`DynamicFunctions`]: DynamicFunction
    /// [`with_name`]: Self::with_name
    pub fn name(&self) -> Option<&Cow<'static, str>> {
        self.info.name()
    }
}

/// Outputs the function's signature.
///
/// This takes the format: `DynamicFunction(fn {name}({arg1}: {type1}, {arg2}: {type2}, ...) -> {return_type})`.
///
/// Names for arguments and the function itself are optional and will default to `_` if not provided.
impl<'env> Debug for DynamicFunction<'env> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let name = self.info.name().unwrap_or(&Cow::Borrowed("_"));
        write!(f, "DynamicFunction(fn {name}(")?;

        for (index, arg) in self.info.args().iter().enumerate() {
            let name = arg.name().unwrap_or("_");
            let ty = arg.type_path();
            write!(f, "{name}: {ty}")?;

            if index + 1 < self.info.args().len() {
                write!(f, ", ")?;
            }
        }

        let ret = self.info.return_info().type_path();
        write!(f, ") -> {ret})")
    }
}

impl<'env> Clone for DynamicFunction<'env> {
    fn clone(&self) -> Self {
        Self {
            info: self.info.clone(),
            func: Arc::clone(&self.func),
        }
    }
}

impl<'env> IntoFunction<'env, ()> for DynamicFunction<'env> {
    #[inline]
    fn into_function(self) -> DynamicFunction<'env> {
        self
    }
}

impl<'env> IntoFunctionMut<'env, ()> for DynamicFunction<'env> {
    #[inline]
    fn into_function_mut(self) -> DynamicFunctionMut<'env> {
        DynamicFunctionMut::from(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_overwrite_function_name() {
        let c = 23;
        let func = (|a: i32, b: i32| a + b + c)
            .into_function()
            .with_name("my_function");
        assert_eq!(func.info().name().unwrap(), "my_function");
    }

    #[test]
    fn should_convert_dynamic_function_with_into_function() {
        fn make_closure<'env, F: IntoFunction<'env, M>, M>(f: F) -> DynamicFunction<'env> {
            f.into_function()
        }

        let c = 23;
        let function: DynamicFunction = make_closure(|a: i32, b: i32| a + b + c);
        let _: DynamicFunction = make_closure(function);
    }

    #[test]
    fn should_clone_dynamic_function() {
        let hello = String::from("Hello");

        let greet = |name: &String| -> String { format!("{}, {}!", hello, name) };

        let greet = greet.into_function().with_name("greet");
        let clone = greet.clone();

        assert_eq!(greet.name().unwrap(), "greet");
        assert_eq!(clone.name().unwrap(), "greet");

        let clone_value = clone
            .call(ArgList::default().push_ref(&String::from("world")))
            .unwrap()
            .unwrap_owned()
            .try_take::<String>()
            .unwrap();

        assert_eq!(clone_value, "Hello, world!");
    }
}
