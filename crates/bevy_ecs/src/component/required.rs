use alloc::{format, vec::Vec};
use bevy_platform::{collections::HashMap, sync::Arc};
use bevy_ptr::OwningPtr;
use core::fmt::Debug;
use smallvec::SmallVec;
use thiserror::Error;

use crate::{
    bundle::BundleInfo,
    change_detection::MaybeLocation,
    component::{Component, ComponentId, Components, ComponentsRegistrator, Tick},
    entity::Entity,
    query::DebugCheckedUnwrap as _,
    storage::{SparseSets, Table, TableRow},
};

impl Components {
    /// Registers the given component `R` and [required components] inherited from it as required by `T`.
    ///
    /// When `T` is added to an entity, `R` will also be added if it was not already provided.
    /// The given `constructor` will be used for the creation of `R`.
    ///
    /// [required components]: Component#required-components
    ///
    /// # Safety
    ///
    /// The given component IDs `required` and `requiree` must be valid.
    ///
    /// # Errors
    ///
    /// Returns a [`RequiredComponentsError`] if the `required` component is already a directly required component for the `requiree`.
    ///
    /// Indirect requirements through other components are allowed. In those cases, the more specific
    /// registration will be used.
    pub(crate) unsafe fn register_required_components<R: Component>(
        &mut self,
        requiree: ComponentId,
        required: ComponentId,
        constructor: fn() -> R,
    ) -> Result<(), RequiredComponentsError> {
        // SAFETY: The caller ensures that the `requiree` is valid.
        let required_components = unsafe {
            self.get_required_components_mut(requiree)
                .debug_checked_unwrap()
        };

        // Cannot directly require the same component twice.
        if required_components
            .0
            .get(&required)
            .is_some_and(|c| c.inheritance_depth == 0)
        {
            return Err(RequiredComponentsError::DuplicateRegistration(
                requiree, required,
            ));
        }

        // Register the required component for the requiree.
        // This is a direct requirement with a depth of `0`.
        required_components.register_by_id(required, constructor, 0);

        // Add the requiree to the list of components that require the required component.
        // SAFETY: The component is in the list of required components, so it must exist already.
        let required_by = unsafe { self.get_required_by_mut(required).debug_checked_unwrap() };
        required_by.insert(requiree);

        let mut required_components_tmp = RequiredComponents::default();
        // SAFETY: The caller ensures that the `requiree` and `required` components are valid.
        let inherited_requirements = unsafe {
            self.register_inherited_required_components(
                requiree,
                required,
                &mut required_components_tmp,
            )
        };

        // SAFETY: The caller ensures that the `requiree` is valid.
        let required_components = unsafe {
            self.get_required_components_mut(requiree)
                .debug_checked_unwrap()
        };
        required_components.0.extend(required_components_tmp.0);

        // Propagate the new required components up the chain to all components that require the requiree.
        if let Some(required_by) = self
            .get_required_by(requiree)
            .map(|set| set.iter().copied().collect::<SmallVec<[ComponentId; 8]>>())
        {
            // `required` is now required by anything that `requiree` was required by.
            self.get_required_by_mut(required)
                .unwrap()
                .extend(required_by.iter().copied());
            for &required_by_id in required_by.iter() {
                // SAFETY: The component is in the list of required components, so it must exist already.
                let required_components = unsafe {
                    self.get_required_components_mut(required_by_id)
                        .debug_checked_unwrap()
                };

                // Register the original required component in the "parent" of the requiree.
                // The inheritance depth is 1 deeper than the `requiree` wrt `required_by_id`.
                let depth = required_components.0.get(&requiree).expect("requiree is required by required_by_id, so its required_components must include requiree").inheritance_depth;
                required_components.register_by_id(required, constructor, depth + 1);

                for (component_id, component) in inherited_requirements.iter() {
                    // Register the required component.
                    // The inheritance depth of inherited components is whatever the requiree's
                    // depth is relative to `required_by_id`, plus the inheritance depth of the
                    // inherited component relative to the requiree, plus 1 to account for the
                    // requiree in between.
                    // SAFETY: Component ID and constructor match the ones on the original requiree.
                    //         The original requiree is responsible for making sure the registration is safe.
                    unsafe {
                        required_components.register_dynamic_with(
                            *component_id,
                            component.inheritance_depth + depth + 1,
                            || component.constructor.clone(),
                        );
                    };
                }
            }
        }

        Ok(())
    }

