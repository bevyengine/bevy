use crate::Reflect;

/// Marks a type as a [reflectable] wrapper for a remote type.
///
/// This allows types from external libraries (remote types) to be included in reflection.
///
/// # Safety
///
/// It is highly recommended to avoid implementing this trait manually and instead use the
/// [`#[reflect_remote]`](crate::reflect_remote) attribute macro.
/// This is because the trait tends to rely on [`transmute`], which is [very unsafe].
///
/// The macro will ensure that the following safety requirements are met:
/// - `Self` is a single-field tuple struct (i.e. a newtype) containing the remote type.
/// - `Self` is `#[repr(transparent)]` over the remote type.
///
/// Additionally, the macro will automatically generate [`Reflect`] and [`FromReflect`] implementations,
/// along with compile-time assertions to validate that the safety requirements have been met.
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
///   #[reflect(remote = MyType)]
///   data: some_lib::TheirType,
/// }
/// ```
///
/// [reflectable]: Reflect
/// [`transmute`]: core::mem::transmute
/// [very unsafe]: https://doc.rust-lang.org/1.71.0/nomicon/transmutes.html
/// [`FromReflect`]: crate::FromReflect
pub trait ReflectRemote: Reflect {
    /// The remote type this type represents via reflection.
    type Remote;

    /// Converts a reference of this wrapper to a reference of its remote type.
    fn as_remote(&self) -> &Self::Remote;
    /// Converts a mutable reference of this wrapper to a mutable reference of its remote type.
    fn as_remote_mut(&mut self) -> &mut Self::Remote;
    /// Converts this wrapper into its remote type.
    fn into_remote(self) -> Self::Remote;

    /// Converts a reference of the remote type to a reference of this wrapper.
    fn as_wrapper(remote: &Self::Remote) -> &Self;
    /// Converts a mutable reference of the remote type to a mutable reference of this wrapper.
    fn as_wrapper_mut(remote: &mut Self::Remote) -> &mut Self;
    /// Converts the remote type into this wrapper.
    fn into_wrapper(remote: Self::Remote) -> Self;
}
