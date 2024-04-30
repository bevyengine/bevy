use crate::func::args::Arg;
use crate::Reflect;

/// A list of arguments that can be passed to a dynamic [`Function`].
///
/// # Example
///
/// ```
/// # use bevy_reflect::func::{Arg, ArgList};
/// let foo = 123;
/// let bar = 456;
/// let mut baz = 789;
/// let args = ArgList::new()
///   // Push an owned argument
///   .push_owned(foo)
///    // Push an owned and boxed argument
///   .push_boxed(Box::new(foo))
///   // Push a reference argument
///   .push_ref(&bar)
///   // Push a mutable reference argument
///   .push_mut(&mut baz)
///   // Push a manually constructed argument
///   .push(Arg::Ref(&3.14));
/// ```
///
/// [`Function`]: crate::func::Function
#[derive(Default, Debug)]
pub struct ArgList<'a>(Vec<Arg<'a>>);

impl<'a> ArgList<'a> {
    /// Create a new empty list of arguments.
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Push an [`Arg`] onto the list.
    pub fn push(mut self, arg: Arg<'a>) -> Self {
        self.0.push(arg);
        self
    }

    /// Push an [`Arg::Ref`] onto the list with the given reference.
    pub fn push_ref(self, arg: &'a dyn Reflect) -> Self {
        self.push(Arg::Ref(arg))
    }

    /// Push an [`Arg::Mut`] onto the list with the given mutable reference.
    pub fn push_mut(self, arg: &'a mut dyn Reflect) -> Self {
        self.push(Arg::Mut(arg))
    }

    /// Push an [`Arg::Owned`] onto the list with the given owned value.
    pub fn push_owned(self, arg: impl Reflect) -> Self {
        self.push(Arg::Owned(Box::new(arg)))
    }

    /// Push an [`Arg::Owned`] onto the list with the given boxed value.
    pub fn push_boxed(self, arg: Box<dyn Reflect>) -> Self {
        self.push(Arg::Owned(arg))
    }

    /// Returns the number of arguments in the list.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the list of arguments is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Take ownership of the list of arguments.
    pub fn take(self) -> Vec<Arg<'a>> {
        self.0
    }
}