    /// Registers the components inherited from `required` for the given `requiree`,
    /// returning the requirements in a list.
    ///
    /// # Safety
    ///
    /// The given component IDs `requiree` and `required` must be valid.
    unsafe fn register_inherited_required_components(
        &mut self,
        requiree: ComponentId,
        required: ComponentId,
        required_components: &mut RequiredComponents,
    ) -> Vec<(ComponentId, RequiredComponent)> {
        // Get required components inherited from the `required` component.
        // SAFETY: The caller ensures that the `required` component is valid.
        let required_component_info = unsafe { self.get_info(required).debug_checked_unwrap() };
        let inherited_requirements: Vec<(ComponentId, RequiredComponent)> = required_component_info
            .required_components()
            .0
            .iter()
            .map(|(component_id, required_component)| {
                (
                    *component_id,
                    RequiredComponent {
                        constructor: required_component.constructor.clone(),
                        // Add `1` to the inheritance depth since this will be registered
                        // for the component that requires `required`.
                        inheritance_depth: required_component.inheritance_depth + 1,
                    },
                )
            })
            .collect();

        // Register the new required components.
        for (component_id, component) in inherited_requirements.iter() {
            // Register the required component for the requiree.
            // SAFETY: Component ID and constructor match the ones on the original requiree.
            unsafe {
                required_components.register_dynamic_with(
                    *component_id,
                    component.inheritance_depth,
                    || component.constructor.clone(),
                );
            };

            // Add the requiree to the list of components that require the required component.
            // SAFETY: The caller ensures that the required components are valid.
            let required_by = unsafe {
                self.get_required_by_mut(*component_id)
                    .debug_checked_unwrap()
            };
            required_by.insert(requiree);
        }

        inherited_requirements
    }

    /// Registers the given component `R` and [required components] inherited from it as required by `T`,
    /// and adds `T` to their lists of requirees.
    ///
    /// The given `inheritance_depth` determines how many levels of inheritance deep the requirement is.
    /// A direct requirement has a depth of `0`, and each level of inheritance increases the depth by `1`.
    /// Lower depths are more specific requirements, and can override existing less specific registrations.
    ///
    /// This method does *not* register any components as required by components that require `T`.
    ///
    /// [required component]: Component#required-components
    ///
    /// # Safety
    ///
    /// The given component IDs `required` and `requiree` must be valid.
    pub(crate) unsafe fn register_required_components_manual_unchecked<R: Component>(
        &mut self,
        requiree: ComponentId,
        required: ComponentId,
        required_components: &mut RequiredComponents,
        constructor: fn() -> R,
        inheritance_depth: u16,
    ) {
        // Components cannot require themselves.
        if required == requiree {
            return;
        }

        // Register the required component `R` for the requiree.
        required_components.register_by_id(required, constructor, inheritance_depth);

        // Add the requiree to the list of components that require `R`.
        // SAFETY: The caller ensures that the component ID is valid.
        //         Assuming it is valid, the component is in the list of required components, so it must exist already.
        let required_by = unsafe { self.get_required_by_mut(required).debug_checked_unwrap() };
        required_by.insert(requiree);

        self.register_inherited_required_components(requiree, required, required_components);
    }
}

impl<'w> ComponentsRegistrator<'w> {
    // NOTE: This should maybe be private, but it is currently public so that `bevy_ecs_macros` can use it.
    //       We can't directly move this there either, because this uses `Components::get_required_by_mut`,
    //       which is private, and could be equally risky to expose to users.
    /// Registers the given component `R` and [required components] inherited from it as required by `T`,
    /// and adds `T` to their lists of requirees.
    ///
    /// The given `inheritance_depth` determines how many levels of inheritance deep the requirement is.
    /// A direct requirement has a depth of `0`, and each level of inheritance increases the depth by `1`.
    /// Lower depths are more specific requirements, and can override existing less specific registrations.
    ///
    /// The `recursion_check_stack` allows checking whether this component tried to register itself as its
    /// own (indirect) required component.
    ///
    /// This method does *not* register any components as required by components that require `T`.
    ///
    /// Only use this method if you know what you are doing. In most cases, you should instead use [`World::register_required_components`],
    /// or the equivalent method in `bevy_app::App`.
    ///
    /// [required component]: Component#required-components
    #[doc(hidden)]
    pub fn register_required_components_manual<T: Component, R: Component>(
        &mut self,
        required_components: &mut RequiredComponents,
        constructor: fn() -> R,
        inheritance_depth: u16,
        recursion_check_stack: &mut Vec<ComponentId>,
    ) {
        let requiree = self.register_component_checked::<T>(recursion_check_stack);
        let required = self.register_component_checked::<R>(recursion_check_stack);

        // SAFETY: We just created the components.
        unsafe {
            self.register_required_components_manual_unchecked::<R>(
                requiree,
                required,
                required_components,
                constructor,
                inheritance_depth,
            );
        }
    }
}

