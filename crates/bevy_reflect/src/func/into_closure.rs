use crate::func::{DynamicCallable, ReflectFn, TypedFunction};

/// A trait for types that can be converted into a [`DynamicCallable`].
///
/// This trait is automatically implemented for any type that implements
/// [`ReflectFn`] and [`TypedFunction`].
///
/// See the [module-level documentation] for more information.
///
/// [module-level documentation]: crate::func
pub trait IntoCallable<'env, Marker> {
    /// Converts [`Self`] into a [`DynamicCallable`].
    fn into_callable(self) -> DynamicCallable<'env>;
}

impl<'env, F, Marker1, Marker2> IntoCallable<'env, (Marker1, Marker2)> for F
where
    F: ReflectFn<'env, Marker1> + TypedFunction<Marker2> + Send + Sync + 'env,
{
    fn into_callable(self) -> DynamicCallable<'env> {
        DynamicCallable::new(move |args| self.reflect_call(args), Self::function_info())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::func::ArgList;

    #[test]
    fn should_create_dynamic_closure_from_closure() {
        let c = 23;
        let func = (|a: i32, b: i32| a + b + c).into_callable();
        let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_downcast_ref::<i32>(), Some(&123));
    }

    #[test]
    fn should_create_dynamic_closure_from_function() {
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        let func = add.into_callable();
        let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_downcast_ref::<i32>(), Some(&100));
    }

    #[test]
    fn should_default_closure_name_to_none() {
        let c = 23;
        let func = (|a: i32, b: i32| a + b + c).into_callable();
        assert_eq!(func.info().name(), None);
    }
}
