use alloc::borrow::Cow;
use core::fmt::Debug;
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use bevy_utils::HashMap;

use crate::func::{DynamicCallable, FunctionRegistrationError, IntoCallable};

/// A registry of [reflected callables].
///
/// This is the callable-equivalent to the [`TypeRegistry`].
///
/// All callables must be `'static` as they are stored as [`DynamicCallable<'static>`].
///
/// [reflected callables]: crate::func
/// [`TypeRegistry`]: crate::TypeRegistry
#[derive(Default)]
pub struct FunctionRegistry {
    /// Maps callable [names] to their respective [`DynamicCallables`].
    ///
    /// [names]: DynamicCallable::name
    /// [`DynamicCallables`]: DynamicCallable
    callables: HashMap<Cow<'static, str>, DynamicCallable<'static>>,
}

impl FunctionRegistry {
    /// Attempts to register the given callable.
    ///
    /// This function accepts both callables that satisfy [`IntoCallable`]
    /// and direct [`DynamicCallable`] instances.
    /// The given callable will internally be stored as a [`DynamicCallable<'static>`]
    /// and mapped according to its [name].
    ///
    /// Because the callable must have a name,
    /// anonymous functions (e.g. `|a: i32, b: i32| { a + b }`) and closures must instead
    /// be registered using [`register_with_name`] or manually converted to a [`DynamicCallable`]
    /// and named using [`DynamicCallable::with_name`].
    /// Failure to do so will result in an error being returned.
    ///
    /// If a registered callable with the same name already exists,
    /// it will not be registered again and an error will be returned.
    /// To register the callable anyway, overwriting any existing registration,
    /// use [`overwrite_registration`] instead.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_reflect::func::{FunctionRegistrationError, FunctionRegistry};
    /// fn add(a: i32, b: i32) -> i32 {
    ///     a + b
    /// }
    ///
    /// # fn main() -> Result<(), FunctionRegistrationError> {
    /// let mut registry = FunctionRegistry::default();
    /// registry.register(add)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Callables cannot be registered more than once.
    ///
    /// ```
    /// # use bevy_reflect::func::{FunctionRegistrationError, FunctionRegistry, IntoCallable};
    /// fn add(a: i32, b: i32) -> i32 {
    ///     a + b
    /// }
    ///
    /// let mut registry = FunctionRegistry::default();
    /// registry.register(add).unwrap();
    ///
    /// let result = registry.register(add);
    /// assert!(matches!(result, Err(FunctionRegistrationError::DuplicateName(_))));
    ///
    /// // Note that this simply relies on the name of the function to determine uniqueness.
    /// // You can rename the function to register a separate instance of it.
    /// let result = registry.register(add.into_callable().with_name("add2"));
    /// assert!(result.is_ok());
    /// ```
    ///
    /// Anonymous functions and closures should be registered using [`register_with_name`] or given a name using [`DynamicCallable::with_name`].
    ///
    /// ```
    /// # use bevy_reflect::func::{FunctionRegistrationError, FunctionRegistry, IntoCallable};
    ///
    /// let anonymous = || -> i32 { 123 };
    ///
    /// let mut registry = FunctionRegistry::default();
    ///
    /// let result = registry.register(|a: i32, b: i32| a + b);
    /// assert!(matches!(result, Err(FunctionRegistrationError::MissingName)));
    ///
    /// let result = registry.register_with_name("my_crate::add", |a: i32, b: i32| a + b);
    /// assert!(result.is_ok());
    ///
    /// let result = registry.register((|a: i32, b: i32| a * b).into_callable().with_name("my_crate::mul"));
    /// assert!(result.is_ok());
    /// ```
    ///
    /// [name]: DynamicCallable::name
    /// [`register_with_name`]: Self::register_with_name
    /// [`overwrite_registration`]: Self::overwrite_registration
    pub fn register<F, Marker>(
        &mut self,
        callable: F,
    ) -> Result<&mut Self, FunctionRegistrationError>
    where
        F: IntoCallable<'static, Marker> + 'static,
    {
        let callable = callable.into_callable();
        let name = callable
            .name()
            .ok_or(FunctionRegistrationError::MissingName)?
            .clone();
        self.callables
            .try_insert(name, callable.into_callable())
            .map_err(|err| FunctionRegistrationError::DuplicateName(err.entry.key().clone()))?;

        Ok(self)
    }

