use std::marker::PhantomData;

#[cfg(feature = "bevy_app")]
use crate::Parent;
use bevy_ecs::prelude::*;
#[cfg(feature = "bevy_app")]
use bevy_utils::{get_short_name, HashSet};

/// When enabled, runs [`check_hierarchy_component_has_valid_parent<T>`].
///
/// This resource is added by [`ValidParentCheckPlugin<T>`].
/// It is enabled on debug builds and disabled in release builds by default,
/// you can update this resource at runtime to change the default behavior.
#[derive(Resource)]
pub struct ReportHierarchyIssue<T> {
    /// Whether to run [`check_hierarchy_component_has_valid_parent<T>`].
    pub enabled: bool,
    _comp: PhantomData<fn(T)>,
}

impl<T> ReportHierarchyIssue<T> {
    /// Constructs a new object
    pub fn new(enabled: bool) -> Self {
        ReportHierarchyIssue {
            enabled,
            _comp: Default::default(),
        }
    }
}

impl<T> PartialEq for ReportHierarchyIssue<T> {
    fn eq(&self, other: &Self) -> bool {
        self.enabled == other.enabled
    }
}

impl<T> Default for ReportHierarchyIssue<T> {
    fn default() -> Self {
        Self {
            enabled: cfg!(debug_assertions),
            _comp: PhantomData,
        }
    }
}

#[cfg(feature = "bevy_app")]
/// System to print a warning for each [`Entity`] with a `T` component
/// which parent hasn't a `T` component.
///
/// Hierarchy propagations are top-down, and limited only to entities
/// with a specific component (such as `InheritedVisibility` and `GlobalTransform`).
/// This means that entities with one of those component
/// and a parent without the same component is probably a programming error.
/// (See B0004 explanation linked in warning message)
pub fn check_hierarchy_component_has_valid_parent<T: Component>(
    parent_query: Query<
        (Entity, &Parent, Option<&bevy_core::Name>),
        (With<T>, Or<(Changed<Parent>, Added<T>)>),
    >,
    component_query: Query<(), With<T>>,
    mut already_diagnosed: Local<HashSet<Entity>>,
) {
    for (entity, parent, name) in &parent_query {
        let parent = parent.get();
        if !component_query.contains(parent) && !already_diagnosed.contains(&entity) {
            already_diagnosed.insert(entity);
            bevy_utils::tracing::warn!(
                "warning[B0004]: {name} with the {ty_name} component has a parent without {ty_name}.\n\
                This will cause inconsistent behaviors! See: https://bevyengine.org/learn/errors/#b0004",
                ty_name = get_short_name(std::any::type_name::<T>()),
                name = name.map_or_else(|| format!("Entity {}", entity), |s| format!("The {s} entity")),
            );
        }
    }
}

/// Run criteria that only allows running when [`ReportHierarchyIssue<T>`] is enabled.
pub fn on_hierarchy_reports_enabled<T>(report: Res<ReportHierarchyIssue<T>>) -> bool
where
    T: Component,
{
    report.enabled
}

/// Print a warning for each `Entity` with a `T` component
/// whose parent doesn't have a `T` component.
///
/// See [`check_hierarchy_component_has_valid_parent`] for details.
pub struct ValidParentCheckPlugin<T: Component>(PhantomData<fn() -> T>);
impl<T: Component> Default for ValidParentCheckPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

#[cfg(feature = "bevy_app")]
impl<T: Component> bevy_app::Plugin for ValidParentCheckPlugin<T> {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<ReportHierarchyIssue<T>>().add_systems(
            bevy_app::Last,
            check_hierarchy_component_has_valid_parent::<T>
                .run_if(resource_equals(ReportHierarchyIssue::<T>::new(true))),
        );
    }
}
