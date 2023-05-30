use crate::Reflect;

/// Marks a type as a [reflectable] wrapper for a remote type.
///
/// This allows types from external libraries (remote types) to be included in reflection.
///
/// # Safety
///
/// Remote reflection uses [`transmute`] internally, which is [very unsafe].
/// To ensure proper safety, it is recommended that this trait not be manually implemented.
/// Instead, use the [`#[reflect_remote]`](crate::reflect_remote) attribute macro.
///
/// The macro will ensure that the following safety requirements are met:
/// - `Self` is a single-field tuple struct (i.e. a newtype).
/// - `Self` is `#[repr(transparent)]` over the remote type.
///
/// Additionally, the macro will generate [`Reflect`] and [`FromReflect`] implementations
/// that make safe use of `transmute`.
///
/// # Example
///
/// ```
/// use bevy_reflect_derive::{reflect_remote, Reflect};
///
/// mod some_lib {
///   pub struct TheirType {
///     pub value: u32
///   }
/// }
///
/// #[reflect_remote(some_lib::TheirType)]
/// struct MyType {
///   pub value: u32
/// }
///
/// #[derive(Reflect)]
/// struct MyStruct {
///   #[reflect(remote = "MyType")]
///   data: some_lib::TheirType,
/// }
/// ```
///
/// [reflectable]: Reflect
/// [`transmute`]: core::mem::transmute
/// [highly unsafe]: https://doc.rust-lang.org/nomicon/transmutes.html
/// [`FromReflect`]: crate::FromReflect
#[allow(unsafe_code)]
pub unsafe trait ReflectRemote: Reflect {
    /// The remote type this type represents via reflection.
    type Remote;
}
