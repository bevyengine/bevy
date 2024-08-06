use alloc::borrow::Cow;
use core::fmt::Debug;
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use bevy_utils::HashMap;

use crate::func::{DynamicFunction, FunctionRegistrationError, IntoFunction};

/// A registry of [reflected functions].
///
/// This is the function-equivalent to the [`TypeRegistry`].
///
/// [reflected functions]: crate::func
/// [`TypeRegistry`]: crate::TypeRegistry
#[derive(Default)]
pub struct FunctionRegistry {
    /// Maps function [names] to their respective [`DynamicFunctions`].
    ///
    /// [names]: DynamicFunction::name
    /// [`DynamicFunctions`]: DynamicFunction
    functions: HashMap<Cow<'static, str>, DynamicFunction>,
}

impl FunctionRegistry {
    /// Attempts to register the given function with the given name.
    ///
    /// This function accepts both functions that satisfy [`IntoFunction`]
    /// and direct [`DynamicFunction`] instances.
    /// The given function will internally be stored as a [`DynamicFunction`]
    /// with its [name] set to the given name.
    ///
    /// If a registered function with the same name already exists,
    /// it will not be registered again and an error will be returned.
    /// To register the function anyway, overwriting any existing registration,
    /// use [`overwrite_registration`] instead.
    ///
    /// To avoid conflicts, it's recommended to use a unique name for the function.
    /// This can be achieved by either using the function's [type name] or
    /// by "namespacing" the function with a unique identifier,
    /// such as the name of your crate.
    ///
    /// For example, to register a function, `add`, from a crate, `my_crate`,
    /// you could use the name, `"my_crate::add"`.
    ///
    /// This method is a convenience around calling [`IntoFunction::into_function`] and [`DynamicFunction::with_name`]
    /// on the function and inserting it into the registry using the [`register_dynamic`] method.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_reflect::func::{FunctionRegistrationError, FunctionRegistry};
    /// fn mul(a: i32, b: i32) -> i32 {
    ///     a * b
    /// }
    ///
    /// # fn main() -> Result<(), FunctionRegistrationError> {
    /// let mut registry = FunctionRegistry::default();
    /// registry
    ///   // Registering an anonymous function with a unique name
    ///   .register("my_crate::add", |a: i32, b: i32| {
    ///     a + b
    ///   })?
    ///   // Registering an existing function with its type name
    ///   .register(std::any::type_name_of_val(&mul), mul)?
    ///   // Registering an existing function with a custom name
    ///   .register("my_crate::mul", mul)?;
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
    /// registry.register("my_function", one).unwrap();
    ///
    /// // Panic! A function has already been registered with the name "my_function"
    /// registry.register("my_function", two).unwrap();
    /// ```
    ///
    /// [name]: DynamicFunction::name
    /// [`overwrite_registration`]: Self::overwrite_registration
    /// [type name]: std::any::type_name
    /// [`register_dynamic`]: Self::register_dynamic
    pub fn register<F, Marker>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        function: F,
    ) -> Result<&mut Self, FunctionRegistrationError>
    where
        F: IntoFunction<Marker> + 'static,
    {
        let function = function.into_function().with_name(name);
        self.register_dynamic(function)
    }

    /// Attempts to register a [`DynamicFunction`] directly using its [name] as the key.
    ///
    /// If a registered function with the same name already exists,
    /// it will not be registered again and an error will be returned.
    /// To register the function anyway, overwriting any existing registration,
    /// use [`overwrite_registration_dynamic`] instead.
    ///
    /// You can change the name of the function using [`DynamicFunction::with_name`].
    ///
    /// Returns an error if the function is missing a name.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_reflect::func::{DynamicFunction, FunctionRegistrationError, FunctionRegistry, IntoFunction};
    /// fn add(a: i32, b: i32) -> i32 {
    ///   a + b
    /// }
    ///
    /// # fn main() -> Result<(), FunctionRegistrationError> {
    /// let mut registry = FunctionRegistry::default();
    ///
    /// // Register a `DynamicFunction` directly
    /// let function: DynamicFunction = add.into_function();
    /// registry.register_dynamic(function)?;
    ///
    /// // Register a `DynamicFunction` with a custom name
    /// let function: DynamicFunction = add.into_function().with_name("my_crate::add");
    /// registry.register_dynamic(function)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Names must be unique.
    ///
    /// ```should_panic
    /// # use bevy_reflect::func::{DynamicFunction, FunctionRegistry, IntoFunction};
    /// fn one() {}
    /// fn two() {}
    ///
    /// let mut registry = FunctionRegistry::default();
    /// registry.register_dynamic(one.into_function().with_name("my_function")).unwrap();
    ///
    /// // Panic! A function has already been registered with the name "my_function"
    /// registry.register_dynamic(two.into_function().with_name("my_function")).unwrap();
    /// ```
    ///
    /// Names must also be present on the function.
    ///
    /// ```should_panic
    /// # use bevy_reflect::func::{DynamicFunction, FunctionRegistry, IntoFunction};
    ///
    /// let anonymous = || -> i32 { 123 };
    ///
    /// let mut registry = FunctionRegistry::default();
    ///
    /// // Panic! The function is missing a name
    /// registry.register_dynamic(anonymous.into_function()).unwrap();
    /// ```
    ///
    /// [name]: DynamicFunction::name
    /// [`overwrite_registration_dynamic`]: Self::overwrite_registration_dynamic
    pub fn register_dynamic(
        &mut self,
        function: DynamicFunction,
    ) -> Result<&mut Self, FunctionRegistrationError> {
        let name = function
            .name()
            .ok_or(FunctionRegistrationError::MissingName)?
            .clone();
        self.functions
            .try_insert(name, function)
            .map_err(|err| FunctionRegistrationError::DuplicateName(err.entry.key().clone()))?;

        Ok(self)
    }

    /// Registers the given function, overwriting any existing registration.
    ///
    /// This function accepts both functions that satisfy [`IntoFunction`]
    /// and direct [`DynamicFunction`] instances.
    /// The given function will internally be stored as a [`DynamicFunction`]
    /// with its [name] set to the given name.
    ///
    /// Functions are mapped according to their name.
    /// To avoid overwriting existing registrations,
    /// it's recommended to use the [`register`] method instead.
    ///
    /// This method is a convenience around calling [`IntoFunction::into_function`] and [`DynamicFunction::with_name`]
    /// on the function and inserting it into the registry using the [`overwrite_registration_dynamic`] method.
    ///
    /// [name]: DynamicFunction::name
    /// [`register`]: Self::register
    /// [`overwrite_registration_dynamic`]: Self::overwrite_registration_dynamic
    pub fn overwrite_registration<F, Marker>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        function: F,
    ) -> Option<DynamicFunction>
    where
        F: IntoFunction<Marker> + 'static,
    {
        let function = function.into_function().with_name(name);
        match self.overwrite_registration_dynamic(function) {
            Ok(existing) => existing,
            Err(FunctionRegistrationError::MissingName) => {
                unreachable!("the function should have a name")
            }
            Err(FunctionRegistrationError::DuplicateName(_)) => {
                unreachable!("should overwrite functions with the same name")
            }
        }
    }

    /// Registers the given [`DynamicFunction`], overwriting any existing registration.
    ///
    /// The given function will internally be stored as a [`DynamicFunction`]
    /// with its [name] set to the given name.
    ///
    /// Functions are mapped according to their name.
    /// To avoid overwriting existing registrations,
    /// it's recommended to use the [`register_dynamic`] method instead.
    ///
    /// Returns an error if the function is missing a name.
    ///
    /// [name]: DynamicFunction::name
    /// [`register_dynamic`]: Self::register_dynamic
    pub fn overwrite_registration_dynamic(
        &mut self,
        function: DynamicFunction,
    ) -> Result<Option<DynamicFunction>, FunctionRegistrationError> {
        let name = function
            .name()
            .ok_or(FunctionRegistrationError::MissingName)?
            .clone();

        Ok(self.functions.insert(name, function))
    }

    /// Get a reference to a registered function by [name].
    ///
    /// [name]: DynamicFunction::name
    pub fn get(&self, name: &str) -> Option<&DynamicFunction> {
        self.functions.get(name)
    }

    /// Returns `true` if a function with the given [name] is registered.
    ///
    /// [name]: DynamicFunction::name
    pub fn contains(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// Returns an iterator over all registered functions.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &DynamicFunction> {
        self.functions.values()
    }

    /// Returns the number of registered functions.
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Returns `true` if no functions are registered.
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }
}

