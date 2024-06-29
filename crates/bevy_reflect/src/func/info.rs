use crate::func::args::{ArgInfo, GetOwnership, Ownership};
use crate::TypePath;
use alloc::borrow::Cow;

/// Type information for a [`DynamicFunction`].
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
#[derive(Debug, Clone)]
pub struct FunctionInfo {
    name: Option<Cow<'static, str>>,
    args: Vec<ArgInfo>,
    return_info: ReturnInfo,
}

impl FunctionInfo {
    /// Create a new [`FunctionInfo`].
    ///
    /// To set the name of the function, use [`Self::with_name`].
    pub fn new() -> Self {
        Self {
            name: None,
            args: Vec::new(),
            return_info: ReturnInfo::new::<()>(),
        }
    }

    /// Set the name of the function.
    ///
    /// Reflected functions are not required to have a name,
    /// so this method must be called manually to set the name.
    pub fn with_name(mut self, name: impl Into<Cow<'static, str>>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the arguments of the function.
    ///
    /// Arguments passed to the function will be validated against the info provided here.
    /// Mismatched arguments may result in the function call returning an [error].
    ///
    /// [error]: crate::func::FunctionError
    pub fn with_args(mut self, args: Vec<ArgInfo>) -> Self {
        self.args = args;
        self
    }

    /// Set the return information of the function.
    pub fn with_return_info(mut self, return_info: ReturnInfo) -> Self {
        self.return_info = return_info;
        self
    }

    /// The name of the function, if it was given one.
    ///
    /// For [`DynamicFunctions`] created using [`IntoFunction`],
    /// the name will always be the full path to the function as returned by [`std::any::type_name`].
    ///
    /// [`DynamicFunctions`]: crate::func::DynamicFunction
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The arguments of the function.
    pub fn args(&self) -> &[ArgInfo] {
        &self.args
    }

    /// The number of arguments the function takes.
    pub fn arg_count(&self) -> usize {
        self.args.len()
    }

    /// The return information of the function.
    pub fn return_info(&self) -> &ReturnInfo {
        &self.return_info
    }
}

impl Default for FunctionInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about the return type of a [`DynamicFunction`].
///
/// [`DynamicFunction`]: crate::func::DynamicFunction
#[derive(Debug, Clone)]
pub struct ReturnInfo {
    type_path: &'static str,
    ownership: Ownership,
}

impl ReturnInfo {
    /// Create a new [`ReturnInfo`] representing the given type, `T`.
    pub fn new<T: TypePath + GetOwnership>() -> Self {
        Self {
            type_path: T::type_path(),
            ownership: T::ownership(),
        }
    }

    /// The type path of the return type.
    pub fn type_path(&self) -> &'static str {
        self.type_path
    }

    /// The ownership of the return type.
    pub fn ownership(&self) -> Ownership {
        self.ownership
    }
}