/// An error returned when the registration of a required component fails.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum RequiredComponentsError {
    /// The component is already a directly required component for the requiree.
    #[error("Component {0:?} already directly requires component {1:?}")]
    DuplicateRegistration(ComponentId, ComponentId),
    /// An archetype with the component that requires other components already exists
    #[error("An archetype with the component {0:?} that requires other components already exists")]
    ArchetypeExists(ComponentId),
}

/// A Required Component constructor. See [`Component`] for details.
#[derive(Clone)]
pub struct RequiredComponentConstructor(
    pub Arc<dyn Fn(&mut Table, &mut SparseSets, Tick, TableRow, Entity, MaybeLocation)>,
);

impl RequiredComponentConstructor {
    /// # Safety
    /// This is intended to only be called in the context of [`BundleInfo::write_components`] to initialized required components.
    /// Calling it _anywhere else_ should be considered unsafe.
    ///
    /// `table_row` and `entity` must correspond to a valid entity that currently needs a component initialized via the constructor stored
    /// on this [`RequiredComponentConstructor`]. The stored constructor must correspond to a component on `entity` that needs initialization.
    /// `table` and `sparse_sets` must correspond to storages on a world where `entity` needs this required component initialized.
    ///
    /// Again, don't call this anywhere but [`BundleInfo::write_components`].
    pub(crate) unsafe fn initialize(
        &self,
        table: &mut Table,
        sparse_sets: &mut SparseSets,
        change_tick: Tick,
        table_row: TableRow,
        entity: Entity,
        caller: MaybeLocation,
    ) {
        (self.0)(table, sparse_sets, change_tick, table_row, entity, caller);
    }
}

/// Metadata associated with a required component. See [`Component`] for details.
#[derive(Clone)]
pub struct RequiredComponent {
    /// The constructor used for the required component.
    pub constructor: RequiredComponentConstructor,

    /// The depth of the component requirement in the requirement hierarchy for this component.
    /// This is used for determining which constructor is used in cases where there are duplicate requires.
    ///
    /// For example, consider the inheritance tree `X -> Y -> Z`, where `->` indicates a requirement.
    /// `X -> Y` and `Y -> Z` are direct requirements with a depth of 0, while `Z` is only indirectly
    /// required for `X` with a depth of `1`.
    ///
    /// In cases where there are multiple conflicting requirements with the same depth, a higher priority
    /// will be given to components listed earlier in the `require` attribute, or to the latest added requirement
    /// if registered at runtime.
    pub inheritance_depth: u16,
}

/// The collection of metadata for components that are required for a given component.
///
/// For more information, see the "Required Components" section of [`Component`].
#[derive(Default, Clone)]
pub struct RequiredComponents(pub(crate) HashMap<ComponentId, RequiredComponent>);

impl Debug for RequiredComponents {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("RequiredComponents")
            .field(&self.0.keys())
            .finish()
    }
}

impl RequiredComponents {
    /// Registers a required component.
    ///
    /// If the component is already registered, it will be overwritten if the given inheritance depth
    /// is smaller than the depth of the existing registration. Otherwise, the new registration will be ignored.
    ///
    /// # Safety
    ///
    /// `component_id` must match the type initialized by `constructor`.
    /// `constructor` _must_ initialize a component for `component_id` in such a way that
    /// matches the storage type of the component. It must only use the given `table_row` or `Entity` to
    /// initialize the storage for `component_id` corresponding to the given entity.
    pub unsafe fn register_dynamic_with(
        &mut self,
        component_id: ComponentId,
        inheritance_depth: u16,
        constructor: impl FnOnce() -> RequiredComponentConstructor,
    ) {
        let entry = self.0.entry(component_id);
        match entry {
            bevy_platform::collections::hash_map::Entry::Occupied(mut occupied) => {
                let current = occupied.get_mut();
                if current.inheritance_depth > inheritance_depth {
                    *current = RequiredComponent {
                        constructor: constructor(),
                        inheritance_depth,
                    }
                }
            }
            bevy_platform::collections::hash_map::Entry::Vacant(vacant) => {
                vacant.insert(RequiredComponent {
                    constructor: constructor(),
                    inheritance_depth,
                });
            }
        }
    }

    /// Registers a required component.
    ///
    /// If the component is already registered, it will be overwritten if the given inheritance depth
    /// is smaller than the depth of the existing registration. Otherwise, the new registration will be ignored.
    pub fn register<C: Component>(
        &mut self,
        components: &mut ComponentsRegistrator,
        constructor: fn() -> C,
        inheritance_depth: u16,
    ) {
        let component_id = components.register_component::<C>();
        self.register_by_id(component_id, constructor, inheritance_depth);
    }

