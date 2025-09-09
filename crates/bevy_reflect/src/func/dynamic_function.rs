use crate::{
    __macro_exports::RegisterForReflection,
    func::{
        args::{ArgCount, ArgList},
        dynamic_function_internal::DynamicFunctionInternal,
        info::FunctionInfo,
        DynamicFunctionMut, Function, FunctionOverloadError, FunctionResult, IntoFunction,
        IntoFunctionMut,
    },
    ApplyError, MaybeTyped, PartialReflect, Reflect, ReflectKind, ReflectMut, ReflectOwned,
    ReflectRef, TypeInfo, TypePath,
};
use alloc::{borrow::Cow, boxed::Box};
use bevy_platform::sync::Arc;
use bevy_reflect_derive::impl_type_path;
use core::fmt::{Debug, Formatter};

/// An [`Arc`] containing a callback to a reflected function.
///
/// The `Arc` is used to both ensure that it is `Send + Sync`
/// and to allow for the callback to be cloned.
///
/// Note that cloning is okay since we only ever need an immutable reference
/// to call a `dyn Fn` function.
/// If we were to contain a `dyn FnMut` instead, cloning would be a lot more complicated.
type ArcFn<'env> = Arc<dyn for<'a> Fn(ArgList<'a>) -> FunctionResult<'a> + Send + Sync + 'env>;

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
/// # use bevy_reflect::func::{ArgList, DynamicFunction, IntoFunction};
/// #
/// fn add(a: i32, b: i32) -> i32 {
///   a + b
/// }
///
/// // Convert the function into a dynamic function using `IntoFunction::into_function`:
/// let mut func: DynamicFunction = add.into_function();
///
/// // Dynamically call it:
/// let args = ArgList::default().with_owned(25_i32).with_owned(75_i32);
/// let value = func.call(args).unwrap().unwrap_owned();
///
/// // Check the result:
/// assert_eq!(value.try_downcast_ref::<i32>(), Some(&100));
/// ```
///
/// [`ReflectFn`]: crate::func::ReflectFn
/// [module-level documentation]: crate::func
#[derive(Clone)]
pub struct DynamicFunction<'env> {
    pub(super) internal: DynamicFunctionInternal<ArcFn<'env>>,
}

