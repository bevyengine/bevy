use crate::func::args::{ArgInfo, GetOwnership, Ownership};
use crate::TypePath;
use alloc::borrow::Cow;

#[derive(Debug)]
pub struct FunctionInfo {
    name: Option<Cow<'static, str>>,
    args: Vec<ArgInfo>,
    return_info: ReturnInfo,
}

impl FunctionInfo {
    pub fn new() -> Self {
        Self {
            name: None,
            args: Vec::new(),
            return_info: ReturnInfo::new::<()>(),
        }
    }

    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_args(mut self, args: Vec<ArgInfo>) -> Self {
        self.args = args;
        self
    }

    pub fn with_return_info(mut self, return_info: ReturnInfo) -> Self {
        self.return_info = return_info;
        self
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn args(&self) -> &[ArgInfo] {
        &self.args
    }

    pub fn return_info(&self) -> &ReturnInfo {
        &self.return_info
    }
}

impl Default for FunctionInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct ReturnInfo {
    type_path: &'static str,
    ownership: Ownership,
}

impl ReturnInfo {
    pub fn new<T: TypePath + GetOwnership>() -> Self {
        Self {
            type_path: T::type_path(),
            ownership: T::ownership(),
        }
    }

    pub fn type_path(&self) -> &'static str {
        self.type_path
    }

    pub fn ownership(&self) -> Ownership {
        self.ownership
    }
}
