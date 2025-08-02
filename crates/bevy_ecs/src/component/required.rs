use alloc::{format, vec::Vec};
use bevy_platform::{hash::FixedHasher, sync::Arc};
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
    pub(crate) direct: IndexMap<ComponentId, RequiredComponent, FixedHasher>,
    /// All the components that are required (i.e. including inherited ones), in depth-first order. Most importantly,
    /// components in this list always appear after all the components that they require.
    ///
    /// Note that the direct components are not necessarily at the end of this list, for example if A and C are directly
    /// requires, and A requires B requires C, then `all` will hold [C, B, A].
    ///
    /// # Safety
    /// The [`RequiredComponent`] instance associated to each ID must be valid for its component.
    pub(crate) all: IndexMap<ComponentId, RequiredComponent, FixedHasher>,
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
    /// as one, potentially overriding the constructor of a inherited required component, otherwise panics.
    ///
    /// # Safety
    ///
    /// - all other components in this [`RequiredComponents`] instance must have been registered in `components`.
    unsafe fn register<C: Component>(
        &mut self,
        components: &mut ComponentsRegistrator<'_>,
        constructor: fn() -> C,
    ) {
        let id = components.register_component::<C>();
        // SAFETY:
        // - `id` was just registered in `components`;
        // - the caller guarantees all other components were registered in `components`.
        unsafe { self.register_by_id::<C>(id, components, constructor) };
    }

    /// Registers the [`Component`] with the given `component_id` ID as an explicitly required component.
    ///
    /// If the component was not already registered as an explicit required component then it is added
    /// as one, potentially overriding the constructor of a inherited required component, otherwise panics.
    ///
    /// # Safety
    ///
    /// - `component_id` must be a valid component in `components` for the type `C`;
    /// - all other components in this [`RequiredComponents`] instance must have been registered in `components`.
    unsafe fn register_by_id<C: Component>(
        &mut self,
        component_id: ComponentId,
        components: &Components,
        constructor: fn() -> C,
    ) {
        // SAFETY: the caller guarantees that `component_id` is valid for the type `C`.
        let constructor =
            || unsafe { RequiredComponentConstructor::new(component_id, constructor) };

        // SAFETY:
        // - the caller guarantees that `component_id` is valid in `components`
        // - the caller guarantees all other components were registered in `components`;
        // - constructor is guaranteed to create a valid constructor for the component with id `component_id`.
        unsafe { self.register_dynamic_with(component_id, components, constructor) };
    }

    /// Registers the [`Component`] with the given `component_id` ID as an explicitly required component.
    ///
    /// If the component was not already registered as an explicit required component then it is added
    /// as one, potentially overriding the constructor of a inherited required component, otherwise panics.
    ///
    /// # Safety
    ///
    /// - `component_id` must be a valid component in `components`;
    /// - all other components in `self` must have been registered in `components`;
    /// - `constructor` must return a [`RequiredComponentConstructor`] that constructs a valid instance for the
    ///   component with ID `component_id`.
    unsafe fn register_dynamic_with(
        &mut self,
        component_id: ComponentId,
        components: &Components,
        constructor: impl FnOnce() -> RequiredComponentConstructor,
    ) {
        // If already registered as a direct required component then bail.
        let entry = match self.direct.entry(component_id) {
            indexmap::map::Entry::Vacant(entry) => entry,
            indexmap::map::Entry::Occupied(_) =>
                panic!("Error while registering required component {component_id:?}: already directly required"),
        };

        // Insert into `direct`.
        let constructor = constructor();
        let required_component = RequiredComponent { constructor };
        entry.insert(required_component.clone());

        // Register inherited required components.
        // SAFETY:
        // - the caller guarantees all components that were in `self` have been registered in `components`;
        // - `component_id` has just been added, but is also guaranteed by the called to be valid in `components`.
        unsafe {
            Self::register_inherited_required_components_unchecked(
                &mut self.all,
                component_id,
                required_component,
                components,
            );
        }
    }

    /// Rebuild the `all` list
    ///
    /// # Safety
    ///
    /// - all components in `self` must have been registered in `components`.
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
                );
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
        all: &mut IndexMap<ComponentId, RequiredComponent, FixedHasher>,
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
                // components in a depth-first order, and this makes us store the components in `self.all` also
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
    /// - all components in `required_components` must have been registered in `self`;
    /// - this is called with `requiree` before being called on any component requiring `requiree`.
    pub(crate) unsafe fn register_required_by(
        &mut self,
        requiree: ComponentId,
        required_components: &RequiredComponents,
    ) {
        for &required in required_components.all.keys() {
            // SAFETY: the caller guarantees that all components in `required_components` have been registered in `self`.
            let required_by = unsafe { self.get_required_by_mut(required).debug_checked_unwrap() };
            // This preserves the invariant of `required_by` because:
            // - components requiring `required` and required by `requiree` are already initialized at this point
            //   and hence registered in `required_by` before `requiree`;
            // - components requiring `requiree` cannot exist yet, as this is called on `requiree` before them.
            required_by.insert(requiree);
        }
    }

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

        // SAFETY: the caller guarantees that `requiree` is valid in `self`.
        self.required_components_scope(requiree, |this, required_components| {
            // SAFETY: the caller guarantees that `required` is valid for type `R` in `self`
            unsafe { required_components.register_by_id(required, this, constructor) };
        });

        // Third step: update the required components and required_by of all the indirect requirements/requirees.

        // Borrow again otherwise it conflicts with the `self.required_components_scope` call.
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
            .collect::<Vec<_>>();

        // Get all the new requiree components, i.e. `requiree` and all the components that `requiree` is required by.
        // SAFETY: The caller ensures that the `requiree` is valid.
        let requiree_required_by = unsafe { self.get_required_by(requiree).debug_checked_unwrap() };
        let new_requiree_components = [requiree]
            .into_iter()
            .chain(requiree_required_by.iter().copied())
            .collect::<IndexSet<_, FixedHasher>>();

        // We now need to update the required and required_by components of all the components
        // directly or indirectly involved.
        // Important: we need to be careful about the order we do these operations in.
        // Since computing the required components of some component depends on the required components of
        // other components, and while we do this operations not all required components are up-to-date, we need
        // to ensure we update components in such a way that we update a component after the components it depends on.
        // Luckily, `new_requiree_components` comes from `ComponentInfo::required_by`, which guarantees an order
        // with that property.

        // Update the inherited required components of all requiree components (directly or indirectly).
        // Skip the first one (requiree) because we already updates it.
        for &indirect_requiree in &new_requiree_components[1..] {
            // SAFETY: `indirect_requiree` comes from `self` so it must be valid.
            self.required_components_scope(indirect_requiree, |this, required_components| {
                // Rebuild the inherited required components.
                // SAFETY: `required_components` comes from `self`, so all its components must have be valid in `self`.
                unsafe { required_components.rebuild_inherited_required_components(this) };
            });
        }

        // Update the `required_by` of all the components that were newly required (directly or indirectly).
        for &indirect_required in &new_required_components {
            // SAFETY: `indirect_required` comes from `self`, so it must be valid.
            let required_by = unsafe {
                self.get_required_by_mut(indirect_required)
                    .debug_checked_unwrap()
            };

            // Remove and re-add all the components in `new_requiree_components`
            // This preserves the invariant of `required_by` because `new_requiree_components`
            // satisfies its invariant, due to being `requiree` followed by its `required_by` components,
            // and because any component not in `new_requiree_components` cannot require a component in it,
            // since if that was the case it would appear in the `required_by` for `requiree`.
            required_by.retain(|id| !new_requiree_components.contains(id));
            required_by.extend(&new_requiree_components);
        }

        Ok(())
    }

    /// Temporarily take out the [`RequiredComponents`] of the component with id `component_id`
    /// and runs the given closure with mutable access to `self` and the given [`RequiredComponents`].
    ///
    /// SAFETY:
    ///
    /// `component_id` is valid in `self.components`
    unsafe fn required_components_scope<R>(
        &mut self,
        component_id: ComponentId,
        f: impl FnOnce(&mut Self, &mut RequiredComponents) -> R,
    ) -> R {
        struct DropGuard<'a> {
            components: &'a mut Components,
            component_id: ComponentId,
            required_components: RequiredComponents,
        }

        impl Drop for DropGuard<'_> {
            fn drop(&mut self) {
                // SAFETY: The caller ensures that the `component_id` is valid.
                let required_components = unsafe {
                    self.components
                        .get_required_components_mut(self.component_id)
                        .debug_checked_unwrap()
                };

                debug_assert!(required_components.direct.is_empty());
                debug_assert!(required_components.all.is_empty());

                *required_components = core::mem::take(&mut self.required_components);
            }
        }

        let mut guard = DropGuard {
            component_id,
            // SAFETY: The caller ensures that the `component_id` is valid.
            required_components: core::mem::take(unsafe {
                self.get_required_components_mut(component_id)
                    .debug_checked_unwrap()
            }),
            components: self,
        };

        f(guard.components, &mut guard.required_components)
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

