use alloc::borrow::Cow;
use core::fmt::{Debug, Formatter};

use crate::func::args::{ArgInfo, ArgList};
use crate::func::info::FunctionInfo;
use crate::func::{DynamicFunction, FunctionResult, IntoFunctionMut, ReturnInfo};

/// A dynamic representation of a function.
///
/// This type can be used to represent any callable that satisfies [`FnMut`]
/// (or the reflection-based equivalent, [`ReflectFnMut`]).
/// That is, any function or closure.
///
/// For functions that do not need to capture their environment mutably,
/// it's recommended to use [`DynamicFunction`] instead.
///
/// This type can be seen as a superset of [`DynamicFunction`].
///
/// See the [module-level documentation] for more information.
///
/// You will generally not need to construct this manually.
/// Instead, many functions and closures can be automatically converted using the [`IntoFunctionMut`] trait.
///
/// # Example
///
/// Most of the time, a [`DynamicFunctionMut`] can be created using the [`IntoFunctionMut`] trait:
///
/// ```
/// # use bevy_reflect::func::{ArgList, DynamicFunctionMut, FunctionInfo, IntoFunctionMut};
/// #
/// let mut list: Vec<i32> = vec![1, 2, 3];
///
/// // `replace` is a closure that captures a mutable reference to `list`
/// let mut replace = |index: usize, value: i32| -> i32 {
///   let old_value = list[index];
///   list[index] = value;
///   old_value
/// };
///
/// // Since this closure mutably borrows data, we can't convert it into a regular `DynamicFunction`,
/// // as doing so would result in a compile-time error:
/// // let mut func: DynamicFunction = replace.into_function();
///
/// // Instead, we convert it into a `DynamicFunctionMut` using `IntoFunctionMut::into_function_mut`:
/// let mut func: DynamicFunctionMut = replace.into_function_mut();
///
/// // Dynamically call it:
/// let args = ArgList::default().push_owned(1_usize).push_owned(-2_i32);
/// let value = func.call(args).unwrap().unwrap_owned();
///
/// // Check the result:
/// assert_eq!(value.try_take::<i32>().unwrap(), 2);
///
/// // Note that `func` still has a reference to `list`,
/// // so we need to drop it before we can access `list` again.
/// // Alternatively, we could have invoked `func` with
/// // `DynamicFunctionMut::call_once` to immediately consume it.
/// drop(func);
/// assert_eq!(list, vec![1, -2, 3]);
/// ```
///
/// [`ReflectFnMut`]: crate::func::ReflectFnMut
/// [module-level documentation]: crate::func
pub struct DynamicFunctionMut<'env> {
    info: FunctionInfo,
    func: Box<dyn for<'a> FnMut(ArgList<'a>) -> FunctionResult<'a> + 'env>,
}

