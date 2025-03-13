use crate::{component::Tick, error::BevyError, resource::Resource};
use alloc::borrow::Cow;

/// Additional context for a failed system run.
pub struct SystemErrorContext {
    /// The name of the system that failed.
    pub name: Cow<'static, str>,

    /// The last tick that the system was run.
    pub last_run: Tick,
}

/// The error handler of last resort used for [`bevy_ecs::error::Result`]s returned by systems, commands and observers,
/// when an error is not otherwise handled.
///
/// This is stored as a resource in the [`World`](crate::world::World),
/// and defaults to panicking if not set.
///
/// See [`bevy_ecs::error`] for more information on error handling,
/// and [`bevy_ecs::error::handler`] for an assortment of built-in error handlers.
pub struct FallbackErrorHandler(pub fn(BevyError, SystemErrorContext));

impl Resource for FallbackErrorHandler {}

impl Default for FallbackErrorHandler {
    fn default() -> Self {
        Self(panic)
    }
}

macro_rules! inner {
    ($call:path, $e:ident, $c:ident) => {
        $call!("Encountered an error in system `{}`: {:?}", $c.name, $e);
    };
}

/// Error handler that panics with the system error.
#[track_caller]
#[inline]
pub fn panic(error: BevyError, ctx: SystemErrorContext) {
    inner!(panic, error, ctx);
}

/// Error handler that logs the system error at the `error` level.
#[track_caller]
#[inline]
pub fn error(error: BevyError, ctx: SystemErrorContext) {
    inner!(log::error, error, ctx);
}

/// Error handler that logs the system error at the `warn` level.
#[track_caller]
#[inline]
pub fn warn(error: BevyError, ctx: SystemErrorContext) {
    inner!(log::warn, error, ctx);
}

/// Error handler that logs the system error at the `info` level.
#[track_caller]
#[inline]
pub fn info(error: BevyError, ctx: SystemErrorContext) {
    inner!(log::info, error, ctx);
}

/// Error handler that logs the system error at the `debug` level.
#[track_caller]
#[inline]
pub fn debug(error: BevyError, ctx: SystemErrorContext) {
    inner!(log::debug, error, ctx);
}

/// Error handler that logs the system error at the `trace` level.
#[track_caller]
#[inline]
pub fn trace(error: BevyError, ctx: SystemErrorContext) {
    inner!(log::trace, error, ctx);
}

/// Error handler that ignores the system error.
#[track_caller]
#[inline]
pub fn ignore(_: BevyError, _: SystemErrorContext) {}
