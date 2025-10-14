use core::{fmt::Debug, hash::Hash};

// TODO: make this generic?
/// An identifier for a draw functions stored.
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct DrawFunctionId(pub u32);
