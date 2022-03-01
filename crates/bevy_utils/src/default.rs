/// Used to make initializing structs with defaults easier:
/// ```
/// use bevy_utils::default;
///
/// #[derive(Default)]
/// struct Foo {
///   bar: usize,
///   baz: usize,
/// }
///
/// // Normally you would do this:
/// let foo = Foo {
///   bar: 10,
///   ..Default::default()
/// };
///
/// // But now you can do this:
/// let foo = Foo {
///   bar: 10,
///   ..default()
/// };
/// ```
pub fn default<T: Default>() -> T {
    std::default::Default::default()
}
