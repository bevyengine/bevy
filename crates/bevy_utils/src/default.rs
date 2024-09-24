/// An ergonomic abbreviation for [`Default::default()`] to make initializing structs easier.
/// This is especially helpful when combined with ["struct update syntax"](https://doc.rust-lang.org/book/ch05-01-defining-structs.html#creating-instances-from-other-instances-with-struct-update-syntax).
/// ```
/// use bevy_utils::default;
///
/// #[derive(Default)]
/// struct Foo {
///   a: usize,
///   b: usize,
///   c: usize,
/// }
///
/// // Normally you would initialize a struct with defaults using "struct update syntax"
/// // combined with `Default::default()`. This example sets `Foo::bar` to 10 and the remaining
/// // values to their defaults.
/// let foo = Foo {
///   a: 10,
///   ..Default::default()
/// };
///
/// // But now you can do this, which is equivalent:
/// let foo = Foo {
///   a: 10,
///   ..default()
/// };
/// ```
#[inline]
pub fn default<T: Default>() -> T {
    Default::default()
}
