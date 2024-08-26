use alloc::borrow::Cow;

use bevy_utils::all_tuples;

use crate::func::args::{ArgInfo, GetOwnership, Ownership};
use crate::type_info::impl_type_methods;
use crate::{Type, TypePath};

/// Type information for a [`DynamicFunction`] or [`DynamicFunctionMut`].
///
/// This information can be retrieved directly from certain functions and closures
/// using the [`TypedFunction`] trait, and manually constructed otherwise.
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
/// [`DynamicFunctionMut`]: crate::func::DynamicFunctionMut
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    name: Option<Cow<'static, str>>,
    args: Vec<ArgInfo>,
    return_info: ReturnInfo,
}

impl FunctionInfo {
    /// Create a new [`FunctionInfo`] for a function with the given name.
    pub fn named(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: Some(name.into()),
            args: Vec::new(),
            return_info: ReturnInfo::new::<()>(),
        }
    }

    /// Create a new [`FunctionInfo`] with no name.
    ///
    /// For the purposes of debugging and [registration],
    /// it's recommended to use [`FunctionInfo::named`] instead.
    ///
    /// [registration]: crate::func::FunctionRegistry
    pub fn anonymous() -> Self {
        Self {
            name: None,
            args: Vec::new(),
            return_info: ReturnInfo::new::<()>(),
        }
    }

    /// Create a new [`FunctionInfo`] from the given function.
    pub fn from<F, Marker>(function: &F) -> Self
    where
        F: TypedFunction<Marker>,
    {
        function.get_function_info()
    }

    /// Set the name of the function.
    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Push an argument onto the function's argument list.
    ///
    /// The order in which this method is called matters as it will determine the index of the argument
    /// based on the current number of arguments.
    pub fn with_arg<T: TypePath + GetOwnership>(
        mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> Self {
        let index = self.args.len();
        self.args.push(ArgInfo::new::<T>(index).with_name(name));
        self
    }

    /// Set the arguments of the function.
    ///
    /// This will completely replace any existing arguments.
    ///
    /// It's preferable to use [`Self::with_arg`] to add arguments to the function
    /// as it will automatically set the index of the argument.
    pub fn with_args(mut self, args: Vec<ArgInfo>) -> Self {
        self.args = args;
        self
    }

    /// Set the [return information] of the function.
    ///
    /// To manually set the [`ReturnInfo`] of the function, see [`Self::with_return_info`].
    ///
    /// [return information]: ReturnInfo
    pub fn with_return<T: TypePath + GetOwnership>(mut self) -> Self {
        self.return_info = ReturnInfo::new::<T>();
        self
    }

    /// Set the [return information] of the function.
    ///
    /// This will completely replace any existing return information.
    ///
    /// For a simpler, static version of this method, see [`Self::with_return`].
    ///
    /// [return information]: ReturnInfo
    pub fn with_return_info(mut self, return_info: ReturnInfo) -> Self {
        self.return_info = return_info;
        self
    }

    /// The name of the function.
    ///
    /// For [`DynamicFunctions`] created using [`IntoFunction`] or [`DynamicFunctionMuts`] created using [`IntoFunctionMut`],
    /// the default name will always be the full path to the function as returned by [`std::any::type_name`],
    /// unless the function is a closure, anonymous function, or function pointer,
    /// in which case the name will be `None`.
    ///
    /// [`DynamicFunctions`]: crate::func::DynamicFunction
    /// [`IntoFunction`]: crate::func::IntoFunction
    /// [`DynamicFunctionMuts`]: crate::func::DynamicFunctionMut
    /// [`IntoFunctionMut`]: crate::func::IntoFunctionMut
    pub fn name(&self) -> Option<&Cow<'static, str>> {
        self.name.as_ref()
    }

    /// The arguments of the function.
    pub fn args(&self) -> &[ArgInfo] {
        &self.args
    }

    /// The number of arguments the function takes.
    pub fn arg_count(&self) -> usize {
        self.args.len()
    }

    /// The return information of the function.
    pub fn return_info(&self) -> &ReturnInfo {
        &self.return_info
    }
}

/// Information about the return type of a [`DynamicFunction`] or [`DynamicFunctionMut`].
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
/// [`DynamicFunctionMut`]: crate::func::DynamicFunctionMut
#[derive(Debug, Clone)]
pub struct ReturnInfo {
    ty: Type,
    ownership: Ownership,
}

impl ReturnInfo {
    /// Create a new [`ReturnInfo`] representing the given type, `T`.
    pub fn new<T: TypePath + GetOwnership>() -> Self {
        Self {
            ty: Type::of::<T>(),
            ownership: T::ownership(),
        }
    }

    impl_type_methods!(ty);

    /// The ownership of this type.
    pub fn ownership(&self) -> Ownership {
        self.ownership
    }
}

