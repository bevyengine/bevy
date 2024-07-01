use std::fmt;

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
/// ```
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
/// [(de)serialization]: crate::serde::ReflectDeserializer
/// [`Reflect`]: crate::Reflect
/// [`type_path`]: TypePath::type_path
/// [`short_type_path`]: TypePath::short_type_path
/// [`crate_name`]: TypePath::crate_name
/// [`module_path`]: TypePath::module_path
/// [`type_ident`]: TypePath::type_ident
#[diagnostic::on_unimplemented(
    message = "`{Self}` does not have a type path",
    note = "consider annotating `{Self}` with `#[derive(Reflect)]` or `#[derive(TypePath)]`"
)]
pub trait TypePath: 'static {
    /// Returns the fully qualified path of the underlying type.
    ///
    /// Generic parameter types are also fully expanded.
    ///
    /// For `Option<Vec<usize>>`, this is `"core::option::Option<alloc::vec::Vec<usize>>"`.
    fn type_path() -> &'static str;

    /// Returns a short, pretty-print enabled path to the type.
    ///
    /// Generic parameter types are also shortened.
    ///
    /// For `Option<Vec<usize>>`, this is `"Option<Vec<usize>>"`.
    fn short_type_path() -> &'static str;

    /// Returns the name of the type, or [`None`] if it is [anonymous].
    ///
    /// Primitive types will return [`Some`].
    ///
    /// For `Option<Vec<usize>>`, this is `"Option"`.
    ///
    /// [anonymous]: TypePath#anonymity
    fn type_ident() -> Option<&'static str> {
        None
    }

    /// Returns the name of the crate the type is in, or [`None`] if it is [anonymous].
    ///
    /// For `Option<Vec<usize>>`, this is `"core"`.
    ///
    /// [anonymous]: TypePath#anonymity
    fn crate_name() -> Option<&'static str> {
        None
    }

    /// Returns the path to the module the type is in, or [`None`] if it is [anonymous].
    ///
    /// For `Option<Vec<usize>>`, this is `"core::option"`.
    ///
    /// [anonymous]: TypePath#anonymity
    fn module_path() -> Option<&'static str> {
        None
    }
}

/// Dynamic dispatch for [`TypePath`].
///
/// Since this is a supertrait of [`Reflect`] its methods can be called on a `dyn Reflect`.
///
/// [`Reflect`]: crate::Reflect
#[diagnostic::on_unimplemented(
    message = "`{Self}` can not be used as a dynamic type path",
    note = "consider annotating `{Self}` with `#[derive(Reflect)]` or `#[derive(TypePath)]`"
)]
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
    #[inline]
    fn reflect_type_path(&self) -> &str {
        Self::type_path()
    }

    #[inline]
    fn reflect_short_type_path(&self) -> &str {
        Self::short_type_path()
    }

    #[inline]
    fn reflect_type_ident(&self) -> Option<&str> {
        Self::type_ident()
    }

    #[inline]
    fn reflect_crate_name(&self) -> Option<&str> {
        Self::crate_name()
    }

    #[inline]
    fn reflect_module_path(&self) -> Option<&str> {
        Self::module_path()
    }
}

/// Provides dynamic access to all methods on [`TypePath`].
#[derive(Clone, Copy)]
pub struct TypePathTable {
    // Cache the type path as it is likely the only one that will be used.
    type_path: &'static str,
    short_type_path: fn() -> &'static str,
    type_ident: fn() -> Option<&'static str>,
    crate_name: fn() -> Option<&'static str>,
    module_path: fn() -> Option<&'static str>,
}

impl fmt::Debug for TypePathTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TypePathVtable")
            .field("type_path", &self.type_path)
            .field("short_type_path", &(self.short_type_path)())
            .field("type_ident", &(self.type_ident)())
            .field("crate_name", &(self.crate_name)())
            .field("module_path", &(self.module_path)())
            .finish()
    }
}

impl TypePathTable {
    /// Creates a new table from a type.
    pub fn of<T: TypePath + ?Sized>() -> Self {
        Self {
            type_path: T::type_path(),
            short_type_path: T::short_type_path,
            type_ident: T::type_ident,
            crate_name: T::crate_name,
            module_path: T::module_path,
        }
    }

    /// See [`TypePath::type_path`].
    pub fn path(&self) -> &'static str {
        self.type_path
    }

    /// See [`TypePath::short_type_path`].
    pub fn short_path(&self) -> &'static str {
        (self.short_type_path)()
    }

    /// See [`TypePath::type_ident`].
    pub fn ident(&self) -> Option<&'static str> {
        (self.type_ident)()
    }

    /// See [`TypePath::crate_name`].
    pub fn crate_name(&self) -> Option<&'static str> {
        (self.crate_name)()
    }

    /// See [`TypePath::module_path`].
    pub fn module_path(&self) -> Option<&'static str> {
        (self.module_path)()
    }
}
