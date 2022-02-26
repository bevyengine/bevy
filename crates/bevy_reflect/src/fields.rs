use std::borrow::{Borrow, Cow};

/// The named field of a struct
#[derive(Clone, Debug)]
pub struct NamedField {
    name: Cow<'static, str>,
    type_name: Cow<'static, str>,
}

impl NamedField {
    pub fn new<I: Into<String>>(name: I, type_name: I) -> Self {
        Self {
            name: Cow::Owned(name.into()),
            type_name: Cow::Owned(type_name.into()),
        }
    }

    /// Returns the name of the field
    pub fn name(&self) -> &str {
        self.name.borrow()
    }

    /// Returns the type of the field
    pub fn type_name(&self) -> &str {
        self.type_name.borrow()
    }
}

/// The unnamed field of a tuple or tuple struct
#[derive(Clone, Debug)]
pub struct UnnamedField {
    index: usize,
    type_name: Cow<'static, str>,
}

impl UnnamedField {
    pub fn new<I: Into<String>>(index: usize, type_name: I) -> Self {
        Self {
            index,
            type_name: Cow::Owned(type_name.into()),
        }
    }

    /// Returns the index of the field
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns the type of the field
    pub fn type_name(&self) -> &str {
        self.type_name.borrow()
    }
}