/// A static accessor to compile-time type information for functions.
///
/// This is the equivalent of [`Typed`], but for function.
///
/// # Blanket Implementation
///
/// This trait has a blanket implementation that covers:
/// - Functions and methods defined with the `fn` keyword
/// - Anonymous functions
/// - Function pointers
/// - Closures that capture immutable references to their environment
/// - Closures that capture mutable references to their environment
/// - Closures that take ownership of captured variables
///
/// For each of the above cases, the function signature may only have up to 15 arguments,
/// not including an optional receiver argument (often `&self` or `&mut self`).
/// This optional receiver argument may be either a mutable or immutable reference to a type.
/// If the return type is also a reference, its lifetime will be bound to the lifetime of this receiver.
///
/// See the [module-level documentation] for more information on valid signatures.
///
/// Arguments and the return type are expected to implement both [`GetOwnership`] and [`TypePath`].
/// By default, these traits are automatically implemented when using the `Reflect` [derive macro].
///
/// # Example
///
/// ```
/// # use bevy_reflect::func::{ArgList, FunctionInfo, ReflectFnMut, TypedFunction};
/// #
/// fn print(value: String) {
///   println!("{}", value);
/// }
///
/// let info = print.get_function_info();
/// assert!(info.name().unwrap().ends_with("print"));
/// assert_eq!(info.arg_count(), 1);
/// assert_eq!(info.args()[0].type_path(), "alloc::string::String");
/// assert_eq!(info.return_info().type_path(), "()");
/// ```
///
/// # Trait Parameters
///
/// This trait has a `Marker` type parameter that is used to get around issues with
/// [unconstrained type parameters] when defining impls with generic arguments or return types.
/// This `Marker` can be any type, provided it doesn't conflict with other implementations.
///
/// [module-level documentation]: crate::func
/// [`Typed`]: crate::Typed
/// [unconstrained type parameters]: https://doc.rust-lang.org/error_codes/E0207.html
pub trait TypedFunction<Marker> {
    /// Get the [`FunctionInfo`] for this type.
    fn function_info() -> FunctionInfo;

    /// Get the [`FunctionInfo`] for this type.
    fn get_function_info(&self) -> FunctionInfo {
        Self::function_info()
    }
}

/// Helper macro for implementing [`TypedFunction`] on Rust functions.
///
/// This currently implements it for the following signatures (where `argX` may be any of `T`, `&T`, or `&mut T`):
/// - `FnMut(arg0, arg1, ..., argN) -> R`
/// - `FnMut(&Receiver, arg0, arg1, ..., argN) -> &R`
/// - `FnMut(&mut Receiver, arg0, arg1, ..., argN) -> &mut R`
/// - `FnMut(&mut Receiver, arg0, arg1, ..., argN) -> &R`
macro_rules! impl_typed_function {
    ($(($Arg:ident, $arg:ident)),*) => {
        // === (...) -> ReturnType === //
        impl<$($Arg,)* ReturnType, Function> TypedFunction<fn($($Arg),*) -> [ReturnType]> for Function
        where
            $($Arg: TypePath + GetOwnership,)*
            ReturnType: TypePath + GetOwnership,
            Function: FnMut($($Arg),*) -> ReturnType,
        {
            fn function_info() -> FunctionInfo {
                create_info::<Function>()
                    .with_args({
                        #[allow(unused_mut)]
                        let mut _index = 0;
                        vec![
                            $(ArgInfo::new::<$Arg>({
                                _index += 1;
                                _index - 1
                            }),)*
                        ]
                    })
                    .with_return_info(ReturnInfo::new::<ReturnType>())
            }
        }

        // === (&self, ...) -> &ReturnType === //
        impl<Receiver, $($Arg,)* ReturnType, Function> TypedFunction<fn(&Receiver, $($Arg),*) -> &ReturnType> for Function
        where
            for<'a> &'a Receiver: TypePath + GetOwnership,
            $($Arg: TypePath + GetOwnership,)*
            for<'a> &'a ReturnType: TypePath + GetOwnership,
            Function: for<'a> FnMut(&'a Receiver, $($Arg),*) -> &'a ReturnType,
        {
            fn function_info() -> $crate::func::FunctionInfo {
                create_info::<Function>()
                    .with_args({
                        #[allow(unused_mut)]
                        let mut _index = 1;
                        vec![
                            ArgInfo::new::<&Receiver>(0),
                            $($crate::func::args::ArgInfo::new::<$Arg>({
                                _index += 1;
                                _index - 1
                            }),)*
                        ]
                    })
                    .with_return_info(ReturnInfo::new::<&ReturnType>())
            }
        }

        // === (&mut self, ...) -> &mut ReturnType === //
        impl<Receiver, $($Arg,)* ReturnType, Function> TypedFunction<fn(&mut Receiver, $($Arg),*) -> &mut ReturnType> for Function
        where
            for<'a> &'a mut Receiver: TypePath + GetOwnership,
            $($Arg: TypePath + GetOwnership,)*
            for<'a> &'a mut ReturnType: TypePath + GetOwnership,
            Function: for<'a> FnMut(&'a mut Receiver, $($Arg),*) -> &'a mut ReturnType,
        {
            fn function_info() -> FunctionInfo {
                create_info::<Function>()
                    .with_args({
                        #[allow(unused_mut)]
                        let mut _index = 1;
                        vec![
                            ArgInfo::new::<&mut Receiver>(0),
                            $(ArgInfo::new::<$Arg>({
                                _index += 1;
                                _index - 1
                            }),)*
                        ]
                    })
                    .with_return_info(ReturnInfo::new::<&mut ReturnType>())
            }
        }

        // === (&mut self, ...) -> &ReturnType === //
        impl<Receiver, $($Arg,)* ReturnType, Function> TypedFunction<fn(&mut Receiver, $($Arg),*) -> &ReturnType> for Function
        where
            for<'a> &'a mut Receiver: TypePath + GetOwnership,
            $($Arg: TypePath + GetOwnership,)*
            for<'a> &'a ReturnType: TypePath + GetOwnership,
            Function: for<'a> FnMut(&'a mut Receiver, $($Arg),*) -> &'a ReturnType,
        {
            fn function_info() -> FunctionInfo {
                create_info::<Function>()
                    .with_args({
                        #[allow(unused_mut)]
                        let mut _index = 1;
                        vec![
                            ArgInfo::new::<&mut Receiver>(0),
                            $(ArgInfo::new::<$Arg>({
                                _index += 1;
                                _index - 1
                            }),)*
                        ]
                    })
                    .with_return_info(ReturnInfo::new::<&ReturnType>())
            }
        }
    };
}