    /// Attempts to register the given callable with the given name.
    ///
    /// This function accepts both callables that satisfy [`IntoCallable`]
    /// and direct [`DynamicCallable`] instances.
    /// The given callable will internally be stored as a [`DynamicCallable<'static>`]
    /// with its [name] set to the given name.
    ///
    /// For named functions (e.g. `fn add(a: i32, b: i32) -> i32 { a + b }`) where a custom name is not needed,
    /// it's recommended to use [`register`] instead as the generated name is guaranteed to be unique.
    ///
    /// If a registered callable with the same name already exists,
    /// it will not be registered again and an error will be returned.
    /// To register the callable anyway, overwriting any existing registration,
    /// use [`overwrite_registration_with_name`] instead.
    ///
    /// To avoid conflicts, it's recommended to use a unique name for the callable.
    /// This can be achieved by "namespacing" the callable with a unique identifier,
    /// such as the name of your crate.
    ///
    /// For example, to register a callable, `add`, from a crate, `my_crate`,
    /// you could use the name, `"my_crate::add"`.
    ///
    /// Another approach could be to use the [type name] of the callable,
    /// however, it should be noted that anonymous functions and closures
    ///are not guaranteed to have unique type names.
    ///
    /// This method is a convenience around calling [`IntoCallable::into_callable`] and [`DynamicCallable::with_name`]
    /// on the callable and inserting it into the registry using the [`register`] method.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_reflect::func::{FunctionRegistrationError, FunctionRegistry};
    /// # fn main() -> Result<(), FunctionRegistrationError> {
    /// fn mul(a: i32, b: i32) -> i32 {
    ///     a * b
    /// }
    ///
    /// let div = |a: i32, b: i32| a / b;
    ///
    /// let mut registry = FunctionRegistry::default();
    /// registry
    ///   // Registering an anonymous function with a unique name
    ///   .register_with_name("my_crate::add", |a: i32, b: i32| {
    ///     a + b
    ///   })?
    ///   // Registering an existing function with its type name
    ///   .register_with_name(std::any::type_name_of_val(&mul), mul)?
    ///   // Registering an existing function with a custom name
    ///   .register_with_name("my_crate::mul", mul)?;
    ///   
    /// // Be careful not to register anonymous functions with their type name.
    /// // This code works but registers the function with a non-unique name like `foo::bar::{{closure}}`
    /// registry.register_with_name(std::any::type_name_of_val(&div), div)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Names must be unique.
    ///
    /// ```should_panic
    /// # use bevy_reflect::func::FunctionRegistry;
    /// fn one() {}
    /// fn two() {}
    ///
    /// let mut registry = FunctionRegistry::default();
    /// registry.register_with_name("my_function", one).unwrap();
    ///
    /// // Panic! A function has already been registered with the name "my_function"
    /// registry.register_with_name("my_function", two).unwrap();
    /// ```
    ///
    /// [name]: DynamicCallable::name
    /// [`register`]: Self::register
    /// [`overwrite_registration_with_name`]: Self::overwrite_registration_with_name
    /// [type name]: std::any::type_name
    pub fn register_with_name<F, Marker>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        callable: F,
    ) -> Result<&mut Self, FunctionRegistrationError>
    where
        F: IntoCallable<'static, Marker> + 'static,
    {
        let callable = callable.into_callable().with_name(name);
        self.register(callable)
    }

    /// Registers the given callable, overwriting any existing registration.
    ///
    /// This function accepts both callables that satisfy [`IntoCallable`]
    /// and direct [`DynamicCallable`] instances.
    /// The given callable will internally be stored as a [`DynamicCallable<'static>`]
    /// and mapped according to its [name].
    ///
    /// Because the callable must have a name,
    /// anonymous functions (e.g. `|a: i32, b: i32| { a + b }`) and closures must instead
    /// be registered using [`overwrite_registration_with_name`] or manually converted to a [`DynamicCallable`]
    /// and named using [`DynamicCallable::with_name`].
    /// Failure to do so will result in an error being returned.
    ///
    /// To avoid overwriting existing registrations,
    /// it's recommended to use the [`register`] method instead.
    ///
    /// Returns the previous callable with the same name, if any.
    ///
    /// [name]: DynamicCallable::name
    /// [`overwrite_registration_with_name`]: Self::overwrite_registration_with_name
    /// [`register`]: Self::register
    pub fn overwrite_registration<F, Marker>(
        &mut self,
        function: F,
    ) -> Result<Option<DynamicCallable<'static>>, FunctionRegistrationError>
    where
        F: IntoCallable<'static, Marker> + 'static,
    {
        let function = function.into_callable();
        let name = function
            .name()
            .ok_or(FunctionRegistrationError::MissingName)?
            .clone();

        Ok(self.callables.insert(name, function))
    }

    /// Registers the given callable, overwriting any existing registration.
    ///
    /// This function accepts both callables that satisfy [`IntoCallable`]
    /// and direct [`DynamicCallable`] instances.
    /// The given callable will internally be stored as a [`DynamicCallable<'static>`]
    /// with its [name] set to the given name.
    ///
    /// Callables are mapped according to their name.
    /// To avoid overwriting existing registrations,
    /// it's recommended to use the [`register_with_name`] method instead.
    ///
    /// This method is a convenience around calling [`IntoCallable::into_callable`] and [`DynamicCallable::with_name`]
    /// on the function and inserting it into the registry using the [`overwrite_registration`] method.
    ///
    /// Returns the previous function with the same name, if any.
    ///
    /// [name]: DynamicCallable::name
    /// [`register_with_name`]: Self::register_with_name
    /// [`overwrite_registration`]: Self::overwrite_registration
    pub fn overwrite_registration_with_name<F, Marker>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        callable: F,
    ) -> Option<DynamicCallable<'static>>
    where
        F: IntoCallable<'static, Marker> + 'static,
    {
        let callable = callable.into_callable().with_name(name);
        match self.overwrite_registration(callable) {
            Ok(existing) => existing,
            Err(FunctionRegistrationError::MissingName) => {
                unreachable!("the function should have a name")
            }
            Err(FunctionRegistrationError::DuplicateName(_)) => {
                unreachable!("should overwrite functions with the same name")
            }
        }
    }

    /// Get a reference to a registered callable by [name].
    ///
    /// [name]: DynamicCallable::name
    pub fn get(&self, name: &str) -> Option<&DynamicCallable<'static>> {
        self.callables.get(name)
    }

    /// Returns `true` if a callable with the given [name] is registered.
    ///
    /// [name]: DynamicCallable::name
    pub fn contains(&self, name: &str) -> bool {
        self.callables.contains_key(name)
    }

    /// Returns an iterator over all registered callables.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &DynamicCallable<'static>> {
        self.callables.values()
    }

    /// Returns the number of registered callables.
    pub fn len(&self) -> usize {
        self.callables.len()
    }

    /// Returns `true` if no callables are registered.
    pub fn is_empty(&self) -> bool {
        self.callables.is_empty()
    }
}

