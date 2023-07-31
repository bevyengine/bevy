/// A static accessor to type paths and names.
///
/// The engine uses this trait over [`std::any::type_name`] for stability and flexibility.
///
/// This trait is automatically implemented by the `#[derive(Reflect)]` macro
/// and allows type path information to be processed without an instance of that type.
///
/// Implementors may have difficulty in generating references with static
/// lifetimes. Luckily, this crate comes with some [utility] structs, to make generating these
/// statics much simpler.
///
/// # Stability
///
/// Certain parts of the engine, e.g. [(de)serialization], rely on type paths as identifiers
/// for matching dynamic values to concrete types.
///
/// Using [`std::any::type_name`], a scene containing `my_crate::foo::MyComponent` would break,
/// failing to deserialize if the component was moved from the `foo` module to the `bar` module,
/// becoming `my_crate::bar::MyComponent`.
/// This trait, through attributes when deriving itself or [`Reflect`], can ensure breaking changes are avoidable.
///
/// The only external factor we rely on for stability when deriving is the [`module_path!`] macro,
/// only if the derive does not provide a `#[type_path = "..."]` attribute.
///
/// # Anonymity
///
/// Some methods on this trait return `Option<&'static str>` over `&'static str`
/// because not all types define all parts of a type path, for example the array type `[T; N]`.
///
/// Such types are 'anonymous' in that they have only a defined [`type_path`] and [`short_type_path`]
/// and the methods [`crate_name`], [`module_path`] and [`type_ident`] all return `None`.
///
/// Primitives are treated like anonymous types, except they also have a defined [`type_ident`].
///
/// # Example
///
/// ```rust
/// use bevy_reflect::TypePath;
///
/// // This type path will not change with compiler versions or recompiles,
/// // although it will not be the same if the definition is moved.
/// #[derive(TypePath)]
/// struct NonStableTypePath;
///
/// // This type path will never change, even if the definition is moved.
/// #[derive(TypePath)]
/// #[type_path = "my_crate::foo"]
/// struct StableTypePath;
///
/// // Type paths can have any number of path segments.
/// #[derive(TypePath)]
/// #[type_path = "my_crate::foo::bar::baz"]
/// struct DeeplyNestedStableTypePath;
///
/// // Including just a crate name!
/// #[derive(TypePath)]
/// #[type_path = "my_crate"]
/// struct ShallowStableTypePath;
///
/// // We can also rename the identifier/name of types.
/// #[derive(TypePath)]
/// #[type_path = "my_crate::foo"]
/// #[type_name = "RenamedStableTypePath"]
/// struct NamedStableTypePath;
///
/// // Generics are also supported.
/// #[derive(TypePath)]
/// #[type_path = "my_crate::foo"]
/// struct StableGenericTypePath<T, const N: usize>([T; N]);
/// ```
///
/// [utility]: crate::utility
/// [(de)serialization]: crate::serde::UntypedReflectDeserializer
/// [`Reflect`]: crate::Reflect
/// [`type_path`]: TypePath::type_path
/// [`short_type_path`]: TypePath::short_type_path
/// [`crate_name`]: TypePath::crate_name
/// [`module_path`]: TypePath::module_path
/// [`type_ident`]: TypePath::type_ident
pub trait TypePath: 'static {
    /// Returns the fully qualified path of the underlying type.
    ///
    /// Generic parameter types are also fully expanded.
    ///
    /// For `Option<PhantomData>`, this is `"core::option::Option<core::marker::PhantomData>"`.
    fn type_path() -> &'static str;

    /// Returns a short, pretty-print enabled path to the type.
    ///
    /// Generic parameter types are also shortened.
    ///
    /// For `Option<PhantomData>`, this is `"Option<PhantomData>"`.
    fn short_type_path() -> &'static str;

    /// Returns the name of the type, or [`None`] if it is [anonymous].
    ///
    /// Primitive types will return [`Some`].
    ///
    /// For `Option<PhantomData>`, this is `"Option"`.
    ///
    /// [anonymous]: TypePath#anonymity
    fn type_ident() -> Option<&'static str> {
        None
    }

    /// Returns the name of the crate the type is in, or [`None`] if it is [anonymous].
    ///
    /// For `Option<PhantomData>`, this is `"core"`.
    ///
    /// [anonymous]: TypePath#anonymity
    fn crate_name() -> Option<&'static str> {
        None
    }

    /// Returns the path to the module the type is in, or [`None`] if it is [anonymous].
    ///
    /// For `Option<PhantomData>`, this is `"core::option"`.
    ///
    /// [anonymous]: TypePath#anonymity
    fn module_path() -> Option<&'static str> {
        None
    }
}

/// Dynamic dispatch for [`TypePath`].
pub trait DynamicTypePath {
    /// See [`TypePath::type_path`].
    fn reflect_type_path(&self) -> &str;

    /// See [`TypePath::short_type_path`].
    fn reflect_short_type_path(&self) -> &str;

    /// See [`TypePath::type_ident`].
    fn reflect_type_ident(&self) -> Option<&str>;

    /// See [`TypePath::crate_name`].
    fn reflect_crate_name(&self) -> Option<&str>;

    /// See [`TypePath::module_path`].
    fn reflect_module_path(&self) -> Option<&str>;
}

impl<T: TypePath> DynamicTypePath for T {
    fn reflect_type_path(&self) -> &str {
        Self::type_path()
    }

    fn reflect_short_type_path(&self) -> &str {
        Self::short_type_path()
    }

    fn reflect_type_ident(&self) -> Option<&str> {
        Self::type_ident()
    }

    fn reflect_crate_name(&self) -> Option<&str> {
        Self::crate_name()
    }

    fn reflect_module_path(&self) -> Option<&str> {
        Self::module_path()
    }
}
