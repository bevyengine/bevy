use alloc::{borrow::Cow, boxed::Box, vec, vec::Vec};
use core::fmt::{Debug, Formatter};

use crate::{
    func::args::{ArgCount, ArgCountOutOfBoundsError, ArgInfo, GetOwnership, Ownership},
    func::signature::ArgumentSignature,
    func::FunctionOverloadError,
    type_info::impl_type_methods,
    Type, TypePath,
};

use variadics_please::all_tuples;

/// Type information for a [`DynamicFunction`] or [`DynamicFunctionMut`].
///
/// This information can be retrieved directly from certain functions and closures
/// using the [`TypedFunction`] trait, and manually constructed otherwise.
///
/// It is compromised of one or more [`SignatureInfo`] structs,
/// allowing it to represent functions with multiple sets of arguments (i.e. "overloaded functions").
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
/// [`DynamicFunctionMut`]: crate::func::DynamicFunctionMut
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    name: Option<Cow<'static, str>>,
    arg_count: ArgCount,
    signatures: Box<[SignatureInfo]>,
}

impl FunctionInfo {
    /// Create a new [`FunctionInfo`] for a function with the given signature.
    ///
    /// # Panics
    ///
    /// Panics if the given signature has more than the maximum number of arguments
    /// as specified by [`ArgCount::MAX_COUNT`].
    pub fn new(signature: SignatureInfo) -> Self {
        Self {
            name: signature.name.clone(),
            arg_count: ArgCount::new(signature.arg_count()).unwrap(),
            signatures: vec![signature].into(),
        }
    }

    /// Create a new [`FunctionInfo`] from a set of signatures.
    ///
    /// Returns an error if the given iterator is empty or contains duplicate signatures.
    pub fn try_from_iter(
        signatures: impl IntoIterator<Item = SignatureInfo>,
    ) -> Result<Self, FunctionOverloadError> {
        let mut iter = signatures.into_iter();

        let base = iter.next().ok_or(FunctionOverloadError::MissingSignature)?;

        if base.arg_count() > ArgCount::MAX_COUNT {
            return Err(FunctionOverloadError::TooManyArguments(
                ArgumentSignature::from(&base),
            ));
        }

        let mut info = Self::new(base);

        for signature in iter {
            if signature.arg_count() > ArgCount::MAX_COUNT {
                return Err(FunctionOverloadError::TooManyArguments(
                    ArgumentSignature::from(&signature),
                ));
            }

            info = info.with_overload(signature).map_err(|sig| {
                FunctionOverloadError::DuplicateSignature(ArgumentSignature::from(&sig))
            })?;
        }

        Ok(info)
    }

    /// The base signature for this function.
    ///
    /// All functions—including overloaded functions—are guaranteed to have at least one signature.
    /// The first signature used to define the [`FunctionInfo`] is considered the base signature.
    pub fn base(&self) -> &SignatureInfo {
        &self.signatures[0]
    }

    /// Whether this function is overloaded.
    ///
    /// This is determined by the existence of multiple signatures.
    pub fn is_overloaded(&self) -> bool {
        self.signatures.len() > 1
    }

    /// Set the name of the function.
    pub fn with_name(mut self, name: Option<impl Into<Cow<'static, str>>>) -> Self {
        self.name = name.map(Into::into);
        self
    }

