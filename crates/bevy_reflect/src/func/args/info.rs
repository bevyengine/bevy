use alloc::borrow::Cow;

use crate::func::args::{GetOwnership, Ownership};
use crate::TypePath;

/// Type information for an [`Arg`] used in a [`Function`](super::function::Function)
///
/// [`Arg`]: crate::func::args::Arg
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgInfo {
    index: usize,
    name: Option<Cow<'static, str>>,
    ownership: Ownership,
    type_path: &'static str,
}

impl ArgInfo {
    /// Create a new [`ArgInfo`] with the given argument index and type `T`.
    pub fn new<T: TypePath + GetOwnership>(index: usize) -> Self {
        Self {
            index,
            name: None,
            ownership: T::ownership(),
            type_path: T::type_path(),
        }
    }

    /// Set the name of the argument.
    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// The index of the argument within its function.
    pub fn index(&self) -> usize {
        self.index
    }

    /// The name of the argument, if it was given one.
    ///
    /// Note that this may return `None` even if the argument has a name.
    /// This is because the name needs to be manually set using [`Self::with_name`]
    /// since the name can't be inferred from the function type alone.
    ///
    /// For [`Functions`] created using [`IntoFunction`], the name will always be `None`.
    ///
    /// [`Functions`]: crate::func::Function
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The ownership of the argument.
    pub fn ownership(&self) -> Ownership {
        self.ownership
    }

    pub fn type_path(&self) -> &'static str {
        self.type_path
    }

    /// Get an ID representing the argument.
    ///
    /// This will return `ArgId::Name` if the argument has a name,
    /// otherwise `ArgId::Index`.
    pub fn id(&self) -> ArgId {
        self.name
            .clone()
            .map(ArgId::Name)
            .unwrap_or_else(|| ArgId::Index(self.index))
    }
}

/// A representation of an argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgId {
    /// The index of the argument within its function.
    Index(usize),
    /// The name of the argument.
    Name(Cow<'static, str>),
}
