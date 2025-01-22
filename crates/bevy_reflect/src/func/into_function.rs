use crate::func::{DynamicFunction, ReflectFn, TypedFunction};

/// A trait for types that can be converted into a [`DynamicFunction`].
///
/// This trait is automatically implemented for any type that implements
/// [`ReflectFn`] and [`TypedFunction`].
///
/// See the [module-level documentation] for more information.
///
/// # Trait Parameters
///
/// This trait has a `Marker` type parameter that is used to get around issues with
/// [unconstrained type parameters] when defining impls with generic arguments or return types.
/// This `Marker` can be any type, provided it doesn't conflict with other implementations.
///
/// Additionally, it has a lifetime parameter, `'env`, that is used to bound the lifetime of the function.
/// For named functions and some closures, this will end up just being `'static`,
/// however, closures that borrow from their environment will have a lifetime bound to that environment.
///
/// [module-level documentation]: crate::func
/// [unconstrained type parameters]: https://doc.rust-lang.org/error_codes/E0207.html
pub trait IntoFunction<'env, Marker> {
    /// Converts [`Self`] into a [`DynamicFunction`].
    fn into_function(self) -> DynamicFunction<'env>;
}

impl<'env, F, Marker1, Marker2> IntoFunction<'env, (Marker1, Marker2)> for F
where
    F: ReflectFn<'env, Marker1> + TypedFunction<Marker2> + Send + Sync + 'env,
{
    fn into_function(self) -> DynamicFunction<'env> {
        DynamicFunction::new(move |args| self.reflect_call(args), Self::function_info())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::func::ArgList;

    #[test]
    fn should_create_dynamic_function_from_closure() {
        let c = 23;
        let func = (|a: i32, b: i32| a + b + c).into_function();
        let args = ArgList::new().with_owned(25_i32).with_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_downcast_ref::<i32>(), Some(&123));
    }

    #[test]
    fn should_create_dynamic_function_from_function() {
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        let func = add.into_function();
        let args = ArgList::new().with_owned(25_i32).with_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_downcast_ref::<i32>(), Some(&100));
    }

    #[test]
    fn should_default_closure_name_to_none() {
        let c = 23;
        let func = (|a: i32, b: i32| a + b + c).into_function();
        assert!(func.name().is_none());
    }
}
