use alloc::borrow::Cow;
use core::fmt::{Debug, Formatter};

use crate::func::args::{ArgInfo, ArgList};
use crate::func::info::FunctionInfo;
use crate::func::{FunctionResult, IntoClosure, ReturnInfo};

/// A dynamic representation of a Rust closure.
///
/// For our purposes, a "closure" is just a callable that may reference its environment.
/// This includes any type of Rust function or closure.
///
/// This type can be seen as a superset of [`DynamicFunction`].
///
/// See the [module-level documentation] for more information.
///
/// You will generally not need to construct this manually.
/// Instead, many functions and closures can be automatically converted using the [`IntoClosure`] trait.
///
/// # Example
///
/// Most of the time, a [`DynamicClosure`] can be created using the [`IntoClosure`] trait:
///
/// ```
/// # use bevy_reflect::func::{ArgList, DynamicClosure, FunctionInfo, IntoClosure};
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
/// // Convert the closure into a dynamic closure using `IntoClosure::into_closure`
/// let mut func: DynamicClosure = replace.into_closure();
///
/// // Dynamically call the closure:
/// let args = ArgList::default().push_owned(1_usize).push_owned(-2_i32);
/// let value = func.call(args).unwrap().unwrap_owned();
///
/// // Check the result:
/// assert_eq!(value.take::<i32>().unwrap(), 2);
///
/// // Note that `func` still has a reference to `list`,
/// // so we need to drop it before we can access `list` again.
/// drop(func);
/// assert_eq!(list, vec![1, -2, 3]);
/// ```
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
pub struct DynamicClosure<'env> {
    info: FunctionInfo,
    func: Box<dyn for<'a> FnMut(ArgList<'a>, &FunctionInfo) -> FunctionResult<'a> + 'env>,
}

impl<'env> DynamicClosure<'env> {
    /// Create a new [`DynamicClosure`].
    ///
    /// The given function can be used to call out to a regular function, closure, or method.
    ///
    /// It's important that the closure signature matches the provided [`FunctionInfo`].
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

    /// Set the name of the closure.
    ///
    /// For [`DynamicClosures`] created using [`IntoClosure`],
    /// the default name will always be the full path to the closure as returned by [`std::any::type_name`].
    ///
    /// This default name generally does not contain the actual name of the closure, only its module path.
    /// It is therefore recommended to set the name manually using this method.
    ///
    /// [`DynamicClosures`]: DynamicClosure
    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.info = self.info.with_name(name);
        self
    }

    /// Set the arguments of the closure.
    ///
    /// It is very important that the arguments match the intended closure signature,
    /// as this is used to validate arguments passed to the closure.
    pub fn with_args(mut self, args: Vec<ArgInfo>) -> Self {
        self.info = self.info.with_args(args);
        self
    }

    /// Set the return information of the closure.
    pub fn with_return_info(mut self, return_info: ReturnInfo) -> Self {
        self.info = self.info.with_return_info(return_info);
        self
    }

    /// Call the closure with the given arguments.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::{IntoClosure, ArgList};
    /// let add = |a: i32, b: i32| -> i32 {
    ///   a + b
    /// };
    ///
    /// let mut func = add.into_closure().with_name("add");
    /// let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
    /// let result = func.call(args).unwrap().unwrap_owned();
    /// assert_eq!(result.take::<i32>().unwrap(), 100);
    /// ```
    pub fn call<'a>(&mut self, args: ArgList<'a>) -> FunctionResult<'a> {
        (self.func)(args, &self.info)
    }

    /// Call the closure with the given arguments and consume the closure.
    ///
    /// This is useful for closures that capture their environment because otherwise
    /// any captured variables would still be borrowed by this closure.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::{IntoClosure, ArgList};
    /// let mut count = 0;
    /// let increment = |amount: i32| count += amount;
    ///
    /// let increment_function = increment.into_closure();
    /// let args = ArgList::new().push_owned(5_i32);
    ///
    /// // We need to drop `increment_function` here so that we
    /// // can regain access to `count`.
    /// // `call_once` does this automatically for us.
    /// increment_function.call_once(args).unwrap();
    /// assert_eq!(count, 5);
    /// ```
    pub fn call_once(mut self, args: ArgList) -> FunctionResult {
        (self.func)(args, &self.info)
    }

    /// Returns the closure info.
    pub fn info(&self) -> &FunctionInfo {
        &self.info
    }
}

/// Outputs the closure's signature.
///
/// This takes the format: `DynamicClosure(fn {name}({arg1}: {type1}, {arg2}: {type2}, ...) -> {return_type})`.
///
/// Names for arguments and the closure itself are optional and will default to `_` if not provided.
impl<'env> Debug for DynamicClosure<'env> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let name = self.info.name().unwrap_or("_");
        write!(f, "DynamicClosure(fn {name}(")?;

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

impl<'env> IntoClosure<'env, ()> for DynamicClosure<'env> {
    #[inline]
    fn into_closure(self) -> DynamicClosure<'env> {
        self
    }
}