all_tuples!(impl_typed_function, 0, 15, Arg, arg);

/// Helper function for creating [`FunctionInfo`] with the proper name value.
///
/// Names are only given if:
/// - The function is not a closure
/// - The function is not a function pointer
/// - The function is not an anonymous function
///
/// This function relies on the [`type_name`] of `F` to determine this.
/// The following table describes the behavior for different types of functions:
///
/// | Category           | `type_name`             | `FunctionInfo::name`    |
/// | ------------------ | ----------------------- | ----------------------- |
/// | Named function     | `foo::bar::baz`         | `Some("foo::bar::baz")` |
/// | Closure            | `foo::bar::{{closure}}` | `None`                  |
/// | Anonymous function | `foo::bar::{{closure}}` | `None`                  |
/// | Function pointer   | `fn() -> String`        | `None`                  |
///
/// [`type_name`]: std::any::type_name
fn create_info<F>() -> FunctionInfo {
    let name = std::any::type_name::<F>();

    if name.ends_with("{{closure}}") || name.starts_with("fn(") {
        FunctionInfo::anonymous()
    } else {
        FunctionInfo::named(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_create_function_info() {
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        // Sanity check:
        assert_eq!(
            std::any::type_name_of_val(&add),
            "bevy_reflect::func::info::tests::should_create_function_info::add"
        );

        let info = add.get_function_info();
        assert_eq!(
            info.name().unwrap(),
            "bevy_reflect::func::info::tests::should_create_function_info::add"
        );
        assert_eq!(info.arg_count(), 2);
        assert_eq!(info.args()[0].type_path(), "i32");
        assert_eq!(info.args()[1].type_path(), "i32");
        assert_eq!(info.return_info().type_path(), "i32");
    }

    #[test]
    fn should_create_function_pointer_info() {
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        let add = add as fn(i32, i32) -> i32;

        // Sanity check:
        assert_eq!(std::any::type_name_of_val(&add), "fn(i32, i32) -> i32");

        let info = add.get_function_info();
        assert!(info.name().is_none());
        assert_eq!(info.arg_count(), 2);
        assert_eq!(info.args()[0].type_path(), "i32");
        assert_eq!(info.args()[1].type_path(), "i32");
        assert_eq!(info.return_info().type_path(), "i32");
    }

    #[test]
    fn should_create_anonymous_function_info() {
        let add = |a: i32, b: i32| a + b;

        // Sanity check:
        assert_eq!(
            std::any::type_name_of_val(&add),
            "bevy_reflect::func::info::tests::should_create_anonymous_function_info::{{closure}}"
        );

        let info = add.get_function_info();
        assert!(info.name().is_none());
        assert_eq!(info.arg_count(), 2);
        assert_eq!(info.args()[0].type_path(), "i32");
        assert_eq!(info.args()[1].type_path(), "i32");
        assert_eq!(info.return_info().type_path(), "i32");
    }

    #[test]
    fn should_create_closure_info() {
        let mut total = 0;
        let add = |a: i32, b: i32| total = a + b;

        // Sanity check:
        assert_eq!(
            std::any::type_name_of_val(&add),
            "bevy_reflect::func::info::tests::should_create_closure_info::{{closure}}"
        );

        let info = add.get_function_info();
        assert!(info.name().is_none());
        assert_eq!(info.arg_count(), 2);
        assert_eq!(info.args()[0].type_path(), "i32");
        assert_eq!(info.args()[1].type_path(), "i32");
        assert_eq!(info.return_info().type_path(), "()");
    }
}
