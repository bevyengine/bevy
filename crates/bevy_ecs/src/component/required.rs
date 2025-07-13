use alloc::{format, vec::Vec};
use bevy_platform::sync::Arc;
use bevy_ptr::OwningPtr;
use core::fmt::Debug;
use indexmap::{IndexMap, IndexSet};
use thiserror::Error;

use crate::{
    bundle::BundleInfo,
    change_detection::MaybeLocation,
    component::{Component, ComponentId, Components, ComponentsRegistrator, Tick},
    entity::Entity,
    query::DebugCheckedUnwrap as _,
    storage::{SparseSets, Table, TableRow},
};

/// Metadata associated with a required component. See [`Component`] for details.
#[derive(Clone)]
pub struct RequiredComponent {
    /// The constructor used for the required component.
    pub constructor: RequiredComponentConstructor,
}

/// A Required Component constructor. See [`Component`] for details.
#[derive(Clone)]
pub struct RequiredComponentConstructor(
    // Note: this function makes `unsafe` assumptions, so it cannot be public.
    Arc<dyn Fn(&mut Table, &mut SparseSets, Tick, TableRow, Entity, MaybeLocation)>,
);

impl RequiredComponentConstructor {
    /// Creates a new instance of `RequiredComponentConstructor` for the given type
    ///
    /// # Safety
    ///
    /// - `component_id` must be a valid component for type `C`.
    pub unsafe fn new<C: Component>(component_id: ComponentId, constructor: fn() -> C) -> Self {
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
    }

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

/// The collection of metadata for components that are required for a given component.
///
/// For more information, see the "Required Components" section of [`Component`].
#[derive(Default, Clone)]
pub struct RequiredComponents {
    /// The components that are directly required (i.e. excluding inherited ones), in the order of their precedence.
    ///
    /// # Safety
    /// The [`RequiredComponent`] instance associated to each ID must be valid for its component.
    pub(crate) direct: IndexMap<ComponentId, RequiredComponent>,
    /// All the components that are required (i.e. including inherited ones), in depth-first order. Most importantly,
    /// components in this list always appear after all the components that they require.
    ///
    /// Note that the direct components are not necessarily at the end of this list, for example if A and C are directly
    /// requires, and A requires B requires C, then `all` will hold [C, B, A].
    ///
    /// # Safety
    /// The [`RequiredComponent`] instance associated to each ID must be valid for its component.
    pub(crate) all: IndexMap<ComponentId, RequiredComponent>,
}

impl Debug for RequiredComponents {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RequiredComponents")
            .field("direct", &self.direct.keys())
            .field("all", &self.all.keys())
            .finish()
    }
}

impl RequiredComponents {
    /// Registers the [`Component`] `C` as an explicitly required component.
    ///
    /// If the component was not already registered as an explicit required component then it is added
    /// as one, potentially overriding the constructor of a inherited required component, and `true` is returned.
    /// Otherwise `false` is returned.
    ///
    /// # Safety
    ///
    /// - all other components in this [`RequiredComponents`] instance must have been registrated in `components`.
    pub unsafe fn register<C: Component>(
        &mut self,
        components: &mut ComponentsRegistrator<'_>,
        constructor: fn() -> C,
    ) -> bool {
        let id = components.register_component::<C>();
        // SAFETY:
        // - `id` was just registered in `components`;
        // - the caller guarantees all other components were registered in `components`.
        unsafe { self.register_by_id::<C>(id, components, constructor) }
    }

    /// Registers the [`Component`] with the given `component_id` ID as an explicitly required component.
    ///
    /// If the component was not already registered as an explicit required component then it is added
    /// as one, potentially overriding the constructor of a inherited required component, and `true` is returned.
    /// Otherwise `false` is returned.
    ///
    /// # Safety
    ///
    /// - `component_id` must be a valid component in `components` for the type `C`;
    /// - all other components in this [`RequiredComponents`] instance must have been registrated in `components`.
    pub unsafe fn register_by_id<C: Component>(
        &mut self,
        component_id: ComponentId,
        components: &Components,
        constructor: fn() -> C,
    ) -> bool {
        // SAFETY: the caller guarantees that `component_id` is valid for the type `C`.
        let constructor =
            || unsafe { RequiredComponentConstructor::new(component_id, constructor) };

        // SAFETY:
        // - the caller guarantees that `component_id` is valid in `components`
        // - the caller guarantees all other components were registered in `components`;
        // - constructor is guaranteed to create a valid constructor for the component with id `component_id`.
        unsafe { self.register_dynamic_with(component_id, components, constructor) }
    }

