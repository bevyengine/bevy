use crate::func::{DynamicFunctionMut, ReflectFnMut, TypedFunction};

/// A trait for types that can be converted into a [`DynamicFunctionMut`].
///
/// This trait is automatically implemented for any type that implements
/// [`ReflectFnMut`] and [`TypedFunction`].
///
/// This trait can be seen as a superset of [`IntoFunction`].
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
/// [`IntoFunction`]: crate::func::IntoFunction
/// [module-level documentation]: crate::func
/// [unconstrained type parameters]: https://doc.rust-lang.org/error_codes/E0207.html
pub trait IntoFunctionMut<'env, Marker> {
    /// Converts [`Self`] into a [`DynamicFunctionMut`].
    fn into_function_mut(self) -> DynamicFunctionMut<'env>;
}

impl<'env, F, Marker1, Marker2> IntoFunctionMut<'env, (Marker1, Marker2)> for F
where
    F: ReflectFnMut<'env, Marker1> + TypedFunction<Marker2> + 'env,
{
    fn into_function_mut(mut self) -> DynamicFunctionMut<'env> {
        DynamicFunctionMut::new(
            move |args| self.reflect_call_mut(args),
            Self::function_info(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::func::{ArgList, IntoFunction};

    #[test]
    fn should_create_dynamic_function_mut_from_closure() {
        let c = 23;
        let func = (|a: i32, b: i32| a + b + c).into_function();
        let args = ArgList::new().with_owned(25_i32).with_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_downcast_ref::<i32>(), Some(&123));
    }

    #[test]
    fn should_create_dynamic_function_mut_from_closure_with_mutable_capture() {
        let mut total = 0;
        let func = (|a: i32, b: i32| total = a + b).into_function_mut();
        let args = ArgList::new().with_owned(25_i32).with_owned(75_i32);
        func.call_once(args).unwrap();
        assert_eq!(total, 100);
    }

    #[test]
    fn should_create_dynamic_function_mut_from_function() {
        fn add(a: i32, b: i32) -> i32 {
            a + b
        }

        let mut func = add.into_function_mut();
        let args = ArgList::new().with_owned(25_i32).with_owned(75_i32);
        let result = func.call(args).unwrap().unwrap_owned();
        assert_eq!(result.try_downcast_ref::<i32>(), Some(&100));
    }

    #[test]
    fn should_default_closure_name_to_none() {
        let mut total = 0;
        let func = (|a: i32, b: i32| total = a + b).into_function_mut();
        assert!(func.name().is_none());
    }
}
