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
/// failing to deserialize if the component was moved to be `my_crate::bar::MyComponent`.
/// This trait, through attributes when deriving itself or [`Reflect`], can ensure breaking changes are avoidable.
///
/// The only external factor we rely on for stability when deriving is the [`module_path!`] macro,
/// only if the derive does not provide a `#[type_path = "..."]` attribute.
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
/// struct StableGenericTypePath<T: TypePath>(T);
/// ```
///
/// [utility]: crate::utility
/// [(de)serialization]: crate::serde::UntypedReflectDeserializer
/// [`Reflect`]: crate::Reflect
pub trait TypePath: 'static {
    /// Returns the fully qualified path of the underlying type.
    ///
    /// For [`Option<()>`], this is `core::option::Option::<()>`.
    fn type_path() -> &'static str;

    /// Returns a short pretty-print enabled path to the type.
    ///
    /// For [`Option<()>`], this is `Option<()>`.
    fn short_type_path() -> &'static str;

    /// Returns the name of the type, or [`None`] if it is anonymous.
    ///
    /// For [`Option<()>`], this is `Option`.
    fn type_ident() -> Option<&'static str>;

    /// Returns the name of the crate the type is in, or [`None`] if it is anonymous or a primitive.
    ///
    /// For [`Option<()>`], this is `core`.
    fn crate_name() -> Option<&'static str>;

    /// Returns the path to the moudle the type is in, or [`None`] if it is anonymous or a primitive.
    ///
    /// For [`Option<()>`], this is `core::option`.
    fn module_path() -> Option<&'static str>;
}
