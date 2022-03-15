use crate::{Children, HierarchySystem, Parent};
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

/// Defines a component that inherits state from it's ancestors.
/// These types are typically publicly read-only, relying on a "source"
/// companion component that is used to compute the new state of a by
/// composing the source and it's parent.
pub trait Heritable: Component {
    /// The source component where the base state is sourced from.
    type Source: Component;
    /// Updates the base state of a root-level component based on the companion
    /// source component.
    fn root(&mut self, source: &Self::Source);
    /// Updates the a mid-level or leaf component in a hierarchy based on the
    /// companion source component and the immediate parent of the entity.
    fn inherit(&mut self, parent: &Self, source: &Self::Source);
}

/// Extension trate for adding inheritance based systems to an [`App`].
pub trait HeritableAppExt {
    /// Registers systems for propagating the inherited states.
    /// See [`Heritable`] for more information about hierarchical inheritance.
    fn register_heritable<T: Heritable>(&mut self) -> &mut Self;
}

impl HeritableAppExt for App {
    fn register_heritable<T: Heritable>(&mut self) -> &mut Self {
        // Adding these to startup ensures the first update is "correct"
        self.add_startup_system_to_stage(
            StartupStage::PostStartup,
            inheritance_system::<T>
                .label(HierarchySystem::InheritancePropagation)
                .after(HierarchySystem::ParentUpdate),
        )
        .add_startup_system_to_stage(
            StartupStage::PostStartup,
            inheritance_system_flat::<T>
                .label(HierarchySystem::InheritancePropagation)
                .after(HierarchySystem::ParentUpdate),
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            inheritance_system::<T>
                .label(HierarchySystem::InheritancePropagation)
                .after(HierarchySystem::ParentUpdate),
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            inheritance_system_flat::<T>
                .label(HierarchySystem::InheritancePropagation)
                .after(HierarchySystem::ParentUpdate),
        )
    }
}

struct Pending<T> {
    parent: *const T,
    child: Entity,
    changed: bool,
}

// SAFE: Pending a private type that is completely flushed at the each inheritance system
// run. Never accessed from multiple threads.
unsafe impl<T> Send for Pending<T> {}
// SAFE: Pending a private type that is completely flushed at the each inheritance system
// run. Never accessed from multiple threads.
unsafe impl<T> Sync for Pending<T> {}

fn inheritance_system_flat<T: Heritable>(
    mut root_query: Query<
        (&T::Source, &mut T),
        (Without<Parent>, Without<Children>, Changed<T::Source>),
    >,
) {
    for (source, mut component) in root_query.iter_mut() {
        component.root(source);
    }
}

fn inheritance_system<T: Heritable>(
    mut root_query: Query<(&T::Source, &mut T, Changed<T::Source>, &Children), Without<Parent>>,
    mut query: Query<(&T::Source, Changed<T::Source>, &mut T, Option<&Children>), With<Parent>>,
    mut pending: Local<Vec<Pending<T>>>,
) {
    for (source, mut component, changed, children) in root_query.iter_mut() {
        if changed {
            component.root(source);
        }

        pending.extend(children.0.iter().map(|child| Pending {
            parent: &*component as *const T,
            changed,
            child: *child,
        }));

        while let Some(current) = pending.pop() {
            if let Ok((source, mut changed, mut component, children)) = query.get_mut(current.child)
            {
                changed |= current.changed;
                if changed {
                    // SAFE: The pointers used here are all created during the current traversal
                    // of the hierarchy and are cannot be moved during the middle of it.
                    unsafe {
                        component.inherit(&current.parent.read(), source);
                    }
                }

                if let Some(children) = children {
                    pending.extend(children.0.iter().map(|child| Pending {
                        parent: &*component as *const T,
                        changed,
                        child: *child,
                    }));
                }
            }
        }
    }
    debug_assert!(pending.is_empty());
}
