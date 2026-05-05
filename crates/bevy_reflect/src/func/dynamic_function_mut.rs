use alloc::{borrow::Cow, boxed::Box};
use bevy_platform::sync::Arc;
use core::fmt::{Debug, Formatter};

use crate::func::{
    args::{ArgCount, ArgList},
    dynamic_function_internal::DynamicFunctionInternal,
    DynamicFunction, FunctionInfo, FunctionOverloadError, FunctionResult, IntoFunctionMut,
};

/// A [`Box`] containing a callback to a reflected function.
type BoxFnMut<'env> = Box<dyn for<'a> FnMut(ArgList<'a>) -> FunctionResult<'a> + 'env>;

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
/// let args = ArgList::default().with_owned(1_usize).with_owned(-2_i32);
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
    internal: DynamicFunctionInternal<BoxFnMut<'env>>,
}

impl<'env> DynamicFunctionMut<'env> {
    /// Create a new [`DynamicFunctionMut`].
    ///
    /// The given function can be used to call out to any other callable,
    /// including functions, closures, or methods.
    ///
    /// It's important that the function signature matches the provided [`FunctionInfo`].
    /// as this will be used to validate arguments when [calling] the function.
    /// This is also required in order for [function overloading] to work correctly.
    ///
    /// # Panics
    ///
    /// This function may panic for any of the following reasons:
    /// - No [`SignatureInfo`] is provided.
    /// - A provided [`SignatureInfo`] has more arguments than [`ArgCount::MAX_COUNT`].
    /// - The conversion to [`FunctionInfo`] fails.
    ///
    /// [calling]: crate::func::dynamic_function_mut::DynamicFunctionMut::call
    /// [`SignatureInfo`]: crate::func::SignatureInfo
    /// [function overloading]: Self::with_overload
    pub fn new<F: for<'a> FnMut(ArgList<'a>) -> FunctionResult<'a> + 'env>(
        func: F,
        info: impl TryInto<FunctionInfo, Error: Debug>,
    ) -> Self {
        Self {
            internal: DynamicFunctionInternal::new(Box::new(func), info.try_into().unwrap()),
        }
    }

    /// Set the name of the function.
    ///
    /// For [`DynamicFunctionMuts`] created using [`IntoFunctionMut`],
    /// the default name will always be the full path to the function as returned by [`core::any::type_name`],
    /// unless the function is a closure, anonymous function, or function pointer,
    /// in which case the name will be `None`.
    ///
    /// [`DynamicFunctionMuts`]: DynamicFunctionMut
    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.internal = self.internal.with_name(name);
        self
    }

    /// Add an overload to this function.
    ///
    /// Overloads allow a single [`DynamicFunctionMut`] to represent multiple functions of different signatures.
    ///
    /// This can be used to handle multiple monomorphizations of a generic function
    /// or to allow functions with a variable number of arguments.
    ///
    /// Any functions with the same [argument signature] will be overwritten by the one from the new function, `F`.
    /// For example, if the existing function had the signature `(i32, i32) -> i32`,
    /// and the new function, `F`, also had the signature `(i32, i32) -> i32`,
    /// the one from `F` would replace the one from the existing function.
    ///
    /// Overloaded functions retain the [name] of the original function.
    ///
    /// Note that it may be impossible to overload closures that mutably borrow from their environment
    /// due to Rust's borrowing rules.
    /// However, it's still possible to overload functions that do not capture their environment mutably,
    /// or those that maintain mutually exclusive mutable references to their environment.
    ///
    /// # Panics
    ///
    /// Panics if the function, `F`, contains a signature already found in this function.
    ///
    /// For a non-panicking version, see [`try_with_overload`].
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::IntoFunctionMut;
    /// let mut total_i32 = 0;
    /// let mut add_i32 = |a: i32| total_i32 += a;
    ///
    /// let mut total_f32 = 0.0;
    /// let mut add_f32 = |a: f32| total_f32 += a;
    ///
    /// // Currently, the only generic type `func` supports is `i32`.
    /// let mut func = add_i32.into_function_mut();
    ///
    /// // However, we can add an overload to handle `f32` as well:
    /// func = func.with_overload(add_f32);
    ///
    /// // Test `i32`:
    /// let args = bevy_reflect::func::ArgList::new().with_owned(123_i32);
    /// func.call(args).unwrap();
    ///
    /// // Test `f32`:
    /// let args = bevy_reflect::func::ArgList::new().with_owned(1.23_f32);
    /// func.call(args).unwrap();
    ///
    /// drop(func);
    /// assert_eq!(total_i32, 123);
    /// assert_eq!(total_f32, 1.23);
    /// ```
    ///
    /// [argument signature]: crate::func::signature::ArgumentSignature
    /// [name]: Self::name
    /// [`try_with_overload`]: Self::try_with_overload
    pub fn with_overload<'a, F: IntoFunctionMut<'a, Marker>, Marker>(
        self,
        function: F,
    ) -> DynamicFunctionMut<'a>
    where
        'env: 'a,
    {
        self.try_with_overload(function).unwrap_or_else(|(_, err)| {
            panic!("{}", err);
        })
    }

    /// Attempt to add an overload to this function.
    ///
    /// If the function, `F`, contains a signature already found in this function,
    /// an error will be returned along with the original function.
    ///
    /// For a panicking version, see [`with_overload`].
    ///
    /// [`with_overload`]: Self::with_overload
    pub fn try_with_overload<F: IntoFunctionMut<'env, Marker>, Marker>(
        mut self,
        function: F,
    ) -> Result<Self, (Box<Self>, FunctionOverloadError)> {
        let function = function.into_function_mut();

        match self.internal.merge(function.internal) {
            Ok(_) => Ok(self),
            Err(err) => Err((Box::new(self), err)),
        }
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
    /// let args = ArgList::new().with_owned(25_i32).with_owned(75_i32);
    /// let result = func.call(args).unwrap().unwrap_owned();
    /// assert_eq!(result.try_take::<i32>().unwrap(), 100);
    /// ```
    ///
    /// # Errors
    ///
    /// This method will return an error if the number of arguments provided does not match
    /// the number of arguments expected by the function's [`FunctionInfo`].
    ///
    /// The function itself may also return any errors it needs to.
    ///
    /// [`call_once`]: DynamicFunctionMut::call_once
    pub fn call<'a>(&mut self, args: ArgList<'a>) -> FunctionResult<'a> {
        self.internal.validate_args(&args)?;
        let func = self.internal.get_mut(&args)?;
        func(args)
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
    /// let args = ArgList::new().with_owned(5_i32);
    ///
    /// // We need to drop `increment_function` here so that we
    /// // can regain access to `count`.
    /// // `call_once` does this automatically for us.
    /// increment_function.call_once(args).unwrap();
    /// assert_eq!(count, 5);
    /// ```
    ///
    /// # Errors
    ///
    /// This method will return an error if the number of arguments provided does not match
    /// the number of arguments expected by the function's [`FunctionInfo`].
    ///
    /// The function itself may also return any errors it needs to.
    pub fn call_once(mut self, args: ArgList) -> FunctionResult {
        self.call(args)
    }

    /// Returns the function info.
    pub fn info(&self) -> &FunctionInfo {
        self.internal.info()
    }

    /// The name of the function.
    ///
    /// For [`DynamicFunctionMuts`] created using [`IntoFunctionMut`],
    /// the default name will always be the full path to the function as returned by [`core::any::type_name`],
    /// unless the function is a closure, anonymous function, or function pointer,
    /// in which case the name will be `None`.
    ///
    /// This can be overridden using [`with_name`].
    ///
    /// [`DynamicFunctionMuts`]: DynamicFunctionMut
    /// [`with_name`]: Self::with_name
    pub fn name(&self) -> Option<&Cow<'static, str>> {
        self.internal.name()
    }

    /// Returns `true` if the function is [overloaded].
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::IntoFunctionMut;
    /// let mut total_i32 = 0;
    /// let increment = (|value: i32| total_i32 += value).into_function_mut();
    /// assert!(!increment.is_overloaded());
    ///
    /// let mut total_f32 = 0.0;
    /// let increment = increment.with_overload(|value: f32| total_f32 += value);
    /// assert!(increment.is_overloaded());
    /// ```
    ///
    /// [overloaded]: Self::with_overload
    pub fn is_overloaded(&self) -> bool {
        self.internal.is_overloaded()
    }

    /// Returns the number of arguments the function expects.
    ///
    /// For [overloaded] functions that can have a variable number of arguments,
    /// this will contain the full set of counts for all signatures.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::IntoFunctionMut;
    /// let add = (|a: i32, b: i32| a + b).into_function_mut();
    /// assert!(add.arg_count().contains(2));
    ///
    /// let add = add.with_overload(|a: f32, b: f32, c: f32| a + b + c);
    /// assert!(add.arg_count().contains(2));
    /// assert!(add.arg_count().contains(3));
    /// ```
    ///
    /// [overloaded]: Self::with_overload
    pub fn arg_count(&self) -> ArgCount {
        self.internal.arg_count()
    }
}