/// This is a safe handle around `ComponentsRegistrator` and `RequiredComponents` to register required components.
pub struct RequiredComponentsRegistrator<'a, 'w> {
    components: &'a mut ComponentsRegistrator<'w>,
    required_components: &'a mut RequiredComponents,
}

impl<'a, 'w> RequiredComponentsRegistrator<'a, 'w> {
    /// # Safety
    ///
    /// All components in `required_components` must have been registered in `components`
    pub(super) unsafe fn new(
        components: &'a mut ComponentsRegistrator<'w>,
        required_components: &'a mut RequiredComponents,
    ) -> Self {
        Self {
            components,
            required_components,
        }
    }

    /// Registers the [`Component`] `C` as an explicitly required component.
    ///
    /// If the component was not already registered as an explicit required component then it is added
    /// as one, potentially overriding the constructor of a inherited required component, otherwise panics.
    pub fn register_required<C: Component>(&mut self, constructor: fn() -> C) {
        // SAFETY: we internally guarantee that all components in `required_components`
        // are registered in `components`
        unsafe {
            self.required_components
                .register(self.components, constructor);
        }
    }

    /// Registers the [`Component`] with the given `component_id` ID as an explicitly required component.
    ///
    /// If the component was not already registered as an explicit required component then it is added
    /// as one, potentially overriding the constructor of a inherited required component, otherwise panics.
    ///
    /// # Safety
    ///
    /// `component_id` must be a valid [`ComponentId`] for `C` in the [`Components`] instance of `self`.
    pub unsafe fn register_required_by_id<C: Component>(
        &mut self,
        component_id: ComponentId,
        constructor: fn() -> C,
    ) {
        // SAFETY:
        // - the caller guarantees `component_id` is a valid component in `components` for `C`;
        // - we internally guarantee all other components in `required_components` are registered in `components`.
        unsafe {
            self.required_components.register_by_id::<C>(
                component_id,
                self.components,
                constructor,
            );
        }
    }

