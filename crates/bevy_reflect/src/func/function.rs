use crate::func::args::{ArgInfo, ArgList};
use crate::func::error::FuncError;
use crate::func::info::FunctionInfo;
use crate::func::return_type::Return;
use alloc::borrow::Cow;
use core::fmt::{Debug, Formatter};
use std::ops::DerefMut;

pub type FunctionResult<'a> = Result<Return<'a>, FuncError>;

pub struct Function {
    info: FunctionInfo,
    func: Box<dyn for<'a> FnMut(ArgList<'a>, &FunctionInfo) -> FunctionResult<'a> + 'static>,
}

impl Debug for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let name = self.info.name().unwrap_or("_");
        write!(f, "Function(fn {name}(")?;

        for (index, arg) in self.info.args().iter().enumerate() {
            let name = arg.name().unwrap_or("_");
            let ty = arg.type_path();
            write!(f, "{name}: {ty}")?;

            if index + 1 < self.info.args().len() {
                write!(f, ", ")?;
            }
        }

        let ret = self.info.return_info().type_path();
        write!(f, ") -> {ret})")
    }
}

impl Function {
    pub fn new<F: for<'a> FnMut(ArgList<'a>, &FunctionInfo) -> FunctionResult<'a> + 'static>(
        func: F,
        info: FunctionInfo,
    ) -> Self {
        Self {
            info,
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

    pub fn call<'a>(&mut self, args: ArgList<'a>) -> FunctionResult<'a> {
        (self.func.deref_mut())(args, &self.info)
    }
}
