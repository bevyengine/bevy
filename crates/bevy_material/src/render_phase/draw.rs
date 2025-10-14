use core::{fmt::Debug, hash::Hash};

// TODO: make this generic?
/// An identifier for a [`Draw`] function stored in [`DrawFunctions`].
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct DrawFunctionId(pub u32);
