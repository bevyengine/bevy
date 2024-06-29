use crate::func::args::{ArgInfo, ArgList};
use crate::func::error::FunctionError;
use crate::func::info::FunctionInfo;
use crate::func::return_type::Return;
use crate::func::ReturnInfo;
use alloc::borrow::Cow;
use core::fmt::{Debug, Formatter};
use std::ops::DerefMut;

/// The result of calling a dynamic [`DynamicFunction`].
///
/// Returns `Ok(value)` if the function was called successfully,
/// where `value` is the [`Return`] value of the function.
pub type FunctionResult<'a> = Result<Return<'a>, FunctionError>;

/// A dynamic representation of a Rust function.
///
/// Internally this stores a function pointer and associated info.
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
/// [`IntoFunction`]: crate::func::IntoFunction
pub struct DynamicFunction<'env> {
    info: FunctionInfo,
    func: Box<dyn for<'a> FnMut(ArgList<'a>, &FunctionInfo) -> FunctionResult<'a> + 'env>,
}

impl<'env> DynamicFunction<'env> {
    /// Create a new dynamic [`DynamicFunction`].
    ///
    /// The given function can be used to call out to a regular function, closure, or method.
    ///
    /// It's important that the function signature matches the provided [`FunctionInfo`].
    /// This info is used to validate the arguments and return value.
    pub fn new<F: for<'a> FnMut(ArgList<'a>, &FunctionInfo) -> FunctionResult<'a> + 'env>(
        func: F,
        info: FunctionInfo,
    ) -> Self {
        Self {
            info,
            func: Box::new(func),
        }
    }

    /// Set the name of the function.
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
    /// fn add(left: i32, right: i32) -> i32 {
    ///   left + right
    /// }
    ///
    /// let mut func = add.into_function();
    /// let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
    /// let result = func.call(args).unwrap().unwrap_owned();
    /// assert_eq!(result.take::<i32>().unwrap(), 100);
    /// ```
    pub fn call<'a>(&mut self, args: ArgList<'a>) -> FunctionResult<'a> {
        (self.func.deref_mut())(args, &self.info)
    }

    /// Call the function with the given arguments and consume the function.
    ///
    /// This is useful for closures that capture their environment because otherwise
    /// any captured variables would still be borrowed by this function.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::{IntoFunction, ArgList};
    /// let mut count = 0;
    /// let increment = |amount: i32| {
    ///   count += amount;
    /// };
    /// let increment_function = increment.into_function();
    /// let args = ArgList::new().push_owned(5_i32);
    /// // We need to drop `increment_function` here so that we
    /// // can regain access to `count`.
    /// increment_function.call_once(args).unwrap();
    /// assert_eq!(count, 5);
    /// ```
    pub fn call_once(mut self, args: ArgList) -> FunctionResult {
        (self.func.deref_mut())(args, &self.info)
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
impl<'env> Debug for DynamicFunction<'env> {
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
