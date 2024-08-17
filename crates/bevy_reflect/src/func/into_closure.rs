use crate::func::{DynamicClosure, ReflectFn, TypedFunction};

/// A trait for types that can be converted into a [`DynamicClosure`].
///
/// This trait is automatically implemented for any type that implements
/// [`ReflectFn`] and [`TypedFunction`].
///
/// See the [module-level documentation] for more information.
///
/// [module-level documentation]: crate::func
pub trait IntoClosure<'env, Marker> {
    /// Converts [`Self`] into a [`DynamicClosure`].
    fn into_closure(self) -> DynamicClosure<'env>;
}

impl<'env, F, Marker1, Marker2> IntoClosure<'env, (Marker1, Marker2)> for F
where
    F: ReflectFn<'env, Marker1> + TypedFunction<Marker2> + Send + Sync + 'env,
{
    fn into_closure(self) -> DynamicClosure<'env> {
        DynamicClosure::new(move |args| self.reflect_call(args), Self::function_info())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::func::ArgList;

    #[test]
    fn should_create_dynamic_closure_from_closure() {
        let c = 23;
        let func = (|a: i32, b: i32| a + b + c).into_closure();
        let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_downcast_ref::<i32>(), Some(&123));
    }

    #[test]
    fn should_create_dynamic_closure_from_function() {
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        let func = add.into_closure();
        let args = ArgList::new().push_owned(25_i32).push_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_downcast_ref::<i32>(), Some(&100));
    }

    #[test]
    fn should_default_closure_name_to_none() {
        let c = 23;
        let func = (|a: i32, b: i32| a + b + c).into_closure();
        assert_eq!(func.info().name(), None);
    }
}
