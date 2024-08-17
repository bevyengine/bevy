use alloc::borrow::Cow;
use core::fmt::Debug;
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use bevy_utils::HashMap;

use crate::func::{DynamicClosure, FunctionRegistrationError, IntoClosure};

/// A registry of [reflected functions].
///
/// This is the function-equivalent to the [`TypeRegistry`].
///
/// All functions and closures are stored as `'static` closures via [`DynamicClosure<'static>`].
/// [`DynamicClosure`] is used instead of [`DynamicFunction`] as it can represent both functions and closures,
/// allowing registered functions to contain captured environment data.
///
/// [reflected functions]: crate::func
/// [`TypeRegistry`]: crate::TypeRegistry
/// [`DynamicFunction`]: crate::func::DynamicFunction
#[derive(Default)]
pub struct FunctionRegistry {
    /// Maps function [names] to their respective [`DynamicClosures`].
    ///
    /// [names]: DynamicClosure::name
    /// [`DynamicClosures`]: DynamicClosure
    functions: HashMap<Cow<'static, str>, DynamicClosure<'static>>,
}

impl FunctionRegistry {
    /// Attempts to register the given function.
    ///
    /// This function accepts both functions/closures that satisfy [`IntoClosure`]
    /// and direct [`DynamicFunction`]/[`DynamicClosure`] instances.
    /// The given function will internally be stored as a [`DynamicClosure<'static>`]
    /// and mapped according to its [name].
    ///
    /// Because the function must have a name,
    /// anonymous functions (e.g. `|a: i32, b: i32| { a + b }`) and closures must instead
    /// be registered using [`register_with_name`] or converted to a [`DynamicClosure`]
    /// and named using [`DynamicClosure::with_name`].
    /// Failure to do so will result in an error being returned.
    ///
    /// If a registered function with the same name already exists,
    /// it will not be registered again and an error will be returned.
    /// To register the function anyway, overwriting any existing registration,
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
    /// Functions cannot be registered more than once.
    ///
    /// ```
    /// # use bevy_reflect::func::{DynamicFunction, FunctionRegistrationError, FunctionRegistry, IntoClosure};
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
    /// let result = registry.register(add.into_closure().with_name("add2"));
    /// assert!(result.is_ok());
    /// ```
    ///
    /// Anonymous functions and closures should be registered using [`register_with_name`] or given a name using [`DynamicClosure::with_name`].
    ///
    /// ```
    /// # use bevy_reflect::func::{DynamicFunction, FunctionRegistrationError, FunctionRegistry, IntoClosure};
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
    /// let result = registry.register((|a: i32, b: i32| a * b).into_closure().with_name("my_crate::mul"));
    /// assert!(result.is_ok());
    /// ```
    ///
    /// [`DynamicFunction`]: crate::func::DynamicFunction
    /// [name]: DynamicClosure::name
    /// [`register_with_name`]: Self::register_with_name
    /// [`overwrite_registration`]: Self::overwrite_registration
    pub fn register<F, Marker>(
        &mut self,
        function: F,
    ) -> Result<&mut Self, FunctionRegistrationError>
    where
        F: IntoClosure<'static, Marker> + 'static,
    {
        let function = function.into_closure();
        let name = function
            .name()
            .ok_or(FunctionRegistrationError::MissingName)?
            .clone();
        self.functions
            .try_insert(name, function.into_closure())
            .map_err(|err| FunctionRegistrationError::DuplicateName(err.entry.key().clone()))?;

        Ok(self)
    }

