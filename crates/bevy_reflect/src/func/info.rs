use crate::func::args::ArgInfo;
use alloc::borrow::Cow;

#[derive(Debug)]
pub struct FunctionInfo {
    name: Option<Cow<'static, str>>,
    args: Vec<ArgInfo>,
}

impl FunctionInfo {
    pub fn new(args: Vec<ArgInfo>) -> Self {
        Self { name: None, args }
    }

    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_args(mut self, args: Vec<ArgInfo>) -> Self {
        self.args = args;
        self
    }

    pub fn args(&self) -> &[ArgInfo] {
        &self.args
    }
}