    /// Registers the [`Component`] with the given ID as required if it exists.
    ///
    /// If the component is already registered, it will be overwritten if the given inheritance depth
    /// is smaller than the depth of the existing registration. Otherwise, the new registration will be ignored.
    pub fn register_by_id<C: Component>(
        &mut self,
        component_id: ComponentId,
        constructor: fn() -> C,
        inheritance_depth: u16,
    ) {
        let erased = || {
            RequiredComponentConstructor({
                // `portable-atomic-util` `Arc` is not able to coerce an unsized
                // type like `std::sync::Arc` can. Creating a `Box` first does the
                // coercion.
                //
                // This would be resolved by https://github.com/rust-lang/rust/issues/123430

                #[cfg(not(target_has_atomic = "ptr"))]
                use alloc::boxed::Box;

                type Constructor = dyn for<'a, 'b> Fn(
                    &'a mut Table,
                    &'b mut SparseSets,
                    Tick,
                    TableRow,
                    Entity,
                    MaybeLocation,
                );

                #[cfg(not(target_has_atomic = "ptr"))]
                type Intermediate<T> = Box<T>;

                #[cfg(target_has_atomic = "ptr")]
                type Intermediate<T> = Arc<T>;

                let boxed: Intermediate<Constructor> = Intermediate::new(
                    move |table, sparse_sets, change_tick, table_row, entity, caller| {
                        OwningPtr::make(constructor(), |ptr| {
                            // SAFETY: This will only be called in the context of `BundleInfo::write_components`, which will
                            // pass in a valid table_row and entity requiring a C constructor
                            // C::STORAGE_TYPE is the storage type associated with `component_id` / `C`
                            // `ptr` points to valid `C` data, which matches the type associated with `component_id`
                            unsafe {
                                BundleInfo::initialize_required_component(
                                    table,
                                    sparse_sets,
                                    change_tick,
                                    table_row,
                                    entity,
                                    component_id,
                                    C::STORAGE_TYPE,
                                    ptr,
                                    caller,
                                );
                            }
                        });
                    },
                );

                Arc::from(boxed)
            })
        };

        // SAFETY:
        // `component_id` matches the type initialized by the `erased` constructor above.
        // `erased` initializes a component for `component_id` in such a way that
        // matches the storage type of the component. It only uses the given `table_row` or `Entity` to
        // initialize the storage corresponding to the given entity.
        unsafe { self.register_dynamic_with(component_id, inheritance_depth, erased) };
    }

    /// Iterates the ids of all required components. This includes recursive required components.
    pub fn iter_ids(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.0.keys().copied()
    }

    /// Removes components that are explicitly provided in a given [`Bundle`]. These components should
    /// be logically treated as normal components, not "required components".
    ///
    /// [`Bundle`]: crate::bundle::Bundle
    pub(crate) fn remove_explicit_components(&mut self, components: &[ComponentId]) {
        for component in components {
            self.0.remove(component);
        }
    }

    /// Merges `required_components` into this collection. This only inserts a required component
    /// if it _did not already exist_ *or* if the required component is more specific than the existing one
    /// (in other words, if the inheritance depth is smaller).
    ///
    /// See [`register_dynamic_with`](Self::register_dynamic_with) for details.
    pub(crate) fn merge(&mut self, required_components: &RequiredComponents) {
        for (
            component_id,
            RequiredComponent {
                constructor,
                inheritance_depth,
            },
        ) in required_components.0.iter()
        {
            // SAFETY: This exact registration must have been done on `required_components`, so safety is ensured by that caller.
            unsafe {
                self.register_dynamic_with(*component_id, *inheritance_depth, || {
                    constructor.clone()
                });
            }
        }
    }
}

// NOTE: This should maybe be private, but it is currently public so that `bevy_ecs_macros` can use it.
// This exists as a standalone function instead of being inlined into the component derive macro so as
// to reduce the amount of generated code.
#[doc(hidden)]
pub fn enforce_no_required_components_recursion(
    components: &Components,
    recursion_check_stack: &[ComponentId],
) {
    if let Some((&requiree, check)) = recursion_check_stack.split_last() {
        if let Some(direct_recursion) = check
            .iter()
            .position(|&id| id == requiree)
            .map(|index| index == check.len() - 1)
        {
            panic!(
                "Recursive required components detected: {}\nhelp: {}",
                recursion_check_stack
                    .iter()
                    .map(|id| format!("{}", components.get_name(*id).unwrap().shortname()))
                    .collect::<Vec<_>>()
                    .join(" → "),
                if direct_recursion {
                    format!(
                        "Remove require({}).",
                        components.get_name(requiree).unwrap().shortname()
                    )
                } else {
                    "If this is intentional, consider merging the components.".into()
                }
            );
        }
    }
}