    /// The name of the function.
    ///
    /// For [`DynamicFunctions`] created using [`IntoFunction`] or [`DynamicFunctionMuts`] created using [`IntoFunctionMut`],
    /// the default name will always be the full path to the function as returned by [`std::any::type_name`],
    /// unless the function is a closure, anonymous function, or function pointer,
    /// in which case the name will be `None`.
    ///
    /// For overloaded functions, this will be the name of the base signature,
    /// unless manually overwritten using [`Self::with_name`].
    ///
    /// [`DynamicFunctions`]: crate::func::DynamicFunction
    /// [`IntoFunction`]: crate::func::IntoFunction
    /// [`DynamicFunctionMuts`]: crate::func::DynamicFunctionMut
    /// [`IntoFunctionMut`]: crate::func::IntoFunctionMut
    pub fn name(&self) -> Option<&Cow<'static, str>> {
        self.name.as_ref()
    }

    /// Add a signature to this function.
    ///
    /// If a signature with the same [`ArgumentSignature`] already exists,
    /// an error is returned with the given signature.
    ///
    /// # Panics
    ///
    /// Panics if the given signature has more than the maximum number of arguments
    /// as specified by [`ArgCount::MAX_COUNT`].
    pub fn with_overload(mut self, signature: SignatureInfo) -> Result<Self, SignatureInfo> {
        let is_duplicate = self.signatures.iter().any(|s| {
            s.arg_count() == signature.arg_count()
                && ArgumentSignature::from(s) == ArgumentSignature::from(&signature)
        });

        if is_duplicate {
            return Err(signature);
        }

        self.arg_count.add(signature.arg_count());
        self.signatures = IntoIterator::into_iter(self.signatures)
            .chain(Some(signature))
            .collect();
        Ok(self)
    }

    /// Returns the number of arguments the function expects.
    ///
    /// For [overloaded] functions that can have a variable number of arguments,
    /// this will contain the full set of counts for all signatures.
    ///
    /// [overloaded]: crate::func#overloading-functions
    pub fn arg_count(&self) -> ArgCount {
        self.arg_count
    }

    /// The signatures of the function.
    ///
    /// This is guaranteed to always contain at least one signature.
    /// Overloaded functions will contain two or more.
    pub fn signatures(&self) -> &[SignatureInfo] {
        &self.signatures
    }

    /// Returns a wrapper around this info that implements [`Debug`] for pretty-printing the function.
    ///
    /// This can be useful for more readable debugging and logging.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_reflect::func::{FunctionInfo, TypedFunction};
    /// #
    /// fn add(a: i32, b: i32) -> i32 {
    ///     a + b
    /// }
    ///
    /// let info = add.get_function_info();
    ///
    /// let pretty = info.pretty_printer();
    /// assert_eq!(format!("{:?}", pretty), "(_: i32, _: i32) -> i32");
    /// ```
    pub fn pretty_printer(&self) -> PrettyPrintFunctionInfo<'_> {
        PrettyPrintFunctionInfo::new(self)
    }

    /// Extend this [`FunctionInfo`] with another without checking for duplicates.
    ///
    /// # Panics
    ///
    /// Panics if the given signature has more than the maximum number of arguments
    /// as specified by [`ArgCount::MAX_COUNT`].
    pub(super) fn extend_unchecked(&mut self, other: FunctionInfo) {
        if self.name.is_none() {
            self.name = other.name;
        }

        let signatures = core::mem::take(&mut self.signatures);
        self.signatures = IntoIterator::into_iter(signatures)
            .chain(IntoIterator::into_iter(other.signatures))
            .collect();
        self.arg_count = self
            .signatures
            .iter()
            .fold(ArgCount::default(), |mut count, sig| {
                count.add(sig.arg_count());
                count
            });
    }
}

impl TryFrom<SignatureInfo> for FunctionInfo {
    type Error = ArgCountOutOfBoundsError;

    fn try_from(signature: SignatureInfo) -> Result<Self, Self::Error> {
        let count = signature.arg_count();
        if count > ArgCount::MAX_COUNT {
            return Err(ArgCountOutOfBoundsError(count));
        }

        Ok(Self::new(signature))
    }
}

impl TryFrom<Vec<SignatureInfo>> for FunctionInfo {
    type Error = FunctionOverloadError;

    fn try_from(signatures: Vec<SignatureInfo>) -> Result<Self, Self::Error> {
        Self::try_from_iter(signatures)
    }
}

impl<const N: usize> TryFrom<[SignatureInfo; N]> for FunctionInfo {
    type Error = FunctionOverloadError;

    fn try_from(signatures: [SignatureInfo; N]) -> Result<Self, Self::Error> {
        Self::try_from_iter(signatures)
    }
}

/// Type information for the signature of a [`DynamicFunction`] or [`DynamicFunctionMut`].
///
/// Every [`FunctionInfo`] contains one or more [`SignatureInfo`]s.
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
/// [`DynamicFunctionMut`]: crate::func::DynamicFunctionMut
#[derive(Debug, Clone)]
pub struct SignatureInfo {
    name: Option<Cow<'static, str>>,
    args: Box<[ArgInfo]>,
    return_info: ReturnInfo,
}

