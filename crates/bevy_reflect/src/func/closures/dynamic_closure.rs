use crate::func::args::{ArgInfo, ArgList};
use crate::func::info::FunctionInfo;
use crate::func::{
    DynamicClosureMut, DynamicFunction, FunctionResult, IntoClosure, IntoClosureMut, ReturnInfo,
};
use alloc::borrow::Cow;
use core::fmt::{Debug, Formatter};
use std::sync::Arc;

/// A dynamic representation of a Rust closure.
///
/// This type can be used to represent any Rust closure that captures its environment immutably.
/// For closures that need to capture their environment mutably,
/// see [`DynamicClosureMut`].
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
/// let punct = String::from("!!!");
///
/// let punctuate = |text: &String| -> String {
///   format!("{}{}", text, punct)
/// };
///
/// // Convert the closure into a dynamic closure using `IntoClosure::into_closure`
/// let mut func: DynamicClosure = punctuate.into_closure();
///
/// // Dynamically call the closure:
/// let text = String::from("Hello, world");
/// let args = ArgList::default().push_ref(&text);
/// let value = func.call(args).unwrap().unwrap_owned();
///
/// // Check the result:
/// assert_eq!(value.try_take::<String>().unwrap(), "Hello, world!!!");
/// ```
pub struct DynamicClosure<'env> {
    pub(super) info: FunctionInfo,
    pub(super) func: Arc<dyn for<'a> Fn(ArgList<'a>) -> FunctionResult<'a> + Send + Sync + 'env>,
}

impl<'env> DynamicClosure<'env> {
    /// Create a new [`DynamicClosure`].
    ///
    /// The given function can be used to call out to a regular function, closure, or method.
    ///
    /// It's important that the closure signature matches the provided [`FunctionInfo`].
    /// This info may be used by consumers of the function for validation and debugging.
    pub fn new<F: for<'a> Fn(ArgList<'a>) -> FunctionResult<'a> + Send + Sync + 'env>(
        func: F,
        info: FunctionInfo,
    ) -> Self {
        Self {
            info,
            func: Arc::new(func),
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
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::{IntoClosure, ArgList};
    /// let c = 23;
    /// let add = |a: i32, b: i32| -> i32 {
    ///   a + b + c
    /// };
    ///
    /// let mut func = add.into_closure().with_name("add");
    /// let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
    /// let result = func.call(args).unwrap().unwrap_owned();
    /// assert_eq!(result.try_take::<i32>().unwrap(), 123);
    /// ```
    pub fn call<'a>(&self, args: ArgList<'a>) -> FunctionResult<'a> {
        (self.func)(args)
    }

    /// Returns the closure info.
    pub fn info(&self) -> &FunctionInfo {
        &self.info
    }

    /// The [name] of the closure.
    ///
    /// If this [`DynamicClosure`] was created using [`IntoClosure`],
    /// then the default name will always be `None`.
    ///
    /// This can be overridden using [`with_name`].
    ///
    /// [name]: FunctionInfo::name
    /// [`with_name`]: Self::with_name
    pub fn name(&self) -> Option<&Cow<'static, str>> {
        self.info.name()
    }
}

/// Outputs the closure's signature.
///
/// This takes the format: `DynamicClosure(fn {name}({arg1}: {type1}, {arg2}: {type2}, ...) -> {return_type})`.
///
/// Names for arguments and the closure itself are optional and will default to `_` if not provided.
impl<'env> Debug for DynamicClosure<'env> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let name = self.info.name().unwrap_or(&Cow::Borrowed("_"));
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

impl<'env> Clone for DynamicClosure<'env> {
    fn clone(&self) -> Self {
        Self {
            info: self.info.clone(),
            func: Arc::clone(&self.func),
        }
    }
}

impl From<DynamicFunction> for DynamicClosure<'static> {
    #[inline]
    fn from(func: DynamicFunction) -> Self {
        Self {
            info: func.info,
            func: func.func,
        }
    }
}

impl<'env> IntoClosure<'env, ()> for DynamicClosure<'env> {
    #[inline]
    fn into_closure(self) -> DynamicClosure<'env> {
        self
    }
}

impl<'env> IntoClosureMut<'env, ()> for DynamicClosure<'env> {
    #[inline]
    fn into_closure_mut(self) -> DynamicClosureMut<'env> {
        DynamicClosureMut::from(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_overwrite_closure_name() {
        let c = 23;
        let func = (|a: i32, b: i32| a + b + c)
            .into_closure()
            .with_name("my_closure");
        assert_eq!(func.info().name().unwrap(), "my_closure");
    }

    #[test]
    fn should_convert_dynamic_closure_with_into_closure() {
        fn make_closure<'env, F: IntoClosure<'env, M>, M>(f: F) -> DynamicClosure<'env> {
            f.into_closure()
        }

        let c = 23;
        let closure: DynamicClosure = make_closure(|a: i32, b: i32| a + b + c);
        let _: DynamicClosure = make_closure(closure);
    }

    #[test]
    fn should_clone_dynamic_closure() {
        let hello = String::from("Hello");

        let greet = |name: &String| -> String { format!("{}, {}!", hello, name) };

        let greet = greet.into_closure().with_name("greet");
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
