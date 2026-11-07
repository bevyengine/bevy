//! Tests that reflecting unsound definitions fails to compile. Each failing test is paired with a
//! passing test, so that it doesn't pass because of an unrelated error.

/// Re-stated field types must exactly match the actual field types. A subtype (ex:
/// `for<'a> fn(&'a u8)` for `fn(&'static u8)`) would let reflection write a value of the re-stated
/// type into a field that requires the more general type.
///
/// ```compile_fail,E0308
/// mod other {
///     pub struct Foo {
///         pub f: for<'a> fn(&'a u8),
///     }
/// }
/// use other::Foo;
///
/// bevy_reflect2::impl_reflect! {
///     #[type_path = "other"]
///     struct Foo {
///         #[reflect(ignore)]
///         f: fn(&'static u8),
///     }
/// }
/// ```
///
/// ```
/// mod other {
///     pub struct Foo {
///         pub f: for<'a> fn(&'a u8),
///     }
/// }
/// use other::Foo;
///
/// bevy_reflect2::impl_reflect! {
///     #[type_path = "other"]
///     struct Foo {
///         #[reflect(ignore)]
///         f: for<'a> fn(&'a u8),
///     }
/// }
/// ```
fn _impl_reflect_subtype() {}

/// Fields of packed structs can be unaligned, but reflection accesses fields through references.
///
/// ```compile_fail
/// #[derive(bevy_reflect2::Reflect)]
/// #[repr(C, packed)]
/// struct Foo {
///     a: u8,
///     b: u32,
/// }
/// ```
///
/// ```compile_fail
/// #[derive(bevy_reflect2::Reflect)]
/// #[repr(packed(2))]
/// struct Foo {
///     a: u8,
///     b: u32,
/// }
/// ```
///
/// Opaque types don't reflect their fields, so they can be packed:
///
/// ```
/// #[derive(bevy_reflect2::Reflect)]
/// #[reflect(opaque)]
/// #[repr(C, packed)]
/// struct Foo {
///     a: u8,
///     b: u32,
/// }
///
/// #[derive(bevy_reflect2::Reflect)]
/// #[repr(C, align(8))]
/// struct Bar {
///     a: u8,
///     b: u32,
/// }
/// ```
fn _derive_packed() {}