impl SignatureInfo {
    /// Create a new [`SignatureInfo`] for a function with the given name.
    pub fn named(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: Some(name.into()),
            args: Box::new([]),
            return_info: ReturnInfo::new::<()>(),
        }
    }

    /// Create a new [`SignatureInfo`] with no name.
    ///
    /// For the purposes of debugging and [registration],
    /// it's recommended to use [`Self::named`] instead.
    ///
    /// [registration]: crate::func::FunctionRegistry
    pub fn anonymous() -> Self {
        Self {
            name: None,
            args: Box::new([]),
            return_info: ReturnInfo::new::<()>(),
        }
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
        self.args = IntoIterator::into_iter(self.args)
            .chain(Some(ArgInfo::new::<T>(index).with_name(name)))
            .collect();
        self
    }

    /// Set the arguments of the function.
    ///
    /// This will completely replace any existing arguments.
    ///
    /// It's preferable to use [`Self::with_arg`] to add arguments to the function
    /// as it will automatically set the index of the argument.
    pub fn with_args(mut self, args: Vec<ArgInfo>) -> Self {
        self.args = IntoIterator::into_iter(self.args).chain(args).collect();
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
    /// the default name will always be the full path to the function as returned by [`core::any::type_name`],
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

/// A wrapper around [`FunctionInfo`] that implements [`Debug`] for pretty-printing function information.
///
/// # Example
///
/// ```
/// # use bevy_reflect::func::{FunctionInfo, PrettyPrintFunctionInfo, TypedFunction};
/// #
/// fn add(a: i32, b: i32) -> i32 {
///     a + b
/// }
///
/// let info = add.get_function_info();
///
/// let pretty = PrettyPrintFunctionInfo::new(&info);
/// assert_eq!(format!("{:?}", pretty), "(_: i32, _: i32) -> i32");
/// ```
pub struct PrettyPrintFunctionInfo<'a> {
    info: &'a FunctionInfo,
    include_fn_token: bool,
    include_name: bool,
}

impl<'a> PrettyPrintFunctionInfo<'a> {
    /// Create a new pretty-printer for the given [`FunctionInfo`].
    pub fn new(info: &'a FunctionInfo) -> Self {
        Self {
            info,
            include_fn_token: false,
            include_name: false,
        }
    }

    /// Include the function name in the pretty-printed output.
    pub fn include_name(mut self) -> Self {
        self.include_name = true;
        self
    }

    /// Include the `fn` token in the pretty-printed output.
    pub fn include_fn_token(mut self) -> Self {
        self.include_fn_token = true;
        self
    }
}

impl<'a> Debug for PrettyPrintFunctionInfo<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        if self.include_fn_token {
            write!(f, "fn")?;

            if self.include_name {
                write!(f, " ")?;
            }
        }

        match (self.include_name, self.info.name()) {
            (true, Some(name)) => write!(f, "{name}")?,
            (true, None) => write!(f, "_")?,
            _ => {}
        }

        if self.info.is_overloaded() {
            // `{(arg0: i32, arg1: i32) -> (), (arg0: f32, arg1: f32) -> ()}`
            let mut set = f.debug_set();
            for signature in self.info.signatures() {
                set.entry(&PrettyPrintSignatureInfo::new(signature));
            }
            set.finish()
        } else {
            // `(arg0: i32, arg1: i32) -> ()`
            PrettyPrintSignatureInfo::new(self.info.base()).fmt(f)
        }
    }
}

/// A wrapper around [`SignatureInfo`] that implements [`Debug`] for pretty-printing function signature information.
///
/// # Example
///
/// ```
/// # use bevy_reflect::func::{FunctionInfo, PrettyPrintSignatureInfo, TypedFunction};
/// #
/// fn add(a: i32, b: i32) -> i32 {
///     a + b
/// }
///
/// let info = add.get_function_info();
///
/// let pretty = PrettyPrintSignatureInfo::new(info.base());
/// assert_eq!(format!("{:?}", pretty), "(_: i32, _: i32) -> i32");
/// ```
pub struct PrettyPrintSignatureInfo<'a> {
    info: &'a SignatureInfo,
    include_fn_token: bool,
    include_name: bool,
}

impl<'a> PrettyPrintSignatureInfo<'a> {
    /// Create a new pretty-printer for the given [`SignatureInfo`].
    pub fn new(info: &'a SignatureInfo) -> Self {
        Self {
            info,
            include_fn_token: false,
            include_name: false,
        }
    }

    /// Include the function name in the pretty-printed output.
    pub fn include_name(mut self) -> Self {
        self.include_name = true;
        self
    }

    /// Include the `fn` token in the pretty-printed output.
    pub fn include_fn_token(mut self) -> Self {
        self.include_fn_token = true;
        self
    }
}

