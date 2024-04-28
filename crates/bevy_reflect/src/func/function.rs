use crate::func::args::{ArgInfo, ArgList};
use crate::func::error::FuncError;
use crate::func::info::FunctionInfo;
use crate::Reflect;
use alloc::borrow::Cow;
use std::ops::DerefMut;

// TODO: Support reference return types
pub type FunctionResult = Result<Option<Box<dyn Reflect>>, FuncError>;

pub struct Function {
    info: FunctionInfo,
    func: Box<dyn for<'a> FnMut(ArgList<'a>, &FunctionInfo) -> FunctionResult + 'static>,
}

impl Function {
    pub fn new<F: for<'a> FnMut(ArgList<'a>, &FunctionInfo) -> FunctionResult + 'static>(
        func: F,
        args: Vec<ArgInfo>,
    ) -> Self {
        Self {
            info: FunctionInfo::new(args),
            func: Box::new(func),
        }
    }

    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.info = self.info.with_name(name);
        self
    }

    pub fn with_args(mut self, args: Vec<ArgInfo>) -> Self {
        self.info = self.info.with_args(args);
        self
    }

    pub fn call<'a>(&mut self, args: ArgList<'a>) -> FunctionResult {
        (self.func.deref_mut())(args, &self.info)
    }
}