/// Outputs the function's signature.
///
/// This takes the format: `DynamicFunctionMut(fn {name}({arg1}: {type1}, {arg2}: {type2}, ...) -> {return_type})`.
///
/// Names for arguments and the function itself are optional and will default to `_` if not provided.
///
/// If the function is [overloaded], the output will include the signatures of all overloads as a set.
/// For example, `DynamicFunctionMut(fn add{(_: i32, _: i32) -> i32, (_: f32, _: f32) -> f32})`.
///
/// [overloaded]: DynamicFunctionMut::with_overload
impl<'env> Debug for DynamicFunctionMut<'env> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicFunctionMut({:?})", &self.internal)
    }
}

impl<'env> From<DynamicFunction<'env>> for DynamicFunctionMut<'env> {
    #[inline]
    fn from(function: DynamicFunction<'env>) -> Self {
        Self {
            internal: function.internal.map_functions(arc_to_box),
        }
    }
}

/// Helper function from converting an [`Arc`] function to a [`Box`] function.
///
/// This is needed to help the compiler infer the correct types.
fn arc_to_box<'env>(
    f: Arc<dyn for<'a> Fn(ArgList<'a>) -> FunctionResult<'a> + Send + Sync + 'env>,
) -> BoxFnMut<'env> {
    Box::new(move |args| f(args))
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
    use crate::func::{FunctionError, IntoReturn, SignatureInfo};
    use alloc::vec;
    use core::ops::Add;

    #[test]
    fn should_overwrite_function_name() {
        let mut total = 0;
        let func = (|a: i32, b: i32| total = a + b).into_function_mut();
        assert!(func.name().is_none());

        let func = func.with_name("my_function");
        assert_eq!(func.name().unwrap(), "my_function");
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

    #[test]
    fn should_return_error_on_arg_count_mismatch() {
        let mut total = 0;
        let mut func = (|a: i32, b: i32| total = a + b).into_function_mut();

        let args = ArgList::default().with_owned(25_i32);
        let error = func.call(args).unwrap_err();
        assert_eq!(
            error,
            FunctionError::ArgCountMismatch {
                expected: ArgCount::new(2).unwrap(),
                received: 1
            }
        );

        let args = ArgList::default().with_owned(25_i32);
        let error = func.call_once(args).unwrap_err();
        assert_eq!(
            error,
            FunctionError::ArgCountMismatch {
                expected: ArgCount::new(2).unwrap(),
                received: 1
            }
        );
    }

    #[test]
    fn should_allow_creating_manual_generic_dynamic_function_mut() {
        let mut total = 0_i32;
        let func = DynamicFunctionMut::new(
            |mut args| {
                let value = args.take_arg()?;

                if value.is::<i32>() {
                    let value = value.take::<i32>()?;
                    total += value;
                } else {
                    let value = value.take::<i16>()?;
                    total += value as i32;
                }

                Ok(().into_return())
            },
            vec![
                SignatureInfo::named("add::<i32>").with_arg::<i32>("value"),
                SignatureInfo::named("add::<i16>").with_arg::<i16>("value"),
            ],
        );

        assert_eq!(func.name().unwrap(), "add::<i32>");
        let mut func = func.with_name("add");
        assert_eq!(func.name().unwrap(), "add");

        let args = ArgList::default().with_owned(25_i32);
        func.call(args).unwrap();
        let args = ArgList::default().with_owned(75_i16);
        func.call(args).unwrap();

        drop(func);
        assert_eq!(total, 100);
    }

    // Closures that mutably borrow from their environment cannot realistically
    // be overloaded since that would break Rust's borrowing rules.
    // However, we still need to verify overloaded functions work since a
    // `DynamicFunctionMut` can also be made from a non-mutably borrowing closure/function.
    #[test]
    fn should_allow_function_overloading() {
        fn add<T: Add<Output = T>>(a: T, b: T) -> T {
            a + b
        }

        let mut func = add::<i32>.into_function_mut().with_overload(add::<f32>);

        let args = ArgList::default().with_owned(25_i32).with_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_take::<i32>().unwrap(), 100);

        let args = ArgList::default().with_owned(25.0_f32).with_owned(75.0_f32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_take::<f32>().unwrap(), 100.0);
    }
}
