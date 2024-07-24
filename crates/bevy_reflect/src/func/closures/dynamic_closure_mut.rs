use alloc::borrow::Cow;
use core::fmt::{Debug, Formatter};

use crate::func::args::{ArgInfo, ArgList};
use crate::func::info::FunctionInfo;
use crate::func::{FunctionResult, IntoClosureMut, ReturnInfo};

/// A dynamic representation of a Rust closure.
///
/// This type can be used to represent any Rust closure that captures its environment mutably.
/// For closures that only need to capture their environment immutably,
/// consider using [`DynamicClosure`].
///
/// This type can be seen as a superset of [`DynamicClosure`].
///
/// See the [module-level documentation] for more information.
///
/// You will generally not need to construct this manually.
/// Instead, many functions and closures can be automatically converted using the [`IntoClosureMut`] trait.
///
/// # Example
///
/// Most of the time, a [`DynamicClosureMut`] can be created using the [`IntoClosureMut`] trait:
///
/// ```
/// # use bevy_reflect::func::{ArgList, DynamicClosureMut, FunctionInfo, IntoClosureMut};
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
/// // Convert the closure into a dynamic closure using `IntoClosureMut::into_closure_mut`
/// let mut func: DynamicClosureMut = replace.into_closure_mut();
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
/// // Alternatively, we could have called the `func` using
/// // `DynamicClosureMut::call_once` to immediately consume the closure.
/// drop(func);
/// assert_eq!(list, vec![1, -2, 3]);
/// ```
///
/// [`DynamicClosure`]: crate::func::closures::DynamicClosure
/// [`DynamicFunction`]: crate::func::DynamicFunction
pub struct DynamicClosureMut<'env> {
    info: FunctionInfo,
    func: Box<dyn for<'a> FnMut(ArgList<'a>) -> FunctionResult<'a> + 'env>,
}

impl<'env> DynamicClosureMut<'env> {
    /// Create a new [`DynamicClosureMut`].
    ///
    /// The given function can be used to call out to a regular function, closure, or method.
    ///
    /// It's important that the closure signature matches the provided [`FunctionInfo`].
    /// This info may be used by consumers of the function for validation and debugging.
    pub fn new<F: for<'a> FnMut(ArgList<'a>) -> FunctionResult<'a> + 'env>(
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
    /// For [`DynamicClosureMuts`] created using [`IntoClosureMut`],
    /// the default name will always be the full path to the closure as returned by [`std::any::type_name`].
    ///
    /// This default name generally does not contain the actual name of the closure, only its module path.
    /// It is therefore recommended to set the name manually using this method.
    ///
    /// [`DynamicClosureMuts`]: DynamicClosureMut
    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.info = self.info.with_name(name);
        self
    }

    /// Set the arguments of the closure.
    ///
    /// It's important that the arguments match the intended closure signature,
    /// as this can be used by consumers of the function for validation and debugging.
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
    /// Variables that are captured mutably by this closure
    /// won't be usable until this closure is dropped.
    /// Consider using [`call_once`] if you want to consume the closure
    /// immediately after calling it.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::{IntoClosureMut, ArgList};
    /// let mut total = 0;
    /// let add = |a: i32, b: i32| -> i32 {
    ///   total = a + b;
    ///   total
    /// };
    ///
    /// let mut func = add.into_closure_mut().with_name("add");
    /// let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
    /// let result = func.call(args).unwrap().unwrap_owned();
    /// assert_eq!(result.take::<i32>().unwrap(), 100);
    /// ```
    ///
    /// [`call_once`]: DynamicClosureMut::call_once
    pub fn call<'a>(&mut self, args: ArgList<'a>) -> FunctionResult<'a> {
        (self.func)(args)
    }

    /// Call the closure with the given arguments and consume the closure.
    ///
    /// This is useful for closures that capture their environment mutably
    /// because otherwise any captured variables would still be borrowed by this closure.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::{IntoClosureMut, ArgList};
    /// let mut count = 0;
    /// let increment = |amount: i32| count += amount;
    ///
    /// let increment_function = increment.into_closure_mut();
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

    /// Returns the closure info.
    pub fn info(&self) -> &FunctionInfo {
        &self.info
    }
}

/// Outputs the closure's signature.
///
/// This takes the format: `DynamicClosureMut(fn {name}({arg1}: {type1}, {arg2}: {type2}, ...) -> {return_type})`.
///
/// Names for arguments and the closure itself are optional and will default to `_` if not provided.
impl<'env> Debug for DynamicClosureMut<'env> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let name = self.info.name().unwrap_or("_");
        write!(f, "DynamicClosureMut(fn {name}(")?;

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

impl<'env> IntoClosureMut<'env, ()> for DynamicClosureMut<'env> {
    #[inline]
    fn into_closure_mut(self) -> DynamicClosureMut<'env> {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_overwrite_closure_name() {
        let mut total = 0;
        let func = (|a: i32, b: i32| total = a + b)
            .into_closure_mut()
            .with_name("my_closure");
        assert_eq!(func.info().name(), Some("my_closure"));
    }

    #[test]
    fn should_convert_dynamic_closure_mut_with_into_closure() {
        fn make_closure<'env, F: IntoClosureMut<'env, M>, M>(f: F) -> DynamicClosureMut<'env> {
            f.into_closure_mut()
        }

        let mut total = 0;
        let closure: DynamicClosureMut = make_closure(|a: i32, b: i32| total = a + b);
        let _: DynamicClosureMut = make_closure(closure);
    }
}
