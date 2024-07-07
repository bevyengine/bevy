use alloc::borrow::Cow;

use bevy_utils::all_tuples;

use crate::func::args::{ArgInfo, GetOwnership, Ownership};
use crate::TypePath;

/// Type information for a [`DynamicFunction`] or [`DynamicClosure`].
///
/// This information can be retrieved from certain functions and closures
/// using the [`TypedFunction`] trait.
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
/// [`DynamicClosure`]: crate::func::DynamicClosure
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    name: Option<Cow<'static, str>>,
    args: Vec<ArgInfo>,
    return_info: ReturnInfo,
}

impl FunctionInfo {
    /// Create a new [`FunctionInfo`].
    ///
    /// To set the name of the function, use [`Self::with_name`].
    pub fn new() -> Self {
        Self {
            name: None,
            args: Vec::new(),
            return_info: ReturnInfo::new::<()>(),
        }
    }

    /// Create a new [`FunctionInfo`] from the given function or closure.
    pub fn from<F, Marker>(function: &F) -> Self
    where
        F: TypedFunction<Marker>,
    {
        function.get_function_info()
    }

    /// Set the name of the function.
    ///
    /// Reflected functions are not required to have a name,
    /// so this method must be called manually to set the name.
    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the arguments of the function.
    pub fn with_args(mut self, args: Vec<ArgInfo>) -> Self {
        self.args = args;
        self
    }

    /// Set the return information of the function.
    pub fn with_return_info(mut self, return_info: ReturnInfo) -> Self {
        self.return_info = return_info;
        self
    }

    /// The name of the function, if it was given one.
    ///
    /// For [`DynamicFunctions`] created using [`IntoFunction`] or [`DynamicClosures`] created using [`IntoClosure`],
    /// the name will always be the full path to the function as returned by [`std::any::type_name`].
    ///
    /// [`DynamicFunctions`]: crate::func::DynamicFunction
    /// [`IntoFunction`]: crate::func::IntoFunction
    /// [`DynamicClosures`]: crate::func::DynamicClosure
    /// [`IntoClosure`]: crate::func::IntoClosure
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
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

impl Default for FunctionInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about the return type of a [`DynamicFunction`] or [`DynamicClosure`].
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
/// [`DynamicClosure`]: crate::func::DynamicClosure
#[derive(Debug, Clone)]
pub struct ReturnInfo {
    type_path: &'static str,
    ownership: Ownership,
}

impl ReturnInfo {
    /// Create a new [`ReturnInfo`] representing the given type, `T`.
    pub fn new<T: TypePath + GetOwnership>() -> Self {
        Self {
            type_path: T::type_path(),
            ownership: T::ownership(),
        }
    }

    /// The type path of the return type.
    pub fn type_path(&self) -> &'static str {
        self.type_path
    }

    /// The ownership of the return type.
    pub fn ownership(&self) -> Ownership {
        self.ownership
    }
}

/// A static accessor to compile-time type information for functions.
///
/// This is the equivalent of [`Typed`] for functions.
///
/// # Blanket Implementation
///
/// This trait has a blanket implementation that covers:
/// - Functions and methods defined with the `fn` keyword
/// - Closures that do not capture their environment
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
pub trait TypedFunction<Marker> {
    /// Get the [`FunctionInfo`] for this type.
    fn function_info() -> FunctionInfo;

    /// Get the [`FunctionInfo`] for this type.
    fn get_function_info(&self) -> FunctionInfo {
        Self::function_info()
    }
}

/// Helper macro for implementing [`TypedFunction`] on Rust closures.
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
                FunctionInfo::new()
                    .with_name(std::any::type_name::<Function>())
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
                FunctionInfo::new()
                    .with_name(std::any::type_name::<Function>())
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
                FunctionInfo::new()
                    .with_name(std::any::type_name::<Function>())
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
                FunctionInfo::new()
                    .with_name(std::any::type_name::<Function>())
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