impl<'env> DynamicFunction<'env> {
    /// Create a new [`DynamicFunction`].
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
    /// [calling]: crate::func::dynamic_function::DynamicFunction::call
    /// [`SignatureInfo`]: crate::func::SignatureInfo
    /// [function overloading]: Self::with_overload
    pub fn new<F: for<'a> Fn(ArgList<'a>) -> FunctionResult<'a> + Send + Sync + 'env>(
        func: F,
        info: impl TryInto<FunctionInfo, Error: Debug>,
    ) -> Self {
        let arc = Arc::new(func);

        #[cfg(not(target_has_atomic = "ptr"))]
        #[expect(
            unsafe_code,
            reason = "unsized coercion is an unstable feature for non-std types"
        )]
        // SAFETY:
        // - Coercion from `T` to `dyn for<'a> Fn(ArgList<'a>) -> FunctionResult<'a> + Send + Sync + 'env`
        //   is valid as `T: for<'a> Fn(ArgList<'a>) -> FunctionResult<'a> + Send + Sync + 'env`
        // - `Arc::from_raw` receives a valid pointer from a previous call to `Arc::into_raw`
        let arc = unsafe { ArcFn::<'env>::from_raw(Arc::into_raw(arc) as *const _) };

        Self {
            internal: DynamicFunctionInternal::new(arc, info.try_into().unwrap()),
        }
    }

    /// Set the name of the function.
    ///
    /// For [`DynamicFunctions`] created using [`IntoFunction`],
    /// the default name will always be the full path to the function as returned by [`core::any::type_name`],
    /// unless the function is a closure, anonymous function, or function pointer,
    /// in which case the name will be `None`.
    ///
    /// [`DynamicFunctions`]: DynamicFunction
    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.internal = self.internal.with_name(name);
        self
    }

    /// Add an overload to this function.
    ///
    /// Overloads allow a single [`DynamicFunction`] to represent multiple functions of different signatures.
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
    /// # Panics
    ///
    /// Panics if the function, `F`, contains a signature already found in this function.
    ///
    /// For a non-panicking version, see [`try_with_overload`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::ops::Add;
    /// # use bevy_reflect::func::{ArgList, IntoFunction};
    /// #
    /// fn add<T: Add<Output = T>>(a: T, b: T) -> T {
    ///     a + b
    /// }
    ///
    /// // Currently, the only generic type `func` supports is `i32`:
    /// let mut func = add::<i32>.into_function();
    ///
    /// // However, we can add an overload to handle `f32` as well:
    /// func = func.with_overload(add::<f32>);
    ///
    /// // Test `i32`:
    /// let args = ArgList::default().with_owned(25_i32).with_owned(75_i32);
    /// let result = func.call(args).unwrap().unwrap_owned();
    /// assert_eq!(result.try_take::<i32>().unwrap(), 100);
    ///
    /// // Test `f32`:
    /// let args = ArgList::default().with_owned(25.0_f32).with_owned(75.0_f32);
    /// let result = func.call(args).unwrap().unwrap_owned();
    /// assert_eq!(result.try_take::<f32>().unwrap(), 100.0);
    ///```
    ///
    /// ```
    /// # use bevy_reflect::func::{ArgList, IntoFunction};
    /// #
    /// fn add_2(a: i32, b: i32) -> i32 {
    ///     a + b
    /// }
    ///
    /// fn add_3(a: i32, b: i32, c: i32) -> i32 {
    ///     a + b + c
    /// }
    ///
    /// // Currently, `func` only supports two arguments.
    /// let mut func = add_2.into_function();
    ///
    /// // However, we can add an overload to handle three arguments as well.
    /// func = func.with_overload(add_3);
    ///
    /// // Test two arguments:
    /// let args = ArgList::default().with_owned(25_i32).with_owned(75_i32);
    /// let result = func.call(args).unwrap().unwrap_owned();
    /// assert_eq!(result.try_take::<i32>().unwrap(), 100);
    ///
    /// // Test three arguments:
    /// let args = ArgList::default()
    ///     .with_owned(25_i32)
    ///     .with_owned(75_i32)
    ///     .with_owned(100_i32);
    /// let result = func.call(args).unwrap().unwrap_owned();
    /// assert_eq!(result.try_take::<i32>().unwrap(), 200);
    /// ```
    ///
    ///```should_panic
    /// # use bevy_reflect::func::IntoFunction;
    ///
    /// fn add(a: i32, b: i32) -> i32 {
    ///     a + b
    /// }
    ///
    /// fn sub(a: i32, b: i32) -> i32 {
    ///     a - b
    /// }
    ///
    /// let mut func = add.into_function();
    ///
    /// // This will panic because the function already has an argument signature for `(i32, i32)`:
    /// func = func.with_overload(sub);
    /// ```
    ///
    /// [argument signature]: crate::func::signature::ArgumentSignature
    /// [name]: Self::name
    /// [`try_with_overload`]: Self::try_with_overload
    pub fn with_overload<'a, F: IntoFunction<'a, Marker>, Marker>(
        self,
        function: F,
    ) -> DynamicFunction<'a>
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
    pub fn try_with_overload<F: IntoFunction<'env, Marker>, Marker>(
        mut self,
        function: F,
    ) -> Result<Self, (Box<Self>, FunctionOverloadError)> {
        let function = function.into_function();

        match self.internal.merge(function.internal) {
            Ok(_) => Ok(self),
            Err(err) => Err((Box::new(self), err)),
        }
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
    /// let args = ArgList::new().with_owned(25_i32).with_owned(75_i32);
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
        self.internal.validate_args(&args)?;
        let func = self.internal.get(&args)?;
        func(args)
    }

    /// Returns the function info.
    pub fn info(&self) -> &FunctionInfo {
        self.internal.info()
    }

    /// The name of the function.
    ///
    /// For [`DynamicFunctions`] created using [`IntoFunction`],
    /// the default name will always be the full path to the function as returned by [`core::any::type_name`],
    /// unless the function is a closure, anonymous function, or function pointer,
    /// in which case the name will be `None`.
    ///
    /// This can be overridden using [`with_name`].
    ///
    /// If the function was [overloaded], it will retain its original name if it had one.
    ///
    /// [`DynamicFunctions`]: DynamicFunction
    /// [`with_name`]: Self::with_name
    /// [overloaded]: Self::with_overload
    pub fn name(&self) -> Option<&Cow<'static, str>> {
        self.internal.name()
    }

    /// Returns `true` if the function is [overloaded].
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::IntoFunction;
    /// let add = (|a: i32, b: i32| a + b).into_function();
    /// assert!(!add.is_overloaded());
    ///
    /// let add = add.with_overload(|a: f32, b: f32| a + b);
    /// assert!(add.is_overloaded());
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
    /// # use bevy_reflect::func::IntoFunction;
    /// let add = (|a: i32, b: i32| a + b).into_function();
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