    /// Attempts to register the given function with the given name.
    ///
    /// This function accepts both functions/closures that satisfy [`IntoClosure`]
    /// and direct [`DynamicFunction`]/[`DynamicClosure`] instances.
    /// The given function will internally be stored as a [`DynamicClosure<'static>`]
    /// with its [name] set to the given name.
    ///
    /// For named functions (e.g. `fn add(a: i32, b: i32) -> i32 { a + b }`) where a custom name is not needed,
    /// it's recommended to use [`register`] instead as the generated name is guaranteed to be unique.
    ///
    /// If a registered function with the same name already exists,
    /// it will not be registered again and an error will be returned.
    /// To register the function anyway, overwriting any existing registration,
    /// use [`overwrite_registration_with_name`] instead.
    ///
    /// To avoid conflicts, it's recommended to use a unique name for the function.
    /// This can be achieved by "namespacing" the function with a unique identifier,
    /// such as the name of your crate.
    ///
    /// For example, to register a function, `add`, from a crate, `my_crate`,
    /// you could use the name, `"my_crate::add"`.
    ///
    /// Another approach could be to use the [type name] of the function,
    /// however, it should be noted that anonymous functions do _not_ have unique type names.
    ///
    /// This method is a convenience around calling [`IntoClosure::into_closure`] and [`DynamicClosure::with_name`]
    /// on the function and inserting it into the registry using the [`register`] method.
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
    /// [`DynamicFunction`]: crate::func::DynamicFunction
    /// [name]: DynamicClosure::name
    /// [`register`]: Self::register
    /// [`overwrite_registration_with_name`]: Self::overwrite_registration_with_name
    /// [type name]: std::any::type_name
    pub fn register_with_name<F, Marker>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        function: F,
    ) -> Result<&mut Self, FunctionRegistrationError>
    where
        F: IntoClosure<'static, Marker> + 'static,
    {
        let function = function.into_closure().with_name(name);
        self.register(function)
    }

    /// Registers the given function, overwriting any existing registration.
    ///
    /// This function accepts both functions/closures that satisfy [`IntoClosure`]
    /// and direct [`DynamicFunction`]/[`DynamicClosure`] instances.
    /// The given function will internally be stored as a [`DynamicClosure<'static>`]
    /// and mapped according to its [name].
    ///
    /// Because the function must have a name,
    /// anonymous functions (e.g. `|a: i32, b: i32| { a + b }`) and closures must instead
    /// be registered using [`overwrite_registration_with_name`] or converted to a [`DynamicClosure`]
    /// and named using [`DynamicClosure::with_name`].
    /// Failure to do so will result in an error being returned.
    ///
    /// To avoid overwriting existing registrations,
    /// it's recommended to use the [`register`] method instead.
    ///
    /// Returns the previous function with the same name, if any.
    ///
    /// [`DynamicFunction`]: crate::func::DynamicFunction
    /// [name]: DynamicClosure::name
    /// [`overwrite_registration_with_name`]: Self::overwrite_registration_with_name
    /// [`register`]: Self::register
    pub fn overwrite_registration<F, Marker>(
        &mut self,
        function: F,
    ) -> Result<Option<DynamicClosure<'static>>, FunctionRegistrationError>
    where
        F: IntoClosure<'static, Marker> + 'static,
    {
        let function = function.into_closure();
        let name = function
            .name()
            .ok_or(FunctionRegistrationError::MissingName)?
            .clone();

        Ok(self.functions.insert(name, function))
    }

    /// Registers the given function, overwriting any existing registration.
    ///
    /// This function accepts both functions/closures that satisfy [`IntoClosure`]
    /// and direct [`DynamicFunction`]/[`DynamicClosure`] instances.
    /// The given function will internally be stored as a [`DynamicClosure<'static>`]
    /// with its [name] set to the given name.
    ///
    /// Functions are mapped according to their name.
    /// To avoid overwriting existing registrations,
    /// it's recommended to use the [`register_with_name`] method instead.
    ///
    /// This method is a convenience around calling [`IntoClosure::into_closure`] and [`DynamicClosure::with_name`]
    /// on the function and inserting it into the registry using the [`overwrite_registration`] method.
    ///
    /// Returns the previous function with the same name, if any.
    ///
    /// [`DynamicFunction`]: crate::func::DynamicFunction
    /// [name]: DynamicClosure::name
    /// [`register_with_name`]: Self::register_with_name
    /// [`overwrite_registration`]: Self::overwrite_registration
    pub fn overwrite_registration_with_name<F, Marker>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
        function: F,
    ) -> Option<DynamicClosure<'static>>
    where
        F: IntoClosure<'static, Marker> + 'static,
    {
        let function = function.into_closure().with_name(name);
        match self.overwrite_registration(function) {
            Ok(existing) => existing,
            Err(FunctionRegistrationError::MissingName) => {
                unreachable!("the function should have a name")
            }
            Err(FunctionRegistrationError::DuplicateName(_)) => {
                unreachable!("should overwrite functions with the same name")
            }
        }
    }

    /// Get a reference to a registered function or closure by [name].
    ///
    /// [name]: DynamicClosure::name
    pub fn get(&self, name: &str) -> Option<&DynamicClosure<'static>> {
        self.functions.get(name)
    }

    /// Returns `true` if a function or closure with the given [name] is registered.
    ///
    /// [name]: DynamicClosure::name
    pub fn contains(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// Returns an iterator over all registered functions/closures.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &DynamicClosure<'static>> {
        self.functions.values()
    }

    /// Returns the number of registered functions/closures.
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Returns `true` if no functions or closures are registered.
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
    use crate::func::{ArgList, IntoFunction};

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

        let function = foo.into_function().with_name("custom_name");

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

        let function = foo.into_closure().with_name("custom_name");

        let mut registry = FunctionRegistry::default();
        registry.register(function).unwrap();

        let function = registry.get("custom_name").unwrap();
        let value = function.call(ArgList::new()).unwrap().unwrap_owned();
        assert_eq!(value.try_downcast_ref::<i32>(), Some(&123));
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
        registry.register(foo).unwrap();
        let result = registry.register(bar.into_function().with_name(name));

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
            .overwrite_registration(bar.into_function().with_name(name))
            .unwrap();

        assert_eq!(registry.len(), 1);

        let function = registry.get(name).unwrap();
        let value = function.call(ArgList::new()).unwrap().unwrap_owned();
        assert_eq!(value.try_downcast_ref::<i32>(), Some(&321));
    }

    #[test]
    fn should_error_on_missing_name() {
        let foo = || -> i32 { 123 };

        let function = foo.into_function();

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
        assert_eq!(debug, "{DynamicClosure(fn foo() -> i32)}");
    }
}