impl<'a> Debug for PrettyPrintSignatureInfo<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        if self.include_fn_token {
            write!(f, "fn")?;

            if self.include_name {
                write!(f, " ")?;
            }
        }

        match (self.include_name, self.info.name()) {
            (true, Some(name)) => write!(f, "{name}")?,
            (true, None) => write!(f, "_")?,
            _ => {}
        }

        write!(f, "(")?;

        // We manually write the args instead of using `DebugTuple` to avoid trailing commas
        // and (when used with `{:#?}`) unnecessary newlines
        for (index, arg) in self.info.args().iter().enumerate() {
            if index > 0 {
                write!(f, ", ")?;
            }

            let name = arg.name().unwrap_or("_");
            let ty = arg.type_path();
            write!(f, "{name}: {ty}")?;
        }

        let ret = self.info.return_info().type_path();
        write!(f, ") -> {ret}")
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
/// # use bevy_reflect::func::{ArgList, ReflectFnMut, TypedFunction};
/// #
/// fn print(value: String) {
///   println!("{}", value);
/// }
///
/// let info = print.get_function_info();
/// assert!(info.name().unwrap().ends_with("print"));
/// assert!(info.arg_count().contains(1));
/// assert_eq!(info.base().args()[0].type_path(), "alloc::string::String");
/// assert_eq!(info.base().return_info().type_path(), "()");
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
                FunctionInfo::new(
                    create_info::<Function>()
                        .with_args({
                            let mut _index = 0;
                            vec![
                                $(ArgInfo::new::<$Arg>({
                                    _index += 1;
                                    _index - 1
                                }),)*
                            ]
                        })
                        .with_return_info(ReturnInfo::new::<ReturnType>())
                )
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
            fn function_info() -> FunctionInfo {
                FunctionInfo::new(
                    create_info::<Function>()
                        .with_args({
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
                )
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
                FunctionInfo::new(
                    create_info::<Function>()
                        .with_args({
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
                )
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
                FunctionInfo::new(
                    create_info::<Function>()
                        .with_args({
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
                )
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
/// [`type_name`]: core::any::type_name
fn create_info<F>() -> SignatureInfo {
    let name = core::any::type_name::<F>();

    if name.ends_with("{{closure}}") || name.starts_with("fn(") {
        SignatureInfo::anonymous()
    } else {
        SignatureInfo::named(name)
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
            core::any::type_name_of_val(&add),
            "bevy_reflect::func::info::tests::should_create_function_info::add"
        );

        let info = add.get_function_info();
        assert_eq!(
            info.name().unwrap(),
            "bevy_reflect::func::info::tests::should_create_function_info::add"
        );
        assert_eq!(info.base().arg_count(), 2);
        assert_eq!(info.base().args()[0].type_path(), "i32");
        assert_eq!(info.base().args()[1].type_path(), "i32");
        assert_eq!(info.base().return_info().type_path(), "i32");
    }

    #[test]
    fn should_create_function_pointer_info() {
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        let add = add as fn(i32, i32) -> i32;

        // Sanity check:
        assert_eq!(core::any::type_name_of_val(&add), "fn(i32, i32) -> i32");

        let info = add.get_function_info();
        assert!(info.name().is_none());
        assert_eq!(info.base().arg_count(), 2);
        assert_eq!(info.base().args()[0].type_path(), "i32");
        assert_eq!(info.base().args()[1].type_path(), "i32");
        assert_eq!(info.base().return_info().type_path(), "i32");
    }

    #[test]
    fn should_create_anonymous_function_info() {
        let add = |a: i32, b: i32| a + b;

        // Sanity check:
        assert_eq!(
            core::any::type_name_of_val(&add),
            "bevy_reflect::func::info::tests::should_create_anonymous_function_info::{{closure}}"
        );

        let info = add.get_function_info();
        assert!(info.name().is_none());
        assert_eq!(info.base().arg_count(), 2);
        assert_eq!(info.base().args()[0].type_path(), "i32");
        assert_eq!(info.base().args()[1].type_path(), "i32");
        assert_eq!(info.base().return_info().type_path(), "i32");
    }

    #[test]
    fn should_create_closure_info() {
        let mut total = 0;
        let add = |a: i32, b: i32| total = a + b;

        // Sanity check:
        assert_eq!(
            core::any::type_name_of_val(&add),
            "bevy_reflect::func::info::tests::should_create_closure_info::{{closure}}"
        );

        let info = add.get_function_info();
        assert!(info.name().is_none());
        assert_eq!(info.base().arg_count(), 2);
        assert_eq!(info.base().args()[0].type_path(), "i32");
        assert_eq!(info.base().args()[1].type_path(), "i32");
        assert_eq!(info.base().return_info().type_path(), "()");
    }

    #[test]
    fn should_pretty_print_info() {
        // fn add(a: i32, b: i32) -> i32 {
        //     a + b
        // }
        //
        // let info = add.get_function_info().with_name("add");
        //
        // let pretty = info.pretty_printer();
        // assert_eq!(format!("{:?}", pretty), "(_: i32, _: i32) -> i32");
        //
        // let pretty = info.pretty_printer().include_fn_token();
        // assert_eq!(format!("{:?}", pretty), "fn(_: i32, _: i32) -> i32");
        //
        // let pretty = info.pretty_printer().include_name();
        // assert_eq!(format!("{:?}", pretty), "add(_: i32, _: i32) -> i32");
        //
        // let pretty = info.pretty_printer().include_fn_token().include_name();
        // assert_eq!(format!("{:?}", pretty), "fn add(_: i32, _: i32) -> i32");
    }
}