    /// Registers the [`Component`] with the given `component_id` ID as an explicitly required component.
    ///
    /// If the component was not already registered as an explicit required component then it is added
    /// as one, potentially overriding the constructor of a inherited required component, and `true` is returned.
    /// Otherwise `false` is returned.
    ///
    /// # Safety
    ///
    /// - `component_id` must be a valid component in `components`;
    /// - all other components in this [`RequiredComponents`] instance must have been registrated in `components`;
    /// - `constructor` must return a [`RequiredComponentConstructor`] that constructs a valid instance for the
    ///   component with ID `component_id`.
    pub unsafe fn register_dynamic_with(
        &mut self,
        component_id: ComponentId,
        components: &Components,
        constructor: impl FnOnce() -> RequiredComponentConstructor,
    ) -> bool {
        // If already registered as a direct required component then bail.
        let entry = match self.direct.entry(component_id) {
            indexmap::map::Entry::Vacant(entry) => entry,
            indexmap::map::Entry::Occupied(_) => return false,
        };

        // Insert into `direct`.
        let constructor = constructor();
        let required_component = RequiredComponent { constructor };
        entry.insert(required_component.clone());

        // Register inherited required components.
        unsafe {
            Self::register_inherited_required_components_unchecked(
                &mut self.all,
                component_id,
                required_component,
                components,
            )
        };

        true
    }

    /// Rebuild the `all` list
    ///
    /// # Safety
    ///
    /// - all components in this [`RequiredComponents`] instance must have been registrated in `components`.
    unsafe fn rebuild_inherited_required_components(&mut self, components: &Components) {
        // Clear `all`, we are re-initializing it.
        self.all.clear();

        // Register all inherited components as if we just registered all components in `direct` one-by-one.
        for (&required_id, required_component) in &self.direct {
            // SAFETY:
            // - the caller guarantees that all components in this instance have been registered in `components`,
            //   meaning both `all` and `required_id` have been registered in `components`;
            // - `required_component` was associated to `required_id`, so it must hold a constructor valid for it.
            unsafe {
                Self::register_inherited_required_components_unchecked(
                    &mut self.all,
                    required_id,
                    required_component.clone(),
                    components,
                )
            }
        }
    }

    /// Registers all the inherited required components from `required_id`.
    ///
    /// # Safety
    ///
    /// - all components in `all` must have been registered in `components`;
    /// - `required_id` must have been registered in `components`;
    /// - `required_component` must hold a valid constructor for the component with id `required_id`.
    unsafe fn register_inherited_required_components_unchecked(
        all: &mut IndexMap<ComponentId, RequiredComponent>,
        required_id: ComponentId,
        required_component: RequiredComponent,
        components: &Components,
    ) {
        // SAFETY: the caller guarantees that `required_id` is valid in `components`.
        let info = unsafe { components.get_info(required_id).debug_checked_unwrap() };

        // Now we need to "recursively" register the
        // Small optimization: if the current required component was already required recursively
        // by an earlier direct required component then all its inherited components have all already
        // been inserted, so let's not try to reinsert them.
        if !all.contains_key(&required_id) {
            for (&inherited_id, inherited_required) in &info.required_components().all {
                // This is an inherited required component: insert it only if not already present.
                // By the invariants of `RequiredComponents`, `info.required_components().all` holds the required
                // components in a depth-first order, and this makes us store teh components in `self.all` also
                // in depth-first order, as long as we don't overwrite existing ones.
                //
                // SAFETY:
                // `inherited_required` was associated to `inherited_id`, so it must have been valid for its component.
                all.entry(inherited_id)
                    .or_insert_with(|| inherited_required.clone());
            }
        }

        // For direct required components:
        // - insert them after inherited components to follow the depth-first order;
        // - insert them unconditionally in order to make their constructor the one that's used.
        // Note that `insert` does not change the order of components, meaning `component_id` will still appear
        // before any other component that requires it.
        //
        // SAFETY: the caller guaranees that `required_component` is valid for the component with ID `required_id`.
        all.insert(required_id, required_component);
    }

