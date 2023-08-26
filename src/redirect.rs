use std::collections::{HashMap, HashSet};

use naga::{Block, Expression, Function, Handle, Module, Statement};
use thiserror::Error;

use crate::derive::DerivedModule;

#[derive(Debug, Error)]
pub enum RedirectError {
    #[error("can't find function {0} for redirection")]
    FunctionNotFound(String),
    #[error("{0} cannot override {1} due to argument mismatch")]
    ArgumentMismatch(String, String),
    #[error("{0} cannot override {1} due to return type mismatch")]
    ReturnTypeMismatch(String, String),
    #[error("circular reference; can't find an order for : {0}")]
    CircularReference(String),
}

pub struct Redirector {
    module: Module,
}

impl Redirector {
    pub fn new(module: Module) -> Self {
        Self { module }
    }

    fn redirect_block(block: &mut Block, original: Handle<Function>, new: Handle<Function>) {
        for stmt in block.iter_mut() {
            match stmt {
                Statement::Call {
                    ref mut function, ..
                } => {
                    if *function == original {
                        *function = new;
                    }
                }
                Statement::Block(b) => Self::redirect_block(b, original, new),
                Statement::If {
                    condition: _,
                    accept,
                    reject,
                } => {
                    Self::redirect_block(accept, original, new);
                    Self::redirect_block(reject, original, new);
                }
                Statement::Switch { selector: _, cases } => {
                    for case in cases.iter_mut() {
                        Self::redirect_block(&mut case.body, original, new);
                    }
                }
                Statement::Loop {
                    body,
                    continuing,
                    break_if: _,
                } => {
                    Self::redirect_block(body, original, new);
                    Self::redirect_block(continuing, original, new);
                }
                Statement::Emit(_)
                | Statement::Break
                | Statement::Continue
                | Statement::Return { .. }
                | Statement::WorkGroupUniformLoad { .. }
                | Statement::Kill
                | Statement::Barrier(_)
                | Statement::Store { .. }
                | Statement::ImageStore { .. }
                | Statement::Atomic { .. }
                | Statement::RayQuery { .. } => (),
            }
        }
    }

    fn redirect_expr(expr: &mut Expression, original: Handle<Function>, new: Handle<Function>) {
        if let Expression::CallResult(f) = expr {
            if f == &original {
                *expr = Expression::CallResult(new);
            }
        }
    }

    fn redirect_fn(func: &mut Function, original: Handle<Function>, new: Handle<Function>) {
        Self::redirect_block(&mut func.body, original, new);
        for (_, expr) in func.expressions.iter_mut() {
            Self::redirect_expr(expr, original, new);
        }
    }

    /// redirect all calls to the function named `original` with references to the function named `replacement`, except within the replacement function
    /// or in any function contained in the `omit` set.
    /// returns handles to the original and replacement functions.
    /// NB: requires the replacement to be defined in the arena before any calls to the original, or validation will fail.
    pub fn redirect_function(
        &mut self,
        original: &str,
        replacement: &str,
        omit: &HashSet<String>,
    ) -> Result<(Handle<Function>, Handle<Function>), RedirectError> {
        let (h_original, f_original) = self
            .module
            .functions
            .iter()
            .find(|(_, f)| f.name.as_deref() == Some(original))
            .ok_or_else(|| RedirectError::FunctionNotFound(original.to_owned()))?;
        let (h_replacement, f_replacement) = self
            .module
            .functions
            .iter()
            .find(|(_, f)| f.name.as_deref() == Some(replacement))
            .ok_or_else(|| RedirectError::FunctionNotFound(replacement.to_owned()))?;

        for (arg1, arg2) in f_original
            .arguments
            .iter()
            .zip(f_replacement.arguments.iter())
        {
            if arg1.ty != arg2.ty {
                return Err(RedirectError::ArgumentMismatch(
                    original.to_owned(),
                    replacement.to_owned(),
                ));
            }
        }

        if f_original.result.as_ref().map(|r| r.ty) != f_replacement.result.as_ref().map(|r| r.ty) {
            return Err(RedirectError::ReturnTypeMismatch(
                original.to_owned(),
                replacement.to_owned(),
            ));
        }

        for (h_f, f) in self.module.functions.iter_mut() {
            if h_f != h_replacement && !omit.contains(f.name.as_ref().unwrap()) {
                Self::redirect_fn(f, h_original, h_replacement);
            }
        }

        for ep in &mut self.module.entry_points {
            Self::redirect_fn(&mut ep.function, h_original, h_replacement);
        }

        Ok((h_original, h_replacement))
    }

    fn gather_requirements(block: &Block) -> HashSet<Handle<Function>> {
        let mut requirements = HashSet::default();

        for stmt in block.iter() {
            match stmt {
                Statement::Block(b) => requirements.extend(Self::gather_requirements(b)),
                Statement::If { accept, reject, .. } => {
                    requirements.extend(Self::gather_requirements(accept));
                    requirements.extend(Self::gather_requirements(reject));
                }
                Statement::Switch { cases, .. } => {
                    for case in cases {
                        requirements.extend(Self::gather_requirements(&case.body));
                    }
                }
                Statement::Loop {
                    body, continuing, ..
                } => {
                    requirements.extend(Self::gather_requirements(body));
                    requirements.extend(Self::gather_requirements(continuing));
                }
                Statement::Call { function, .. } => {
                    requirements.insert(*function);
                }
                _ => (),
            }
        }

        requirements
    }

    pub fn into_module(self) -> Result<naga::Module, RedirectError> {
        // reorder functions so that dependents come first
        let mut requirements: HashMap<_, _> = self
            .module
            .functions
            .iter()
            .map(|(h_f, f)| (h_f, Self::gather_requirements(&f.body)))
            .collect();

        let mut derived = DerivedModule::default();
        derived.set_shader_source(&self.module, 0);

        while !requirements.is_empty() {
            let start_len = requirements.len();

            let mut added: HashSet<Handle<Function>> = HashSet::new();

            // add anything that has all requirements satisfied
            requirements.retain(|h_f, reqs| {
                if reqs.is_empty() {
                    let func = self.module.functions.try_get(*h_f).unwrap();
                    let span = self.module.functions.get_span(*h_f);
                    derived.import_function(func, span);
                    added.insert(*h_f);
                    false
                } else {
                    true
                }
            });

            // remove things we added from requirements
            for reqs in requirements.values_mut() {
                reqs.retain(|req| !added.contains(req));
            }

            if requirements.len() == start_len {
                return Err(RedirectError::CircularReference(format!(
                    "{:#?}",
                    requirements.keys()
                )));
            }
        }

        Ok(derived.into_module_with_entrypoints())
    }
}

impl TryFrom<Redirector> for naga::Module {
    type Error = RedirectError;

    fn try_from(redirector: Redirector) -> Result<Self, Self::Error> {
        redirector.into_module()
    }
}
