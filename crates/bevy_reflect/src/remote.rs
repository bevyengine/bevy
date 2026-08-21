use crate::Reflect;

/// Marks a type as a [reflectable] wrapper for a remote type.
///
/// This allows types from external libraries (remote types) to be included in reflection.
///
/// The [`#[reflect_remote]`](crate::reflect_remote) attribute macro generates a
/// `#[repr(transparent)]` wrapper and an implementation of this trait. Its conversion methods
/// use the wrapper's transparent representation.
///
/// Manual implementations may use different representation and conversion behavior. The
/// associated `Remote` type identifies the remote type represented by this wrapper.
/// When implementing this trait manually, you need to design carefully about the conversion
/// between `Self` and `Remote` type. For example, if you need to resolve the conversion of
/// `u8` and `bool`, you can set the rule that all the values that more than 1 return false.
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
