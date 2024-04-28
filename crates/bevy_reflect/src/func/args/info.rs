use alloc::borrow::Cow;

use crate::func::args::{GetOwnership, Ownership};
use crate::TypePath;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgInfo {
    index: usize,
    name: Option<Cow<'static, str>>,
    ownership: Ownership,
    type_path: &'static str,
}

impl ArgInfo {
    pub fn new<T: TypePath + GetOwnership>(index: usize) -> Self {
        Self {
            index,
            name: None,
            ownership: T::ownership(),
            type_path: T::type_path(),
        }
    }

    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn index(&self) -> usize {
        self.index
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn ownership(&self) -> Ownership {
        self.ownership
    }

    pub fn id(&self) -> ArgId {
        self.name
            .clone()
            .map(ArgId::Name)
            .unwrap_or_else(|| ArgId::Index(self.index))
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
