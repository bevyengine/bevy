use crate::{Children, HierarchySystem, Parent};
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

/// Defines a component that inherits state from it's ancestors.
/// These types are typically publicly read-only, relying on a "source"
/// companion component that is used to compute the new state of a by
/// composing the source and it's parent.
pub trait Heritable: Component + Copy {
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
        .add_system_to_stage(
            CoreStage::PostUpdate,
            inheritance_system::<T>
                .label(HierarchySystem::InheritancePropagation)
                .after(HierarchySystem::ParentUpdate),
        )
    }
}

/// Update children in a hierarchy based on the properties of their parents.
pub fn inheritance_system<T: Heritable>(
    mut root_query: Query<
        (Option<&Children>, &T::Source, Changed<T::Source>, &mut T),
        Without<Parent>,
    >,
    mut source_query: Query<(&T::Source, Changed<T::Source>, &mut T), With<Parent>>,
    children_query: Query<Option<&Children>, (With<Parent>, With<T>)>,
) {
    for (children, source, source_changed, mut root_component) in root_query.iter_mut() {
        if source_changed {
            root_component.root(source);
        }

        if let Some(children) = children {
            for child in children.iter() {
                propagate_recursive(
                    &*root_component,
                    &mut source_query,
                    &children_query,
                    *child,
                    source_changed,
                );
            }
        }
    }
}

fn propagate_recursive<T: Heritable>(
    parent: &T,
    source_query: &mut Query<(&T::Source, Changed<T::Source>, &mut T), With<Parent>>,
    children_query: &Query<Option<&Children>, (With<Parent>, With<T>)>,
    entity: Entity,
    mut changed: bool,
) {
    let component = {
        if let Ok((source, source_changed, mut component)) = source_query.get_mut(entity) {
            changed |= source_changed;
            if changed {
                component.inherit(parent, source);
            }
            *component
        } else {
            return;
        }
    };

    if let Ok(Some(children)) = children_query.get(entity) {
        for child in children.iter() {
            propagate_recursive(&component, source_query, children_query, *child, changed);
        }
    }
}