impl<'env> DynamicFunctionMut<'env> {
    /// Create a new [`DynamicFunctionMut`].
    ///
    /// The given function can be used to call out to any other callable,
    /// including functions, closures, or methods.
    ///
    /// It's important that the function signature matches the provided [`FunctionInfo`].
    /// This info may be used by consumers of this function for validation and debugging.
    pub fn new<F: for<'a> FnMut(ArgList<'a>) -> FunctionResult<'a> + 'env>(
        func: F,
        info: FunctionInfo,
    ) -> Self {
        Self {
            info,
            func: Box::new(func),
        }
    }

    /// Set the name of the function.
    ///
    /// For [`DynamicFunctionMuts`] created using [`IntoFunctionMut`],
    /// the default name will always be the full path to the function as returned by [`std::any::type_name`],
    /// unless the function is a closure, anonymous function, or function pointer,
    /// in which case the name will be `None`.
    ///
    /// [`DynamicFunctionMuts`]: DynamicFunctionMut
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
    /// Variables that are captured mutably by this function
    /// won't be usable until this function is dropped.
    /// Consider using [`call_once`] if you want to consume the function
    /// immediately after calling it.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::{IntoFunctionMut, ArgList};
    /// let mut total = 0;
    /// let add = |a: i32, b: i32| -> i32 {
    ///   total = a + b;
    ///   total
    /// };
    ///
    /// let mut func = add.into_function_mut().with_name("add");
    /// let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
    /// let result = func.call(args).unwrap().unwrap_owned();
    /// assert_eq!(result.try_take::<i32>().unwrap(), 100);
    /// ```
    ///
    /// [`call_once`]: DynamicFunctionMut::call_once
    pub fn call<'a>(&mut self, args: ArgList<'a>) -> FunctionResult<'a> {
        (self.func)(args)
    }

    /// Call the function with the given arguments and consume it.
    ///
    /// This is useful for functions that capture their environment mutably
    /// because otherwise any captured variables would still be borrowed by it.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::{IntoFunctionMut, ArgList};
    /// let mut count = 0;
    /// let increment = |amount: i32| count += amount;
    ///
    /// let increment_function = increment.into_function_mut();
    /// let args = ArgList::new().push_owned(5_i32);
    ///
    /// // We need to drop `increment_function` here so that we
    /// // can regain access to `count`.
    /// // `call_once` does this automatically for us.
    /// increment_function.call_once(args).unwrap();
    /// assert_eq!(count, 5);
    /// ```
    pub fn call_once(mut self, args: ArgList) -> FunctionResult {
        (self.func)(args)
    }

    /// Returns the function info.
    pub fn info(&self) -> &FunctionInfo {
        &self.info
    }

    /// The [name] of the function.
    ///
    /// For [`DynamicFunctionMuts`] created using [`IntoFunctionMut`],
    /// the default name will always be the full path to the function as returned by [`std::any::type_name`],
    /// unless the function is a closure, anonymous function, or function pointer,
    /// in which case the name will be `None`.
    ///
    /// This can be overridden using [`with_name`].
    ///
    /// [name]: FunctionInfo::name
    /// [`DynamicFunctionMuts`]: DynamicFunctionMut
    /// [`with_name`]: Self::with_name
    pub fn name(&self) -> Option<&Cow<'static, str>> {
        self.info.name()
    }
}

/// Outputs the function's signature.
///
/// This takes the format: `DynamicFunctionMut(fn {name}({arg1}: {type1}, {arg2}: {type2}, ...) -> {return_type})`.
///
/// Names for arguments and the function itself are optional and will default to `_` if not provided.
impl<'env> Debug for DynamicFunctionMut<'env> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let name = self.info.name().unwrap_or(&Cow::Borrowed("_"));
        write!(f, "DynamicFunctionMut(fn {name}(")?;

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

impl<'env> From<DynamicFunction<'env>> for DynamicFunctionMut<'env> {
    #[inline]
    fn from(function: DynamicFunction<'env>) -> Self {
        Self {
            info: function.info,
            func: Box::new(move |args| (function.func)(args)),
        }
    }
}

impl<'env> IntoFunctionMut<'env, ()> for DynamicFunctionMut<'env> {
    #[inline]
    fn into_function_mut(self) -> DynamicFunctionMut<'env> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_overwrite_function_name() {
        let mut total = 0;
        let func = (|a: i32, b: i32| total = a + b)
            .into_function_mut()
            .with_name("my_function");
        assert_eq!(func.info().name().unwrap(), "my_function");
    }

    #[test]
    fn should_convert_dynamic_function_mut_with_into_function() {
        fn make_closure<'env, F: IntoFunctionMut<'env, M>, M>(f: F) -> DynamicFunctionMut<'env> {
            f.into_function_mut()
        }

        let mut total = 0;
        let closure: DynamicFunctionMut = make_closure(|a: i32, b: i32| total = a + b);
        let _: DynamicFunctionMut = make_closure(closure);
    }
}
