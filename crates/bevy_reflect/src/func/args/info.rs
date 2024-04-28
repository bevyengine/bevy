use alloc::borrow::Cow;
use core::fmt::{Display, Formatter};
use crate::TypePath;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgInfo {
    id: ArgId,
    type_path: &'static str,
}

impl ArgInfo {
    pub fn new<T: TypePath>(id: ArgId) -> Self {
        Self {
            id,
            type_path: T::type_path(),
        }
    }

    pub fn id(&self) -> &ArgId {
        &self.id
    }

    pub fn type_path(&self) -> &'static str {
        self.type_path
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgId {
    Index(usize),
    Name(Cow<'static, str>),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Ownership {
    Ref,
    Mut,
    Owned,
}

impl Display for Ownership {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Ref => write!(f, "reference"),
            Self::Mut => write!(f, "mutable reference"),
            Self::Owned => write!(f, "owned"),
        }
    }
}
