use core::fmt::Display;
use serde::ser::Error;

#[cfg(feature = "debug_stack")]
use std::thread_local;

#[cfg(feature = "debug_stack")]
thread_local! {
    /// The thread-local [`TypeInfoStack`] used for debugging.
    ///
    /// [`TypeInfoStack`]: crate::type_info_stack::TypeInfoStack
    pub(super) static TYPE_INFO_STACK: core::cell::RefCell<crate::type_info_stack::TypeInfoStack> = const { core::cell::RefCell::new(
        crate::type_info_stack::TypeInfoStack::new()
    ) };
}

/// A helper function for generating a custom serialization error message.
///
/// This function should be preferred over [`Error::custom`] as it will include
/// other useful information, such as the [type info stack].
///
/// [type info stack]: crate::type_info_stack::TypeInfoStack
pub(super) fn make_custom_error<E: Error>(msg: impl Display) -> E {
    #[cfg(feature = "debug_stack")]
    return TYPE_INFO_STACK
        .with_borrow(|stack| E::custom(format_args!("{msg} (stack: {stack:?})")));
    #[cfg(not(feature = "debug_stack"))]
    return E::custom(msg);
}
