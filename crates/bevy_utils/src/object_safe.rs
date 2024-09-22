/// Assert that a given `T` is [object safe](https://doc.rust-lang.org/reference/items/traits.html#object-safety).
/// Will fail to compile if that is not the case.
///
/// # Examples
///
/// ```rust
/// # use bevy_utils::assert_object_safe;
/// // Concrete types are always object safe
/// struct MyStruct;
/// assert_object_safe::<MyStruct>();
///
/// // Trait objects are where that safety comes into question.
/// // This trait is object safe...
/// trait ObjectSafe { }
/// assert_object_safe::<dyn ObjectSafe>();
/// ```
///
/// ```compile_fail
/// # use bevy_utils::assert_object_safe;
/// // ...but this trait is not.
/// trait NotObjectSafe {
///     const VALUE: usize;
/// }
/// assert_object_safe::<dyn NotObjectSafe>();
/// // Error: the trait `NotObjectSafe` cannot be made into an object
/// ```
pub fn assert_object_safe<T: ?Sized>() {
    // This space is left intentionally blank. The type parameter T is sufficient to induce a compiler
    // error without a function body.
}
