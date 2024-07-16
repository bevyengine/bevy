use alloc::borrow::Cow;
use core::fmt::{Debug, Formatter};
use std::sync::Arc;

use crate::func::args::{ArgInfo, ArgList};
use crate::func::info::FunctionInfo;
use crate::func::{FunctionResult, IntoFunction, ReturnInfo};

/// A dynamic representation of a Rust function.
///
/// For our purposes, a "function" is just a callable that may not reference its environment.
///
/// This includes:
/// - Functions and methods defined with the `fn` keyword
/// - Closures that do not capture their environment
/// - Closures that take ownership of captured variables
///
/// To handle closures that capture references to their environment, see [`DynamicClosure`].
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
/// # use bevy_reflect::func::args::ArgList;
/// # use bevy_reflect::func::{DynamicFunction, IntoFunction};
/// fn add(a: i32, b: i32) -> i32 {
///   a + b
/// }
///
/// // Convert the function into a dynamic function using `IntoFunction::into_function`
/// let mut func: DynamicFunction = add.into_function();
///
/// // Dynamically call the function:
/// let args = ArgList::default().push_owned(25_i32).push_owned(75_i32);
/// let value = func.call(args).unwrap().unwrap_owned();
///
/// // Check the result:
/// assert_eq!(value.downcast_ref::<i32>(), Some(&100));
/// ```
///
/// However, in some cases, these functions may need to be created manually:
///
/// ```
/// # use bevy_reflect::func::{ArgList, DynamicFunction, FunctionInfo, IntoFunction, Return, ReturnInfo};
/// # use bevy_reflect::func::args::ArgInfo;
/// fn append(value: String, list: &mut Vec<String>) -> &mut String {
///   list.push(value);
///   list.last_mut().unwrap()
/// }
///
/// // Due to the return value being a reference that is not tied to the first argument,
/// // this will fail to compile:
/// // let mut func: DynamicFunction = append.into_function();
///
/// // Instead, we need to define the function manually.
/// // We start by defining the shape of the function:
/// let info = FunctionInfo::new()
///   .with_name("append")
///   .with_args(vec![
///     ArgInfo::new::<String>(0).with_name("value"),
///     ArgInfo::new::<&mut Vec<String>>(1).with_name("list"),
///   ])
///   .with_return_info(
///     ReturnInfo::new::<&mut String>()
///   );
///
/// // Then we define the dynamic function, which will be used to call our `append` function:
/// let mut func = DynamicFunction::new(|mut args, info| {
///   // Arguments are popped from the list in reverse order:
///   let arg1 = args.pop().unwrap().take_mut::<Vec<String>>(&info.args()[1]).unwrap();
///   let arg0 = args.pop().unwrap().take_owned::<String>(&info.args()[0]).unwrap();
///
///   // Then we can call our function and return the result:
///   Ok(Return::Mut(append(arg0, arg1)))
/// }, info);
///
/// let mut list = Vec::<String>::new();
///
/// // Dynamically call the function:
/// let args = ArgList::default().push_owned("Hello, World".to_string()).push_mut(&mut list);
/// let value = func.call(args).unwrap().unwrap_mut();
///
/// // Mutate the return value:
/// value.downcast_mut::<String>().unwrap().push_str("!!!");
///
/// // Check the result:
/// assert_eq!(list, vec!["Hello, World!!!"]);
/// ```
///
/// [`DynamicClosure`]: crate::func::DynamicClosure
/// [module-level documentation]: crate::func
pub struct DynamicFunction {
    info: FunctionInfo,
    func: Arc<dyn for<'a> Fn(ArgList<'a>, &FunctionInfo) -> FunctionResult<'a> + 'static>,
}

impl DynamicFunction {
    /// Create a new dynamic [`DynamicFunction`].
    ///
    /// The given function can be used to call out to a regular function, closure, or method.
    ///
    /// It's important that the function signature matches the provided [`FunctionInfo`].
    /// This info is used to validate the arguments and return value.
    pub fn new<F: for<'a> Fn(ArgList<'a>, &FunctionInfo) -> FunctionResult<'a> + 'static>(
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
    /// the default name will always be the full path to the function as returned by [`std::any::type_name`].
    ///
    /// [`DynamicFunctions`]: DynamicFunction
    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.info = self.info.with_name(name);
        self
    }

    /// Set the arguments of the function.
    ///
    /// It is very important that the arguments match the intended function signature,
    /// as this is used to validate arguments passed to the function.
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
    /// fn add(a: i32, b: i32) -> i32 {
    ///   a + b
    /// }
    ///
    /// let func = add.into_function();
    /// let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
    /// let result = func.call(args).unwrap().unwrap_owned();
    /// assert_eq!(result.take::<i32>().unwrap(), 100);
    /// ```
    pub fn call<'a>(&self, args: ArgList<'a>) -> FunctionResult<'a> {
        (self.func)(args, &self.info)
    }

    /// Returns the function info.
    pub fn info(&self) -> &FunctionInfo {
        &self.info
    }
}

/// Outputs the function signature.
///
/// This takes the format: `DynamicFunction(fn {name}({arg1}: {type1}, {arg2}: {type2}, ...) -> {return_type})`.
///
/// Names for arguments and the function itself are optional and will default to `_` if not provided.
impl Debug for DynamicFunction {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let name = self.info.name().unwrap_or("_");
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

impl Clone for DynamicFunction {
    fn clone(&self) -> Self {
        Self {
            info: self.info.clone(),
            func: Arc::clone(&self.func),
        }
    }
}

impl IntoFunction<()> for DynamicFunction {
    #[inline]
    fn into_function(self) -> DynamicFunction {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_overwrite_function_name() {
        fn foo() {}

        let func = foo.into_function().with_name("my_function");
        assert_eq!(func.info().name(), Some("my_function"));
    }

    #[test]
    fn should_convert_dynamic_function_with_into_function() {
        fn make_function<F: IntoFunction<M>, M>(f: F) -> DynamicFunction {
            f.into_function()
        }

        let function: DynamicFunction = make_function(|| {});
        let _: DynamicFunction = make_function(function);
    }
}