impl Debug for FunctionRegistry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_set().entries(self.functions.values()).finish()
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
    use crate::func::ArgList;

    #[test]
    fn should_register_function() {
        fn foo() -> i32 {
            123
        }

        let mut registry = FunctionRegistry::default();
        registry.register("foo", foo).unwrap();

        let function = registry.get("foo").unwrap();
        let value = function.call(ArgList::new()).unwrap().unwrap_owned();
        assert_eq!(value.downcast_ref::<i32>(), Some(&123));
    }

    #[test]
    fn should_register_anonymous_function() {
        let mut registry = FunctionRegistry::default();
        registry.register("foo", || 123_i32).unwrap();

        let function = registry.get("foo").unwrap();
        let value = function.call(ArgList::new()).unwrap().unwrap_owned();
        assert_eq!(value.downcast_ref::<i32>(), Some(&123));
    }

    #[test]
    fn should_register_dynamic_function() {
        fn foo() -> i32 {
            123
        }

        let function = foo.into_function().with_name("custom_name");

        let mut registry = FunctionRegistry::default();
        registry.register_dynamic(function).unwrap();

        let function = registry.get("custom_name").unwrap();
        let value = function.call(ArgList::new()).unwrap().unwrap_owned();
        assert_eq!(value.downcast_ref::<i32>(), Some(&123));
    }

    #[test]
    fn should_only_register_function_once() {
        fn foo() -> i32 {
            123
        }

        fn bar() -> i32 {
            321
        }

        let name = std::any::type_name_of_val(&foo);

        let mut registry = FunctionRegistry::default();
        registry.register(name, foo).unwrap();
        let result = registry.register_dynamic(bar.into_function().with_name(name));

        assert!(matches!(
            result,
            Err(FunctionRegistrationError::DuplicateName(_))
        ));
        assert_eq!(registry.len(), 1);

        let function = registry.get(std::any::type_name_of_val(&foo)).unwrap();
        let value = function.call(ArgList::new()).unwrap().unwrap_owned();
        assert_eq!(value.downcast_ref::<i32>(), Some(&123));
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
        registry.register(name, foo).unwrap();
        registry
            .overwrite_registration_dynamic(bar.into_function().with_name(name))
            .unwrap();

        assert_eq!(registry.len(), 1);

        let function = registry.get(std::any::type_name_of_val(&foo)).unwrap();
        let value = function.call(ArgList::new()).unwrap().unwrap_owned();
        assert_eq!(value.downcast_ref::<i32>(), Some(&321));
    }

    #[test]
    fn should_error_on_missing_name() {
        let foo = || -> i32 { 123 };

        let function = foo.into_function();

        let mut registry = FunctionRegistry::default();
        let result = registry.register_dynamic(function);

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
        registry.register("foo", foo).unwrap();

        let debug = format!("{:?}", registry);
        assert_eq!(debug, "{DynamicFunction(fn foo() -> i32)}");
    }
}
