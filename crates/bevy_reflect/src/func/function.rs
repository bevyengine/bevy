use crate::func::args::{ArgInfo, ArgList};
use crate::func::error::FuncError;
use crate::func::info::FunctionInfo;
use crate::func::return_type::Return;
use crate::func::ReturnInfo;
use alloc::borrow::Cow;
use core::fmt::{Debug, Formatter};
use std::ops::DerefMut;

/// The result of calling a dynamic [`Function`].
///
/// Returns `Ok(value)` if the function was called successfully,
/// where `value` is the [`Return`] value of the function.
pub type FunctionResult<'a> = Result<Return<'a>, FuncError>;

/// A dynamic representation of a Rust function.
///
/// Internally this stores a function pointer and associated info.
///
/// You will generally not need to construct this manually.
/// Instead, many functions and closures can be automatically converted using the [`IntoFunction`] trait.
///
/// # Example
///
/// ```
/// # use bevy_reflect::func::args::ArgList;
/// # use bevy_reflect::func::{Function, IntoFunction};
/// fn add(a: i32, b: i32) -> i32 {
///   a + b
/// }
///
/// let mut func: Function = add.into_function();
/// let args = ArgList::default().push_owned(25_i32).push_owned(75_i32);
/// let result = func.call(args).unwrap().unwrap_owned();
/// assert_eq!(result.downcast_ref::<i32>(), Some(&100));
/// ```
///
/// [`IntoFunction`]: crate::func::IntoFunction
pub struct Function {
    info: FunctionInfo,
    func: Box<dyn for<'a> FnMut(ArgList<'a>, &FunctionInfo) -> FunctionResult<'a> + 'static>,
}

impl Function {
    /// Create a new dynamic [`Function`].
    ///
    /// The given function can be used to call out to a regular function, closure, or method.
    ///
    /// It's important that the function signature matches the provided [`FunctionInfo`].
    /// This info is used to validate the arguments and return value.
    pub fn new<F: for<'a> FnMut(ArgList<'a>, &FunctionInfo) -> FunctionResult<'a> + 'static>(
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
    pub fn call<'a>(&mut self, args: ArgList<'a>) -> FunctionResult<'a> {
        (self.func.deref_mut())(args, &self.info)
    }
}

/// Outputs the function signature.
///
/// This takes the format: `Function(fn {name}({arg1}: {type1}, {arg2}: {type2}, ...) -> {return_type})`.
///
/// Names for arguments and the function itself are optional and will default to `_` if not provided.
impl Debug for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let name = self.info.name().unwrap_or("_");
        write!(f, "Function(fn {name}(")?;

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