impl Debug for FunctionRegistry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_set().entries(self.callables.values()).finish()
    }
}

/// A synchronized wrapper around a [`FunctionRegistry`].
#[derive(Clone, Default, Debug)]
pub struct FunctionRegistryArc {
    pub internal: Arc<RwLock<FunctionRegistry>>,
}

impl FunctionRegistryArc {
    /// Takes a read lock on the underlying [`FunctionRegistry`].
    pub fn read(&self) -> RwLockReadGuard<'_, FunctionRegistry> {
        self.internal.read().unwrap_or_else(PoisonError::into_inner)
    }

    /// Takes a write lock on the underlying [`FunctionRegistry`].
    pub fn write(&self) -> RwLockWriteGuard<'_, FunctionRegistry> {
        self.internal
            .write()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::func::{ArgList, IntoCallable};

    #[test]
    fn should_register_function() {
        fn foo() -> i32 {
            123
        }

        let mut registry = FunctionRegistry::default();
        registry.register(foo).unwrap();

        let function = registry.get(std::any::type_name_of_val(&foo)).unwrap();
        let value = function.call(ArgList::new()).unwrap().unwrap_owned();
        assert_eq!(value.try_downcast_ref::<i32>(), Some(&123));
    }

    #[test]
    fn should_register_anonymous_function() {
        let mut registry = FunctionRegistry::default();
        registry.register_with_name("foo", || 123_i32).unwrap();

        let function = registry.get("foo").unwrap();
        let value = function.call(ArgList::new()).unwrap().unwrap_owned();
        assert_eq!(value.try_downcast_ref::<i32>(), Some(&123));
    }

    #[test]
    fn should_register_closure() {
        let value = 123;
        let foo = move || -> i32 { value };

        let mut registry = FunctionRegistry::default();
        registry.register_with_name("foo", foo).unwrap();

        let function = registry.get("foo").unwrap();
        let value = function.call(ArgList::new()).unwrap().unwrap_owned();
        assert_eq!(value.try_downcast_ref::<i32>(), Some(&123));
    }

    #[test]
    fn should_register_dynamic_function() {
        fn foo() -> i32 {
            123
        }

        let function = foo.into_callable().with_name("custom_name");

        let mut registry = FunctionRegistry::default();
        registry.register(function).unwrap();

        let function = registry.get("custom_name").unwrap();
        let value = function.call(ArgList::new()).unwrap().unwrap_owned();
        assert_eq!(value.try_downcast_ref::<i32>(), Some(&123));
    }

    #[test]
    fn should_register_dynamic_closure() {
        let value = 123;
        let foo = move || -> i32 { value };

        let function = foo.into_callable().with_name("custom_name");

        let mut registry = FunctionRegistry::default();
        registry.register(function).unwrap();

        let function = registry.get("custom_name").unwrap();
        let value = function.call(ArgList::new()).unwrap().unwrap_owned();
        assert_eq!(value.try_downcast_ref::<i32>(), Some(&123));
    }

    #[test]
    fn should_only_register_callable_once() {
        fn foo() -> i32 {
            123
        }

        fn bar() -> i32 {
            321
        }

        let name = std::any::type_name_of_val(&foo);

        let mut registry = FunctionRegistry::default();
        registry.register(foo).unwrap();
        let result = registry.register(bar.into_callable().with_name(name));

        assert!(matches!(
            result,
            Err(FunctionRegistrationError::DuplicateName(_))
        ));
        assert_eq!(registry.len(), 1);

        let function = registry.get(name).unwrap();
        let value = function.call(ArgList::new()).unwrap().unwrap_owned();
        assert_eq!(value.try_downcast_ref::<i32>(), Some(&123));
    }

    #[test]
    fn should_allow_overwriting_registration() {
        fn foo() -> i32 {
            123
        }

        fn bar() -> i32 {
            321
        }

        let name = std::any::type_name_of_val(&foo);

        let mut registry = FunctionRegistry::default();
        registry.register(foo).unwrap();
        registry
            .overwrite_registration(bar.into_callable().with_name(name))
            .unwrap();

        assert_eq!(registry.len(), 1);

        let function = registry.get(name).unwrap();
        let value = function.call(ArgList::new()).unwrap().unwrap_owned();
        assert_eq!(value.try_downcast_ref::<i32>(), Some(&321));
    }

    #[test]
    fn should_error_on_missing_name() {
        let foo = || -> i32 { 123 };

        let function = foo.into_callable();

        let mut registry = FunctionRegistry::default();
        let result = registry.register(function);

        assert!(matches!(
            result,
            Err(FunctionRegistrationError::MissingName)
        ));
    }

    #[test]
    fn should_debug_function_registry() {
        fn foo() -> i32 {
            123
        }

        let mut registry = FunctionRegistry::default();
        registry.register_with_name("foo", foo).unwrap();

        let debug = format!("{:?}", registry);
        assert_eq!(debug, "{DynamicCallable(fn foo() -> i32)}");
    }
}
