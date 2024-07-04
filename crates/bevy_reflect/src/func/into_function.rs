use crate::func::function::DynamicFunction;
use crate::func::{ReflectFn, TypedFunction};

/// A trait for types that can be converted into a [`DynamicFunction`].
///
/// This trait is automatically implemented for any type with a `'static` lifetime
/// and also implements [`ReflectFn`] and [`TypedFunction`].
///
/// To handle types such as closures that capture references to their environment,
/// see [`IntoClosure`] instead.
///
/// See the [module-level documentation] for more information.
///
/// [`IntoClosure`]: crate::func::IntoClosure
/// [module-level documentation]: crate::func
pub trait IntoFunction<Marker> {
    /// Converts [`Self`] into a [`DynamicFunction`].
    fn into_function(self) -> DynamicFunction;
}

impl<F, Marker1, Marker2> IntoFunction<(Marker1, Marker2)> for F
where
    F: ReflectFn<'static, Marker1> + TypedFunction<Marker2> + 'static,
{
    fn into_function(self) -> DynamicFunction {
        DynamicFunction::new(
            move |args, info| self.reflect_call(args, info),
            Self::function_info(),
        )
    }
}
