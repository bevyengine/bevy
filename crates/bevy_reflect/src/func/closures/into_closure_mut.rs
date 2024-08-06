use crate::func::{DynamicClosureMut, ReflectFnMut, TypedFunction};

/// A trait for types that can be converted into a [`DynamicClosureMut`].
///
/// This trait is automatically implemented for any type that implements
/// [`ReflectFnMut`] and [`TypedFunction`].
///
/// This trait can be seen as a supertrait of [`IntoClosure`].
///
/// See the [module-level documentation] for more information.
///
/// [`ReflectFn`]: crate::func::ReflectFn
/// [`IntoClosure`]: crate::func::closures::IntoClosure
/// [module-level documentation]: crate::func
pub trait IntoClosureMut<'env, Marker> {
    /// Converts [`Self`] into a [`DynamicClosureMut`].
    fn into_closure_mut(self) -> DynamicClosureMut<'env>;
}

impl<'env, F, Marker1, Marker2> IntoClosureMut<'env, (Marker1, Marker2)> for F
where
    F: ReflectFnMut<'env, Marker1> + TypedFunction<Marker2> + 'env,
{
    fn into_closure_mut(mut self) -> DynamicClosureMut<'env> {
        DynamicClosureMut::new(
            move |args| self.reflect_call_mut(args),
            Self::function_info(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::func::{ArgList, IntoClosure};

    #[test]
    fn should_create_dynamic_closure_mut_from_closure() {
        let c = 23;
        let func = (|a: i32, b: i32| a + b + c).into_closure();
        let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.downcast_ref::<i32>(), Some(&123));
    }

    #[test]
    fn should_create_dynamic_closure_mut_from_closure_with_mutable_capture() {
        let mut total = 0;
        let func = (|a: i32, b: i32| total = a + b).into_closure_mut();
        let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
        func.call_once(args).unwrap();
        assert_eq!(total, 100);
    }

    #[test]
    fn should_create_dynamic_closure_mut_from_function() {
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        let mut func = add.into_closure_mut();
        let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.downcast_ref::<i32>(), Some(&100));
    }

    #[test]
    fn should_default_with_closure_type_name() {
        let mut total = 0;
        let func = (|a: i32, b: i32| total = a + b).into_closure_mut();
        assert_eq!(
            func.info().name(),
            "bevy_reflect::func::closures::into_closure_mut::tests::should_default_with_closure_type_name::{{closure}}"
        );
    }
}
