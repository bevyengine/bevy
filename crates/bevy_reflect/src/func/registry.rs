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
    /// Attempts to register the given function if it has not yet been registered already.
    ///
    /// This function accepts both functions that satisfy [`IntoFunction`]
    /// and direct [`DynamicFunction`] instances.
    ///
    /// Functions are mapped according to their [name].
    /// If a `DynamicFunction` with the same name already exists, it will not be registered again,
    /// and an error will be returned.
    /// To register the function anyway, overwriting any existing registration, use [`overwrite_registration`] instead.
    ///
    /// [name]: DynamicFunction::name
    /// [`overwrite_registration`]: Self::overwrite_registration
    pub fn register<F, Marker>(
        &mut self,
        function: F,
    ) -> Result<&mut Self, FunctionRegistrationError>
    where
        F: IntoFunction<Marker> + 'static,
    {
        let function = function.into_function();
        let name = function.name().clone();
        self.functions
            .try_insert(name, function)
            .map_err(|err| FunctionRegistrationError::DuplicateName(err.entry.key().clone()))?;

        Ok(self)
    }

    /// Registers the given function, overwriting any existing registration.
    ///
    /// This function accepts both functions that satisfy [`IntoFunction`]
    /// and direct [`DynamicFunction`] instances.
    ///
    /// Functions are mapped according to their [name].
    /// To avoid overwriting existing registrations, it's recommended to use the [`register`] method instead.
    ///
    /// [name]: DynamicFunction::name
    /// [`register`]: Self::register
    pub fn overwrite_registration<F, Marker>(&mut self, function: F)
    where
        F: IntoFunction<Marker> + 'static,
    {
        let function = function.into_function();
        let name = function.name().clone();
        self.functions.insert(name, function);
    }

    /// Get a reference to a registered function by [name].
    ///
    /// [name]: DynamicFunction::name
    pub fn get(&self, name: &str) -> Option<&DynamicFunction> {
        self.functions.get(name)
    }

    /// Get a mutable reference to a registered function by [name].
    ///
    /// [name]: DynamicFunction::name
    pub fn get_mut(&mut self, name: &str) -> Option<&mut DynamicFunction> {
        self.functions.get_mut(name)
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

    /// Returns a mutable iterator over all registered functions.
    pub fn iter_mut(&mut self) -> impl ExactSizeIterator<Item = &mut DynamicFunction> {
        self.functions.values_mut()
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
        registry.register(foo).unwrap();

        let function = registry.get_mut(std::any::type_name_of_val(&foo)).unwrap();
        let value = function.call(ArgList::new()).unwrap().unwrap_owned();
        assert_eq!(value.downcast_ref::<i32>(), Some(&123));
    }

    #[test]
    fn should_register_function_with_custom_name() {
        fn foo() -> i32 {
            123
        }

        let function = foo.into_function().with_name("custom_name");

        let mut registry = FunctionRegistry::default();
        registry.register(function).unwrap();

        let function = registry.get_mut("custom_name").unwrap();
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

        let mut registry = FunctionRegistry::default();
        registry.register(foo).unwrap();
        let result = registry.register(
            bar.into_function()
                .with_name(std::any::type_name_of_val(&foo)),
        );

        assert!(matches!(
            result,
            Err(FunctionRegistrationError::DuplicateName(_))
        ));
        assert_eq!(registry.len(), 1);

        let function = registry.get_mut(std::any::type_name_of_val(&foo)).unwrap();
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

        let mut registry = FunctionRegistry::default();
        registry.register(foo).unwrap();
        registry.overwrite_registration(
            bar.into_function()
                .with_name(std::any::type_name_of_val(&foo)),
        );

        assert_eq!(registry.len(), 1);

        let function = registry.get_mut(std::any::type_name_of_val(&foo)).unwrap();
        let value = function.call(ArgList::new()).unwrap().unwrap_owned();
        assert_eq!(value.downcast_ref::<i32>(), Some(&321));
    }

    #[test]
    fn should_debug_function_registry() {
        fn foo() -> i32 {
            123
        }

        let mut registry = FunctionRegistry::default();
        registry.register(foo).unwrap();

        let debug = format!("{:?}", registry);
        assert_eq!(debug, "{DynamicFunction(fn bevy_reflect::func::registry::tests::should_debug_function_registry::foo() -> i32)}");
    }
}