impl Function for DynamicFunction<'static> {
    fn name(&self) -> Option<&Cow<'static, str>> {
        self.internal.name()
    }

    fn info(&self) -> &FunctionInfo {
        self.internal.info()
    }

    fn reflect_call<'a>(&self, args: ArgList<'a>) -> FunctionResult<'a> {
        self.call(args)
    }

    fn to_dynamic_function(&self) -> DynamicFunction<'static> {
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
                *self = func.to_dynamic_function();
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

    fn reflect_ref(&self) -> ReflectRef<'_> {
        ReflectRef::Function(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut<'_> {
        ReflectMut::Function(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Function(self)
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
///
/// If the function is [overloaded], the output will include the signatures of all overloads as a set.
/// For example, `DynamicFunction(fn add{(_: i32, _: i32) -> i32, (_: f32, _: f32) -> f32})`.
///
/// [overloaded]: DynamicFunction::with_overload
impl<'env> Debug for DynamicFunction<'env> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicFunction({:?})", &self.internal)
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
    use crate::func::signature::ArgumentSignature;
    use crate::func::{FunctionError, IntoReturn, SignatureInfo};
    use crate::Type;
    use alloc::{format, string::String, vec, vec::Vec};
    use bevy_platform::collections::HashSet;
    use core::ops::Add;

    #[test]
    fn should_overwrite_function_name() {
        let c = 23;
        let func = (|a: i32, b: i32| a + b + c).into_function();
        assert!(func.name().is_none());

        let func = func.with_name("my_function");
        assert_eq!(func.name().unwrap(), "my_function");
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

        let args = ArgList::default().with_owned(25_i32);
        let error = func.call(args).unwrap_err();

        assert_eq!(
            error,
            FunctionError::ArgCountMismatch {
                expected: ArgCount::new(2).unwrap(),
                received: 1
            }
        );
    }

    #[test]
    fn should_return_error_on_arg_count_mismatch_overloaded() {
        let func = (|a: i32, b: i32| a + b)
            .into_function()
            .with_overload(|a: i32, b: i32, c: i32| a + b + c);

        let args = ArgList::default()
            .with_owned(1_i32)
            .with_owned(2_i32)
            .with_owned(3_i32)
            .with_owned(4_i32);

        let error = func.call(args).unwrap_err();

        let mut expected_count = ArgCount::new(2).unwrap();
        expected_count.add(3);

        assert_eq!(
            error,
            FunctionError::ArgCountMismatch {
                expected: expected_count,
                received: 4
            }
        );
    }

    #[test]
    fn should_clone_dynamic_function() {
        let hello = String::from("Hello");

        let greet = |name: &String| -> String { format!("{hello}, {name}!") };

        let greet = greet.into_function().with_name("greet");
        let clone = greet.clone();

        assert_eq!(greet.name().unwrap(), "greet");
        assert_eq!(clone.name().unwrap(), "greet");

        let cloned_value = clone
            .call(ArgList::default().with_ref(&String::from("world")))
            .unwrap()
            .unwrap_owned()
            .try_take::<String>()
            .unwrap();

        assert_eq!(cloned_value, "Hello, world!");
    }

    #[test]
    fn should_apply_function() {
        let mut func: Box<dyn Function> = Box::new((|a: i32, b: i32| a + b).into_function());
        func.apply(&((|a: i32, b: i32| a * b).into_function()));

        let args = ArgList::new().with_owned(5_i32).with_owned(5_i32);
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
                                .with_ref(this.as_partial_reflect())
                                .with_owned(curr - 1),
                        );
                        let value = result.unwrap().unwrap_owned().try_take::<i32>().unwrap();
                        Ok((curr * value).into_return())
                    }
                    _ => panic!("expected function"),
                }
            },
            // The `FunctionInfo` doesn't really matter for this test
            // so we can just give it dummy information.
            SignatureInfo::anonymous()
                .with_arg::<i32>("curr")
                .with_arg::<()>("this"),
        );

        let args = ArgList::new().with_ref(&factorial).with_owned(5_i32);
        let value = factorial.call(args).unwrap().unwrap_owned();
        assert_eq!(value.try_take::<i32>().unwrap(), 120);
    }

    #[test]
    fn should_allow_creating_manual_generic_dynamic_function() {
        let func = DynamicFunction::new(
            |mut args| {
                let a = args.take_arg()?;
                let b = args.take_arg()?;

                if a.is::<i32>() {
                    let a = a.take::<i32>()?;
                    let b = b.take::<i32>()?;
                    Ok((a + b).into_return())
                } else {
                    let a = a.take::<f32>()?;
                    let b = b.take::<f32>()?;
                    Ok((a + b).into_return())
                }
            },
            vec![
                SignatureInfo::named("add::<i32>")
                    .with_arg::<i32>("a")
                    .with_arg::<i32>("b")
                    .with_return::<i32>(),
                SignatureInfo::named("add::<f32>")
                    .with_arg::<f32>("a")
                    .with_arg::<f32>("b")
                    .with_return::<f32>(),
            ],
        );

        assert_eq!(func.name().unwrap(), "add::<i32>");
        let func = func.with_name("add");
        assert_eq!(func.name().unwrap(), "add");

        let args = ArgList::default().with_owned(25_i32).with_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_take::<i32>().unwrap(), 100);

        let args = ArgList::default().with_owned(25.0_f32).with_owned(75.0_f32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_take::<f32>().unwrap(), 100.0);
    }

    #[test]
    #[should_panic(expected = "called `Result::unwrap()` on an `Err` value: MissingSignature")]
    fn should_panic_on_missing_function_info() {
        let _ = DynamicFunction::new(|_| Ok(().into_return()), Vec::new());
    }

    #[test]
    fn should_allow_function_overloading() {
        fn add<T: Add<Output = T>>(a: T, b: T) -> T {
            a + b
        }

        let func = add::<i32>.into_function().with_overload(add::<f32>);

        let args = ArgList::default().with_owned(25_i32).with_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_take::<i32>().unwrap(), 100);

        let args = ArgList::default().with_owned(25.0_f32).with_owned(75.0_f32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_take::<f32>().unwrap(), 100.0);
    }

    #[test]
    fn should_allow_variable_arguments_via_overloading() {
        fn add_2(a: i32, b: i32) -> i32 {
            a + b
        }

        fn add_3(a: i32, b: i32, c: i32) -> i32 {
            a + b + c
        }

        let func = add_2.into_function().with_overload(add_3);

        let args = ArgList::default().with_owned(25_i32).with_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_take::<i32>().unwrap(), 100);

        let args = ArgList::default()
            .with_owned(25_i32)
            .with_owned(75_i32)
            .with_owned(100_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_take::<i32>().unwrap(), 200);
    }

    #[test]
    fn should_allow_function_overloading_with_manual_overload() {
        let manual = DynamicFunction::new(
            |mut args| {
                let a = args.take_arg()?;
                let b = args.take_arg()?;

                if a.is::<i32>() {
                    let a = a.take::<i32>()?;
                    let b = b.take::<i32>()?;
                    Ok((a + b).into_return())
                } else {
                    let a = a.take::<f32>()?;
                    let b = b.take::<f32>()?;
                    Ok((a + b).into_return())
                }
            },
            vec![
                SignatureInfo::named("add::<i32>")
                    .with_arg::<i32>("a")
                    .with_arg::<i32>("b")
                    .with_return::<i32>(),
                SignatureInfo::named("add::<f32>")
                    .with_arg::<f32>("a")
                    .with_arg::<f32>("b")
                    .with_return::<f32>(),
            ],
        );

        let func = manual.with_overload(|a: u32, b: u32| a + b);

        let args = ArgList::default().with_owned(25_i32).with_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_take::<i32>().unwrap(), 100);

        let args = ArgList::default().with_owned(25_u32).with_owned(75_u32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_take::<u32>().unwrap(), 100);
    }

    #[test]
    fn should_return_error_on_unknown_overload() {
        fn add<T: Add<Output = T>>(a: T, b: T) -> T {
            a + b
        }

        let func = add::<i32>.into_function().with_overload(add::<f32>);

        let args = ArgList::default().with_owned(25_u32).with_owned(75_u32);
        let result = func.call(args);
        assert_eq!(
            result.unwrap_err(),
            FunctionError::NoOverload {
                expected: [
                    ArgumentSignature::from_iter(vec![Type::of::<i32>(), Type::of::<i32>()]),
                    ArgumentSignature::from_iter(vec![Type::of::<f32>(), Type::of::<f32>()])
                ]
                .into_iter()
                .collect::<HashSet<_>>(),
                received: ArgumentSignature::from_iter(vec![Type::of::<u32>(), Type::of::<u32>()]),
            }
        );
    }

    #[test]
    fn should_debug_dynamic_function() {
        fn greet(name: &String) -> String {
            format!("Hello, {name}!")
        }

        let function = greet.into_function();
        let debug = format!("{function:?}");
        assert_eq!(debug, "DynamicFunction(fn bevy_reflect::func::dynamic_function::tests::should_debug_dynamic_function::greet(_: &alloc::string::String) -> alloc::string::String)");
    }

    #[test]
    fn should_debug_anonymous_dynamic_function() {
        let function = (|a: i32, b: i32| a + b).into_function();
        let debug = format!("{function:?}");
        assert_eq!(debug, "DynamicFunction(fn _(_: i32, _: i32) -> i32)");
    }

    #[test]
    fn should_debug_overloaded_dynamic_function() {
        fn add<T: Add<Output = T>>(a: T, b: T) -> T {
            a + b
        }

        let function = add::<i32>
            .into_function()
            .with_overload(add::<f32>)
            .with_name("add");
        let debug = format!("{function:?}");
        assert_eq!(
            debug,
            "DynamicFunction(fn add{(_: i32, _: i32) -> i32, (_: f32, _: f32) -> f32})"
        );
    }
}
