use crate::{
    self as bevy_reflect,
    __macro_exports::RegisterForReflection,
    func::{
        args::ArgList, info::FunctionInfo, DynamicFunctionMut, Function, FunctionError,
        FunctionResult, IntoFunction, IntoFunctionMut,
    },
    serde::Serializable,
    ApplyError, MaybeTyped, PartialReflect, Reflect, ReflectKind, ReflectMut, ReflectOwned,
    ReflectRef, TypeInfo, TypePath,
};
use alloc::{borrow::Cow, sync::Arc};
use bevy_reflect_derive::impl_type_path;
use core::fmt::{Debug, Formatter};

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
    /// It's important that the function signature matches the provided [`FunctionInfo`]
    /// as this will be used to validate arguments when [calling] the function.
    ///
    /// [calling]: DynamicFunction::call
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
    ///
    /// # Errors
    ///
    /// This method will return an error if the number of arguments provided does not match
    /// the number of arguments expected by the function's [`FunctionInfo`].
    ///
    /// The function itself may also return any errors it needs to.
    pub fn call<'a>(&self, args: ArgList<'a>) -> FunctionResult<'a> {
        let expected_arg_count = self.info.arg_count();
        let received_arg_count = args.len();

        if expected_arg_count != received_arg_count {
            Err(FunctionError::ArgCountMismatch {
                expected: expected_arg_count,
                received: received_arg_count,
            })
        } else {
            (self.func)(args)
        }
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

impl Function for DynamicFunction<'static> {
    fn info(&self) -> &FunctionInfo {
        self.info()
    }

    fn reflect_call<'a>(&self, args: ArgList<'a>) -> FunctionResult<'a> {
        self.call(args)
    }

    fn clone_dynamic(&self) -> DynamicFunction<'static> {
        self.clone()
    }
}

impl PartialReflect for DynamicFunction<'static> {
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        None
    }

    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }

    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self
    }

    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self
    }

    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        Err(self)
    }

    fn try_as_reflect(&self) -> Option<&dyn Reflect> {
        None
    }

    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        None
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        match value.reflect_ref() {
            ReflectRef::Function(func) => {
                *self = func.clone_dynamic();
                Ok(())
            }
            _ => Err(ApplyError::MismatchedTypes {
                from_type: value.reflect_type_path().into(),
                to_type: Self::type_path().into(),
            }),
        }
    }

    fn reflect_kind(&self) -> ReflectKind {
        ReflectKind::Function
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Function(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Function(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Function(self)
    }

    fn clone_value(&self) -> Box<dyn PartialReflect> {
        Box::new(self.clone())
    }

    fn reflect_hash(&self) -> Option<u64> {
        None
    }

    fn reflect_partial_eq(&self, _value: &dyn PartialReflect) -> Option<bool> {
        None
    }

    fn debug(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Debug::fmt(self, f)
    }

    fn serializable(&self) -> Option<Serializable> {
        None
    }

    fn is_dynamic(&self) -> bool {
        true
    }
}

impl MaybeTyped for DynamicFunction<'static> {}
impl RegisterForReflection for DynamicFunction<'static> {}

impl_type_path!((in bevy_reflect) DynamicFunction<'env>);

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
    use crate::func::IntoReturn;

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
    fn should_return_error_on_arg_count_mismatch() {
        let func = (|a: i32, b: i32| a + b).into_function();

        let args = ArgList::default().push_owned(25_i32);
        let error = func.call(args).unwrap_err();
        assert!(matches!(
            error,
            FunctionError::ArgCountMismatch {
                expected: 2,
                received: 1
            }
        ));
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

    #[test]
    fn should_apply_function() {
        let mut func: Box<dyn Function> = Box::new((|a: i32, b: i32| a + b).into_function());
        func.apply(&((|a: i32, b: i32| a * b).into_function()));

        let args = ArgList::new().push_owned(5_i32).push_owned(5_i32);
        let result = func.reflect_call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_take::<i32>().unwrap(), 25);
    }

    #[test]
    fn should_allow_recursive_dynamic_function() {
        let factorial = DynamicFunction::new(
            |mut args| {
                let curr = args.pop::<i32>()?;
                if curr == 0 {
                    return Ok(1_i32.into_return());
                }

                let arg = args.pop_arg()?;
                let this = arg.value();

                match this.reflect_ref() {
                    ReflectRef::Function(func) => {
                        let result = func.reflect_call(
                            ArgList::new()
                                .push_ref(this.as_partial_reflect())
                                .push_owned(curr - 1),
                        );
                        let value = result.unwrap().unwrap_owned().try_take::<i32>().unwrap();
                        Ok((curr * value).into_return())
                    }
                    _ => panic!("expected function"),
                }
            },
            // The `FunctionInfo` doesn't really matter for this test
            // so we can just give it dummy information.
            FunctionInfo::anonymous()
                .with_arg::<i32>("curr")
                .with_arg::<()>("this"),
        );

        let args = ArgList::new().push_ref(&factorial).push_owned(5_i32);
        let value = factorial.call(args).unwrap().unwrap_owned();
        assert_eq!(value.try_take::<i32>().unwrap(), 120);
    }
}