    /// Registers the [`Component`] with the given `component_id` ID as an explicitly required component.
    ///
    /// If the component was not already registered as an explicit required component then it is added
    /// as one, potentially overriding the constructor of a inherited required component, otherwise panics.
    ///
    /// # Safety
    ///
    /// - `component_id` must be valid in the [`Components`] instance of `self`;
    /// - `constructor` must return a [`RequiredComponentConstructor`] that constructs a valid instance for the
    ///   component with ID `component_id`.
    pub unsafe fn register_required_dynamic_with(
        &mut self,
        component_id: ComponentId,
        constructor: impl FnOnce() -> RequiredComponentConstructor,
    ) {
        // SAFETY:
        // - the caller guarantees `component_id` is valid in `components`;
        // - the caller guarantees `constructor` returns a valid constructor for `component_id`;
        // - we internally guarantee all other components in `required_components` are registered in `components`.
        unsafe {
            self.required_components.register_dynamic_with(
                component_id,
                self.components,
                constructor,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::string::{String, ToString};

    use crate::{
        bundle::Bundle,
        component::{Component, RequiredComponentsError},
        prelude::Resource,
        world::World,
    };

    #[test]
    fn required_components() {
        #[derive(Component)]
        #[require(Y)]
        struct X;

        #[derive(Component)]
        #[require(Z = new_z())]
        struct Y {
            value: String,
        }

        #[derive(Component)]
        struct Z(u32);

        impl Default for Y {
            fn default() -> Self {
                Self {
                    value: "hello".to_string(),
                }
            }
        }

        fn new_z() -> Z {
            Z(7)
        }

        let mut world = World::new();
        let id = world.spawn(X).id();
        assert_eq!(
            "hello",
            world.entity(id).get::<Y>().unwrap().value,
            "Y should have the default value"
        );
        assert_eq!(
            7,
            world.entity(id).get::<Z>().unwrap().0,
            "Z should have the value provided by the constructor defined in Y"
        );

        let id = world
            .spawn((
                X,
                Y {
                    value: "foo".to_string(),
                },
            ))
            .id();
        assert_eq!(
            "foo",
            world.entity(id).get::<Y>().unwrap().value,
            "Y should have the manually provided value"
        );
        assert_eq!(
            7,
            world.entity(id).get::<Z>().unwrap().0,
            "Z should have the value provided by the constructor defined in Y"
        );

        let id = world.spawn((X, Z(8))).id();
        assert_eq!(
            "hello",
            world.entity(id).get::<Y>().unwrap().value,
            "Y should have the default value"
        );
        assert_eq!(
            8,
            world.entity(id).get::<Z>().unwrap().0,
            "Z should have the manually provided value"
        );
    }

    #[test]
    fn generic_required_components() {
        #[derive(Component)]
        #[require(Y<usize>)]
        struct X;

        #[derive(Component, Default)]
        struct Y<T> {
            value: T,
        }

        let mut world = World::new();
        let id = world.spawn(X).id();
        assert_eq!(
            0,
            world.entity(id).get::<Y<usize>>().unwrap().value,
            "Y should have the default value"
        );
    }

    #[test]
    fn required_components_spawn_nonexistent_hooks() {
        #[derive(Component)]
        #[require(Y)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        #[derive(Resource)]
        struct A(usize);

        #[derive(Resource)]
        struct I(usize);

        let mut world = World::new();
        world.insert_resource(A(0));
        world.insert_resource(I(0));
        world
            .register_component_hooks::<Y>()
            .on_add(|mut world, _| world.resource_mut::<A>().0 += 1)
            .on_insert(|mut world, _| world.resource_mut::<I>().0 += 1);

        // Spawn entity and ensure Y was added
        assert!(world.spawn(X).contains::<Y>());

        assert_eq!(world.resource::<A>().0, 1);
        assert_eq!(world.resource::<I>().0, 1);
    }

    #[test]
    fn required_components_insert_existing_hooks() {
        #[derive(Component)]
        #[require(Y)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        #[derive(Resource)]
        struct A(usize);

        #[derive(Resource)]
        struct I(usize);

        let mut world = World::new();
        world.insert_resource(A(0));
        world.insert_resource(I(0));
        world
            .register_component_hooks::<Y>()
            .on_add(|mut world, _| world.resource_mut::<A>().0 += 1)
            .on_insert(|mut world, _| world.resource_mut::<I>().0 += 1);

        // Spawn entity and ensure Y was added
        assert!(world.spawn_empty().insert(X).contains::<Y>());

        assert_eq!(world.resource::<A>().0, 1);
        assert_eq!(world.resource::<I>().0, 1);
    }

    #[test]
    fn required_components_take_leaves_required() {
        #[derive(Component)]
        #[require(Y)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        let mut world = World::new();
        let e = world.spawn(X).id();
        let _ = world.entity_mut(e).take::<X>().unwrap();
        assert!(world.entity_mut(e).contains::<Y>());
    }

    #[test]
    fn required_components_retain_keeps_required() {
        #[derive(Component)]
        #[require(Y)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        #[derive(Component, Default)]
        struct Z;

        let mut world = World::new();
        let e = world.spawn((X, Z)).id();
        world.entity_mut(e).retain::<X>();
        assert!(world.entity_mut(e).contains::<X>());
        assert!(world.entity_mut(e).contains::<Y>());
        assert!(!world.entity_mut(e).contains::<Z>());
    }

    #[test]
    fn required_components_spawn_then_insert_no_overwrite() {
        #[derive(Component)]
        #[require(Y)]
        struct X;

        #[derive(Component, Default)]
        struct Y(usize);

        let mut world = World::new();
        let id = world.spawn((X, Y(10))).id();
        world.entity_mut(id).insert(X);

        assert_eq!(
            10,
            world.entity(id).get::<Y>().unwrap().0,
            "Y should still have the manually provided value"
        );
    }

    #[test]
    fn dynamic_required_components() {
        #[derive(Component)]
        #[require(Y)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        let mut world = World::new();
        let x_id = world.register_component::<X>();

        let mut e = world.spawn_empty();

        // SAFETY: x_id is a valid component id
        bevy_ptr::OwningPtr::make(X, |ptr| unsafe {
            e.insert_by_id(x_id, ptr);
        });

        assert!(e.contains::<Y>());
    }

    #[test]
    fn remove_component_and_its_runtime_required_components() {
        #[derive(Component)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        #[derive(Component, Default)]
        struct Z;

        #[derive(Component)]
        struct V;

        let mut world = World::new();
        world.register_required_components::<X, Y>();
        world.register_required_components::<Y, Z>();

        let e = world.spawn((X, V)).id();
        assert!(world.entity(e).contains::<X>());
        assert!(world.entity(e).contains::<Y>());
        assert!(world.entity(e).contains::<Z>());
        assert!(world.entity(e).contains::<V>());

        //check that `remove` works as expected
        world.entity_mut(e).remove::<X>();
        assert!(!world.entity(e).contains::<X>());
        assert!(world.entity(e).contains::<Y>());
        assert!(world.entity(e).contains::<Z>());
        assert!(world.entity(e).contains::<V>());

        world.entity_mut(e).insert(X);
        assert!(world.entity(e).contains::<X>());
        assert!(world.entity(e).contains::<Y>());
        assert!(world.entity(e).contains::<Z>());
        assert!(world.entity(e).contains::<V>());

        //remove `X` again and ensure that `Y` and `Z` was removed too
        world.entity_mut(e).remove_with_requires::<X>();
        assert!(!world.entity(e).contains::<X>());
        assert!(!world.entity(e).contains::<Y>());
        assert!(!world.entity(e).contains::<Z>());
        assert!(world.entity(e).contains::<V>());
    }

    #[test]
    fn remove_component_and_its_required_components() {
        #[derive(Component)]
        #[require(Y)]
        struct X;

        #[derive(Component, Default)]
        #[require(Z)]
        struct Y;

        #[derive(Component, Default)]
        struct Z;

        #[derive(Component)]
        struct V;

        let mut world = World::new();

        let e = world.spawn((X, V)).id();
        assert!(world.entity(e).contains::<X>());
        assert!(world.entity(e).contains::<Y>());
        assert!(world.entity(e).contains::<Z>());
        assert!(world.entity(e).contains::<V>());

        //check that `remove` works as expected
        world.entity_mut(e).remove::<X>();
        assert!(!world.entity(e).contains::<X>());
        assert!(world.entity(e).contains::<Y>());
        assert!(world.entity(e).contains::<Z>());
        assert!(world.entity(e).contains::<V>());

        world.entity_mut(e).insert(X);
        assert!(world.entity(e).contains::<X>());
        assert!(world.entity(e).contains::<Y>());
        assert!(world.entity(e).contains::<Z>());
        assert!(world.entity(e).contains::<V>());

        //remove `X` again and ensure that `Y` and `Z` was removed too
        world.entity_mut(e).remove_with_requires::<X>();
        assert!(!world.entity(e).contains::<X>());
        assert!(!world.entity(e).contains::<Y>());
        assert!(!world.entity(e).contains::<Z>());
        assert!(world.entity(e).contains::<V>());
    }

    #[test]
    fn remove_bundle_and_his_required_components() {
        #[derive(Component, Default)]
        #[require(Y)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        #[derive(Component, Default)]
        #[require(W)]
        struct Z;

        #[derive(Component, Default)]
        struct W;

        #[derive(Component)]
        struct V;

        #[derive(Bundle, Default)]
        struct TestBundle {
            x: X,
            z: Z,
        }

        let mut world = World::new();
        let e = world.spawn((TestBundle::default(), V)).id();

        assert!(world.entity(e).contains::<X>());
        assert!(world.entity(e).contains::<Y>());
        assert!(world.entity(e).contains::<Z>());
        assert!(world.entity(e).contains::<W>());
        assert!(world.entity(e).contains::<V>());

        world.entity_mut(e).remove_with_requires::<TestBundle>();
        assert!(!world.entity(e).contains::<X>());
        assert!(!world.entity(e).contains::<Y>());
        assert!(!world.entity(e).contains::<Z>());
        assert!(!world.entity(e).contains::<W>());
        assert!(world.entity(e).contains::<V>());
    }

    #[test]
    fn runtime_required_components() {
        // Same as `required_components` test but with runtime registration

        #[derive(Component)]
        struct X;

        #[derive(Component)]
        struct Y {
            value: String,
        }

        #[derive(Component)]
        struct Z(u32);

        impl Default for Y {
            fn default() -> Self {
                Self {
                    value: "hello".to_string(),
                }
            }
        }

        let mut world = World::new();

        world.register_required_components::<X, Y>();
        world.register_required_components_with::<Y, Z>(|| Z(7));

        let id = world.spawn(X).id();

        assert_eq!(
            "hello",
            world.entity(id).get::<Y>().unwrap().value,
            "Y should have the default value"
        );
        assert_eq!(
            7,
            world.entity(id).get::<Z>().unwrap().0,
            "Z should have the value provided by the constructor defined in Y"
        );

        let id = world
            .spawn((
                X,
                Y {
                    value: "foo".to_string(),
                },
            ))
            .id();
        assert_eq!(
            "foo",
            world.entity(id).get::<Y>().unwrap().value,
            "Y should have the manually provided value"
        );
        assert_eq!(
            7,
            world.entity(id).get::<Z>().unwrap().0,
            "Z should have the value provided by the constructor defined in Y"
        );

        let id = world.spawn((X, Z(8))).id();
        assert_eq!(
            "hello",
            world.entity(id).get::<Y>().unwrap().value,
            "Y should have the default value"
        );
        assert_eq!(
            8,
            world.entity(id).get::<Z>().unwrap().0,
            "Z should have the manually provided value"
        );
    }

    #[test]
    fn runtime_required_components_override_1() {
        #[derive(Component)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        #[derive(Component)]
        struct Z(u32);

        let mut world = World::new();

        // - X requires Y with default constructor
        // - Y requires Z with custom constructor
        // - X requires Z with custom constructor (more specific than X -> Y -> Z)
        world.register_required_components::<X, Y>();
        world.register_required_components_with::<Y, Z>(|| Z(5));
        world.register_required_components_with::<X, Z>(|| Z(7));

        let id = world.spawn(X).id();

        assert_eq!(
            7,
            world.entity(id).get::<Z>().unwrap().0,
            "Z should have the value provided by the constructor defined in X"
        );
    }

    #[test]
    fn runtime_required_components_override_2() {
        // Same as `runtime_required_components_override_1` test but with different registration order

        #[derive(Component)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        #[derive(Component)]
        struct Z(u32);

        let mut world = World::new();

        // - X requires Y with default constructor
        // - X requires Z with custom constructor (more specific than X -> Y -> Z)
        // - Y requires Z with custom constructor
        world.register_required_components::<X, Y>();
        world.register_required_components_with::<X, Z>(|| Z(7));
        world.register_required_components_with::<Y, Z>(|| Z(5));

        let id = world.spawn(X).id();

        assert_eq!(
            7,
            world.entity(id).get::<Z>().unwrap().0,
            "Z should have the value provided by the constructor defined in X"
        );
    }

    #[test]
    fn runtime_required_components_propagate_up() {
        // `A` requires `B` directly.
        #[derive(Component)]
        #[require(B)]
        struct A;

        #[derive(Component, Default)]
        struct B;

        #[derive(Component, Default)]
        struct C;

        let mut world = World::new();

        // `B` requires `C` with a runtime registration.
        // `A` should also require `C` because it requires `B`.
        world.register_required_components::<B, C>();

        let id = world.spawn(A).id();

        assert!(world.entity(id).get::<C>().is_some());
    }

    #[test]
    fn runtime_required_components_propagate_up_even_more() {
        #[derive(Component)]
        struct A;

        #[derive(Component, Default)]
        struct B;

        #[derive(Component, Default)]
        struct C;

        #[derive(Component, Default)]
        struct D;

        let mut world = World::new();

        world.register_required_components::<A, B>();
        world.register_required_components::<B, C>();
        world.register_required_components::<C, D>();

        let id = world.spawn(A).id();

        assert!(world.entity(id).get::<D>().is_some());
    }

    #[test]
    fn runtime_required_components_deep_require_does_not_override_shallow_require() {
        #[derive(Component)]
        struct A;
        #[derive(Component, Default)]
        struct B;
        #[derive(Component, Default)]
        struct C;
        #[derive(Component)]
        struct Counter(i32);
        #[derive(Component, Default)]
        struct D;

        let mut world = World::new();

        world.register_required_components::<A, B>();
        world.register_required_components::<B, C>();
        world.register_required_components::<C, D>();
        world.register_required_components_with::<D, Counter>(|| Counter(2));
        // This should replace the require constructor in A since it is
        // shallower.
        world.register_required_components_with::<C, Counter>(|| Counter(1));

        let id = world.spawn(A).id();

        // The "shallower" of the two components is used.
        assert_eq!(world.entity(id).get::<Counter>().unwrap().0, 1);
    }

    #[test]
    fn runtime_required_components_deep_require_does_not_override_shallow_require_deep_subtree_after_shallow(
    ) {
        #[derive(Component)]
        struct A;
        #[derive(Component, Default)]
        struct B;
        #[derive(Component, Default)]
        struct C;
        #[derive(Component, Default)]
        struct D;
        #[derive(Component, Default)]
        struct E;
        #[derive(Component)]
        struct Counter(i32);
        #[derive(Component, Default)]
        struct F;

        let mut world = World::new();

        world.register_required_components::<A, B>();
        world.register_required_components::<B, C>();
        world.register_required_components::<C, D>();
        world.register_required_components::<D, E>();
        world.register_required_components_with::<E, Counter>(|| Counter(1));
        world.register_required_components_with::<F, Counter>(|| Counter(2));
        world.register_required_components::<E, F>();

        let id = world.spawn(A).id();

        // The "shallower" of the two components is used.
        assert_eq!(world.entity(id).get::<Counter>().unwrap().0, 1);
    }

    #[test]
    fn runtime_required_components_existing_archetype() {
        #[derive(Component)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        let mut world = World::new();

        // Registering required components after the archetype has already been created should panic.
        // This may change in the future.
        world.spawn(X);
        assert!(matches!(
            world.try_register_required_components::<X, Y>(),
            Err(RequiredComponentsError::ArchetypeExists(_))
        ));
    }

    #[test]
    fn runtime_required_components_fail_with_duplicate() {
        #[derive(Component)]
        #[require(Y)]
        struct X;

        #[derive(Component, Default)]
        struct Y;

        let mut world = World::new();

        // This should fail: Tried to register Y as a requirement for X, but the requirement already exists.
        assert!(matches!(
            world.try_register_required_components::<X, Y>(),
            Err(RequiredComponentsError::DuplicateRegistration(_, _))
        ));
    }

    #[test]
    fn required_components_bundle_priority() {
        #[derive(Component, PartialEq, Eq, Clone, Copy, Debug)]
        struct MyRequired(bool);

        #[derive(Component, Default)]
        #[require(MyRequired(false))]
        struct MiddleMan;

        #[derive(Component, Default)]
        #[require(MiddleMan)]
        struct ConflictingRequire;

        #[derive(Component, Default)]
        #[require(MyRequired(true))]
        struct MyComponent;

        let mut world = World::new();
        let order_a = world
            .spawn((ConflictingRequire, MyComponent))
            .get::<MyRequired>()
            .cloned();
        let order_b = world
            .spawn((MyComponent, ConflictingRequire))
            .get::<MyRequired>()
            .cloned();

        assert_eq!(order_a, Some(MyRequired(false)));
        assert_eq!(order_b, Some(MyRequired(true)));
    }

    #[test]
    #[should_panic]
    fn required_components_recursion_errors() {
        #[derive(Component, Default)]
        #[require(B)]
        struct A;

        #[derive(Component, Default)]
        #[require(C)]
        struct B;

        #[derive(Component, Default)]
        #[require(B)]
        struct C;

        World::new().register_component::<A>();
    }

    #[test]
    #[should_panic]
    fn required_components_self_errors() {
        #[derive(Component, Default)]
        #[require(A)]
        struct A;

        World::new().register_component::<A>();
    }

    #[test]
    fn regression_19333() {
        #[derive(Component)]
        struct X(usize);

        #[derive(Default, Component)]
        #[require(X(0))]
        struct Base;

        #[derive(Default, Component)]
        #[require(X(1), Base)]
        struct A;

        #[derive(Default, Component)]
        #[require(A, Base)]
        struct B;

        #[derive(Default, Component)]
        #[require(B, Base)]
        struct C;

        let mut w = World::new();

        assert_eq!(w.spawn(B).get::<X>().unwrap().0, 1);
        assert_eq!(w.spawn(C).get::<X>().unwrap().0, 1);
    }

    #[test]
    fn required_components_depth_first_2v1() {
        #[derive(Component)]
        struct X(usize);

        #[derive(Component)]
        #[require(Left, Right)]
        struct Root;

        #[derive(Component, Default)]
        #[require(LeftLeft)]
        struct Left;

        #[derive(Component, Default)]
        #[require(X(0))] // This is at depth 2 but is more on the left of the tree
        struct LeftLeft;

        #[derive(Component, Default)]
        #[require(X(1))] //. This is at depth 1 but is more on the right of the tree
        struct Right;

        let mut world = World::new();

        // LeftLeft should have priority over Right
        assert_eq!(world.spawn(Root).get::<X>().unwrap().0, 0);
    }

    #[test]
    fn required_components_depth_first_3v1() {
        #[derive(Component)]
        struct X(usize);

        #[derive(Component)]
        #[require(Left, Right)]
        struct Root;

        #[derive(Component, Default)]
        #[require(LeftLeft)]
        struct Left;

        #[derive(Component, Default)]
        #[require(LeftLeftLeft)]
        struct LeftLeft;

        #[derive(Component, Default)]
        #[require(X(0))] // This is at depth 3 but is more on the left of the tree
        struct LeftLeftLeft;

        #[derive(Component, Default)]
        #[require(X(1))] //. This is at depth 1 but is more on the right of the tree
        struct Right;

        let mut world = World::new();

        // LeftLeftLeft should have priority over Right
        assert_eq!(world.spawn(Root).get::<X>().unwrap().0, 0);
    }

    #[test]
    fn runtime_required_components_depth_first_2v1() {
        #[derive(Component)]
        struct X(usize);

        #[derive(Component)]
        struct Root;

        #[derive(Component, Default)]
        struct Left;

        #[derive(Component, Default)]
        struct LeftLeft;

        #[derive(Component, Default)]
        struct Right;

        // Register bottom up: registering higher level components should pick up lower level ones.
        let mut world = World::new();
        world.register_required_components_with::<LeftLeft, X>(|| X(0));
        world.register_required_components_with::<Right, X>(|| X(1));
        world.register_required_components::<Left, LeftLeft>();
        world.register_required_components::<Root, Left>();
        world.register_required_components::<Root, Right>();
        assert_eq!(world.spawn(Root).get::<X>().unwrap().0, 0);

        // Register top down: registering lower components should propagate to higher ones
        let mut world = World::new();
        world.register_required_components::<Root, Left>(); // Note: still register Left before Right
        world.register_required_components::<Root, Right>();
        world.register_required_components::<Left, LeftLeft>();
        world.register_required_components_with::<Right, X>(|| X(1));
        world.register_required_components_with::<LeftLeft, X>(|| X(0));
        assert_eq!(world.spawn(Root).get::<X>().unwrap().0, 0);

        // Register top down again, but this time LeftLeft before Right
        let mut world = World::new();
        world.register_required_components::<Root, Left>();
        world.register_required_components::<Root, Right>();
        world.register_required_components::<Left, LeftLeft>();
        world.register_required_components_with::<LeftLeft, X>(|| X(0));
        world.register_required_components_with::<Right, X>(|| X(1));
        assert_eq!(world.spawn(Root).get::<X>().unwrap().0, 0);
    }

    #[test]
    fn runtime_required_components_propagate_metadata_alternate() {
        #[derive(Component, Default)]
        #[require(L1)]
        struct L0;

        #[derive(Component, Default)]
        struct L1;

        #[derive(Component, Default)]
        #[require(L3)]
        struct L2;

        #[derive(Component, Default)]
        struct L3;

        #[derive(Component, Default)]
        #[require(L5)]
        struct L4;

        #[derive(Component, Default)]
        struct L5;

        // Try to piece the 3 requirements together
        let mut world = World::new();
        world.register_required_components::<L1, L2>();
        world.register_required_components::<L3, L4>();
        let e = world.spawn(L0).id();
        assert!(world
            .query::<(&L0, &L1, &L2, &L3, &L4, &L5)>()
            .get(&world, e)
            .is_ok());

        // Repeat but in the opposite order
        let mut world = World::new();
        world.register_required_components::<L3, L4>();
        world.register_required_components::<L1, L2>();
        let e = world.spawn(L0).id();
        assert!(world
            .query::<(&L0, &L1, &L2, &L3, &L4, &L5)>()
            .get(&world, e)
            .is_ok());
    }

    #[test]
    fn runtime_required_components_propagate_metadata_chain() {
        #[derive(Component, Default)]
        #[require(L1)]
        struct L0;

        #[derive(Component, Default)]
        struct L1;

        #[derive(Component, Default)]
        struct L2;

        #[derive(Component, Default)]
        #[require(L4)]
        struct L3;

        #[derive(Component, Default)]
        struct L4;

        // Try to piece the 3 requirements together
        let mut world = World::new();
        world.register_required_components::<L1, L2>();
        world.register_required_components::<L2, L3>();
        let e = world.spawn(L0).id();
        assert!(world
            .query::<(&L0, &L1, &L2, &L3, &L4)>()
            .get(&world, e)
            .is_ok());

        // Repeat but in the opposite order
        let mut world = World::new();
        world.register_required_components::<L2, L3>();
        world.register_required_components::<L1, L2>();
        let e = world.spawn(L0).id();
        assert!(world
            .query::<(&L0, &L1, &L2, &L3, &L4)>()
            .get(&world, e)
            .is_ok());
    }

    #[test]
    fn runtime_required_components_cyclic() {
        #[derive(Component, Default)]
        #[require(B)]
        struct A;

        #[derive(Component, Default)]
        struct B;

        #[derive(Component, Default)]
        struct C;

        let mut world = World::new();

        assert!(world.try_register_required_components::<B, C>().is_ok());
        assert!(matches!(
            world.try_register_required_components::<C, A>(),
            Err(RequiredComponentsError::CyclicRequirement(_, _))
        ));
    }
}