    /// Iterates the ids of all required components. This includes recursive required components.
    pub fn iter_ids(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.all.keys().copied()
    }
}

impl Components {
    /// Registers the components in `required_components` as required by `requiree`.
    ///
    /// # Safety
    ///
    /// - `requiree` must have been registered in `self`
    /// - all components in `required_components` must have been registered in `self`.
    pub(crate) unsafe fn register_required_by(
        &mut self,
        requiree: ComponentId,
        required_components: &RequiredComponents,
    ) {
        for &required in required_components.all.keys() {
            // SAFETY: the caller guarantees that all components in `required_components` have been registered in `self`.
            let required_by = unsafe { self.get_required_by_mut(required).debug_checked_unwrap() };
            required_by.insert(requiree);
        }
    }
}

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
    /// - the given component IDs `required` and `requiree` must be valid in `self`;
    /// - the given component ID `required` must be valid for the component type `R`.
    ///
    ///
    /// # Errors
    ///
    /// Returns a [`RequiredComponentsError`] if either of these are true:
    /// - the `required` component is already a *directly* required component for the `requiree`; indirect
    ///   requirements through other components are allowed. In those cases, the more specific
    ///   registration will be used.
    /// - the `requiree` component is already a (possibly indirect) required component for the `required` component.
    pub(crate) unsafe fn register_required_components<R: Component>(
        &mut self,
        requiree: ComponentId,
        required: ComponentId,
        constructor: fn() -> R,
    ) -> Result<(), RequiredComponentsError> {
        // First step: validate inputs and return errors.

        // SAFETY: The caller ensures that the `required` is valid.
        let required_required_components = unsafe {
            self.get_required_components(required)
                .debug_checked_unwrap()
        };

        // Cannot create cyclic requirements.
        if required_required_components.all.contains_key(&requiree) {
            return Err(RequiredComponentsError::CyclicRequirement(
                requiree, required,
            ));
        }

        // SAFETY: The caller ensures that the `requiree` is valid.
        let required_components = unsafe {
            self.get_required_components_mut(requiree)
                .debug_checked_unwrap()
        };

        // Cannot directly require the same component twice.
        if required_components.direct.contains_key(&required) {
            return Err(RequiredComponentsError::DuplicateRegistration(
                requiree, required,
            ));
        }

        // Second step: register the single requirement requiree->required

        // Store the old count of (all) required components. This will help determine which ones are new.
        let old_required_count = required_components.all.len();

        // SAFETY: the caller guarantees that `requiree` and `required` are valid in `self`, with `required` valid for R.
        unsafe { self.register_required_component_single(requiree, required, constructor) };

        // Third step: update the required components and required_by of all the indirect requirements/requirees.

        // Borrow again otherwise it conflicts with the `self.register_required_component_single` call.
        // SAFETY: The caller ensures that the `requiree` is valid.
        let required_components = unsafe {
            self.get_required_components_mut(requiree)
                .debug_checked_unwrap()
        };

        // Optimization: get all the new required components, i.e. those that were appended.
        // Other components that might be inherited when requiring `required` can be safely ignored because
        // any component requiring `requiree` will already transitively require them.
        // Note: the only small exception is for `required` itself, for which we cannot ignore the value of the
        // constructor. But for simplicity we will rebuild any `RequiredComponents`
        let new_required_components = required_components.all[old_required_count..]
            .keys()
            .copied()
            .collect::<IndexSet<_>>();

        // Get all the new requiree components, i.e. `requiree` and all the components that `requiree` is required by.
        // SAFETY: The caller ensures that the `requiree` is valid.
        let new_requiree_components =
            unsafe { self.get_required_by(requiree).debug_checked_unwrap() }.clone();

        // We now need to update the required and required_by components of all the components
        // directly or indirectly involved.
        // Important: we need to be careful about the order we do these operations in.
        // Since computing the required components of some component depends on the required components of
        // other components, and while we do this operations not all required components are up-to-date, we need
        // to ensure we update components in such a way that we update a component after the components it depends on.
        // Luckily this is exactly the depth-first order, which is guaranteed to be the order of `new_requiree_components`.

        // Update the inherited required components of all requiree components (directly or indirectly).
        for &indirect_requiree in &new_requiree_components {
            // Extract the required components to avoid conflicting borrows. Remember to put this back before continuing!
            // SAFETY: `indirect_requiree` comes from `self`, so it must be valid.
            let mut required_components = std::mem::take(unsafe {
                self.get_required_components_mut(indirect_requiree)
                    .debug_checked_unwrap()
            });

            // Rebuild the inherited required components.
            // SAFETY: `required_components` comes from `self`, so all its components must have be valid in `self`.
            unsafe { required_components.rebuild_inherited_required_components(self) };

            // Let's not forget to put back `required_components`!
            // SAFETY: `indirect_requiree` comes from `self`, so it must be valid.
            *unsafe {
                self.get_required_components_mut(indirect_requiree)
                    .debug_checked_unwrap()
            } = required_components;
        }

        // Update the `required_by` of all the components that were newly required (directly or indirectly).
        for &indirect_required in &new_required_components {
            // SAFETY: `indirect_required` comes from `self`, so it must be valid.
            let required_by = unsafe {
                self.get_required_by_mut(indirect_required)
                    .debug_checked_unwrap()
            };

            for &requiree in [&requiree].into_iter().chain(&new_requiree_components) {
                required_by.insert_before(required_by.len(), requiree);
            }
        }

        Ok(())
    }

    /// Register the `required` as a required component in the [`RequiredComponents`] for `requiree`.
    /// This function does not update any other metadata, such as required components of components requiring `requiree`.
    ///
    /// # Safety
    ///
    /// - `requiree` and `required` must be defined in `self`.
    /// - `required` must be a valid component ID for the type `R`.
    unsafe fn register_required_component_single<R: Component>(
        &mut self,
        requiree: ComponentId,
        required: ComponentId,
        constructor: fn() -> R,
    ) {
        // Extract the required components to avoid conflicting borrows. Remember to put this back before returning!
        // SAFETY: The caller ensures that the `requiree` is valid.
        let mut required_components = std::mem::take(unsafe {
            self.get_required_components_mut(requiree)
                .debug_checked_unwrap()
        });

        // Register the required component for the requiree.
        required_components.register_by_id(required, self, constructor);

        // Let's not forget to put back `required_components`!
        // SAFETY: The caller ensures that the `requiree` is valid.
        *unsafe {
            self.get_required_components_mut(requiree)
                .debug_checked_unwrap()
        } = required_components;
    }
}

/// An error returned when the registration of a required component fails.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum RequiredComponentsError {
    /// The component is already a directly required component for the requiree.
    #[error("Component {0:?} already directly requires component {1:?}")]
    DuplicateRegistration(ComponentId, ComponentId),
    /// Adding the given requirement would create a cycle.
    #[error("Cyclic requirement found: the requiree component {0:?} is required by the required component {1:?}")]
    CyclicRequirement(ComponentId, ComponentId),
    /// An archetype with the component that requires other components already exists
    #[error("An archetype with the component {0:?} that requires other components already exists")]
    ArchetypeExists(ComponentId),
}

pub(super) fn enforce_no_required_components_recursion(
    components: &Components,
    recursion_check_stack: &[ComponentId],
    required: ComponentId,
) {
    if let Some(direct_recursion) = recursion_check_stack
        .iter()
        .position(|&id| id == required)
        .map(|index| index == recursion_check_stack.len() - 1)
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
                    components.get_name(required).unwrap().shortname()
                )
            } else {
                "If this is intentional, consider merging the components.".into()
            }
        );
    }
}
