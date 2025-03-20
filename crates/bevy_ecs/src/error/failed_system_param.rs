//! Tools for controlling the error behavior of fallible system parameters.

use crate::system::{FunctionSystem, IntoSystem, SystemParam, SystemParamFunction};

/// State machine for emitting warnings when [system params are invalid](System::validate_param).
#[derive(Clone, Copy)]
pub enum ParamWarnPolicy {
    /// Stop app with a panic.
    Panic,
    /// No warning should ever be emitted.
    Never,
    /// The warning will be emitted once and status will update to [`Self::Never`].
    Warn,
}

impl ParamWarnPolicy {
    /// Advances the warn policy after validation failed.
    #[inline]
    pub(crate) fn advance(&mut self) {
        // Ignore `Panic` case, because it stops execution before this function gets called.
        *self = Self::Never;
    }

    /// Emits a warning about inaccessible system param if policy allows it.
    #[inline]
    pub(crate) fn try_warn<P>(&self, name: &str)
    where
        P: SystemParam,
    {
        match self {
            Self::Panic => panic!(
                "{0} could not access system parameter {1}",
                name,
                disqualified::ShortName::of::<P>()
            ),
            Self::Warn => {
                log::warn!(
                    "{0} did not run because it requested inaccessible system parameter {1}",
                    name,
                    disqualified::ShortName::of::<P>()
                );
            }
            Self::Never => {}
        }
    }
}

/// Trait for manipulating warn policy of systems.
///
/// By default, the fallback behavior of a system with invalid parameters is to panic,
/// although that can be configured globally via the [`GLOBAL_ERROR_HANDLER`](bevy_ecs::error::GLOBAL_ERROR_HANDLER).,
/// found in [`bevy_ecs::error`].
pub trait WithParamWarnPolicy<M, F>
where
    M: 'static,
    F: SystemParamFunction<M>,
    Self: Sized,
{
    /// Set warn policy.
    fn with_param_warn_policy(self, warn_policy: ParamWarnPolicy) -> FunctionSystem<M, F>;

    /// Warn and ignore systems with invalid parameters.
    fn warn_param_missing(self) -> FunctionSystem<M, F> {
        self.with_param_warn_policy(ParamWarnPolicy::Warn)
    }

    /// Silently ignore systems with invalid parameters.
    fn ignore_param_missing(self) -> FunctionSystem<M, F> {
        self.with_param_warn_policy(ParamWarnPolicy::Never)
    }
}

impl<M, F> WithParamWarnPolicy<M, F> for F
where
    M: 'static,
    F: SystemParamFunction<M>,
{
    fn with_param_warn_policy(self, param_warn_policy: ParamWarnPolicy) -> FunctionSystem<M, F> {
        let mut system = IntoSystem::into_system(self);
        system.set_param_warn_policy(param_warn_policy);
        system
    }
}
