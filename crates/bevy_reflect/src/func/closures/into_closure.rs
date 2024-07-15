use crate::func::{DynamicClosure, ReflectFnMut, TypedFunction};

/// A trait for types that can be converted into a [`DynamicClosure`].
///
/// This trait is automatically implemented for any type that implements
/// [`ReflectFnMut`] and [`TypedFunction`].
///
/// Because [`ReflectFn`] is a subtrait of [`ReflectFnMut`],
/// this trait can be seen as a supertrait of [`IntoFunction`].
///
/// See the [module-level documentation] for more information.
///
/// [`ReflectFn`]: crate::func::ReflectFn
/// [`IntoFunction`]: crate::func::IntoFunction
/// [module-level documentation]: crate::func
pub trait IntoClosure<'env, Marker> {
    /// Converts [`Self`] into a [`DynamicClosure`].
    fn into_closure(self) -> DynamicClosure<'env>;
}

impl<'env, F, Marker1, Marker2> IntoClosure<'env, (Marker1, Marker2)> for F
where
    F: ReflectFnMut<'env, Marker1> + TypedFunction<Marker2> + 'env,
{
    fn into_closure(mut self) -> DynamicClosure<'env> {
        DynamicClosure::new(
            move |args, info| self.reflect_call_mut(args, info),
            Self::function_info(),
        )
    }
}
