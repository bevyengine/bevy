use crate::{
    experimental::UiChildren,
    prelude::{Button, Label},
    ui_transform::UiGlobalTransform,
    widget::{ImageNode, TextUiReader},
    ComputedNode, UiSystems,
};
use bevy_a11y::{AccessibilityNode, AccessibilitySystems};
use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    hierarchy::ChildOf,
    lifecycle::HookContext,
    prelude::Entity,
    query::{Changed, With, Without},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query},
    world::{DeferredWorld, Ref},
};
use bevy_math::Affine2;
use bevy_reflect::prelude::ReflectDefault;

use accesskit::{Affine, Node, Rect, Role};
use bevy_reflect::Reflect;

fn calc_label(
    text_reader: &mut TextUiReader,
    children: impl Iterator<Item = Entity>,
) -> Option<Box<str>> {
    let mut name = None;
    for child in children {
        let values = text_reader
            .iter(child)
            .map(|(_, _, text, _, _, _, _)| text.into())
            .collect::<Vec<String>>();
        if !values.is_empty() {
            name = Some(values.join(" "));
        }
    }
    name.map(String::into_boxed_str)
}

fn sync_bounds_and_transforms(
    mut accessible_nodes_query: Query<(
        &mut AccessibilityNode,
        Ref<ComputedNode>,
        Ref<UiGlobalTransform>,
        Option<&ChildOf>,
    )>,
    accessible_transform_query: Query<Ref<UiGlobalTransform>, With<AccessibilityNode>>,
) {
    for (mut accessible, node, ui_transform, maybe_child_of) in &mut accessible_nodes_query {
        let maybe_parent_transform = maybe_child_of
            .and_then(|child_of| accessible_transform_query.get(child_of.parent()).ok());

        if !(node.is_changed()
            || ui_transform.is_changed()
            || maybe_parent_transform.is_some_and(|transform| transform.is_changed()))
        {
            continue;
        }

        accessible.set_bounds(Rect::new(
            -0.5 * node.size.x as f64,
            -0.5 * node.size.y as f64,
            0.5 * node.size.x as f64,
            0.5 * node.size.y as f64,
        ));

        // If the node has an accessible parent, its transform in the accessibility tree must be relative to the parent.
        let transform = maybe_parent_transform
            .and_then(|transform| transform.try_inverse())
            .unwrap_or_default()
            * ui_transform.affine();

        if transform.is_finite() && transform != Affine2::IDENTITY {
            accessible.set_transform(Affine::new(transform.to_cols_array().map(f64::from)));
        } else {
            accessible.clear_transform();
        }
    }
}

fn button_changed(
    mut commands: Commands,
    mut query: Query<(Entity, Option<&mut AccessibilityNode>), Changed<Button>>,
    ui_children: UiChildren,
    mut text_reader: TextUiReader,
) {
    for (entity, accessible) in &mut query {
        let label = calc_label(&mut text_reader, ui_children.iter_ui_children(entity));
        if let Some(mut accessible) = accessible {
            accessible.set_role(Role::Button);
            if let Some(name) = label {
                accessible.set_label(name);
            } else {
                accessible.clear_label();
            }
        } else {
            let mut node = Node::new(Role::Button);
            if let Some(label) = label {
                node.set_label(label);
            }
            commands
                .entity(entity)
                .try_insert(AccessibilityNode::from(node));
        }
    }
}

fn image_changed(
    mut commands: Commands,
    mut query: Query<
        (Entity, Option<&mut AccessibilityNode>),
        (Changed<ImageNode>, Without<Button>),
    >,
    ui_children: UiChildren,
    mut text_reader: TextUiReader,
) {
    for (entity, accessible) in &mut query {
        let label = calc_label(&mut text_reader, ui_children.iter_ui_children(entity));
        if let Some(mut accessible) = accessible {
            accessible.set_role(Role::Image);
            if let Some(label) = label {
                accessible.set_label(label);
            } else {
                accessible.clear_label();
            }
        } else {
            let mut node = Node::new(Role::Image);
            if let Some(label) = label {
                node.set_label(label);
            }
            commands
                .entity(entity)
                .try_insert(AccessibilityNode::from(node));
        }
    }
}

fn label_changed(
    mut commands: Commands,
    mut query: Query<(Entity, Option<&mut AccessibilityNode>), Changed<Label>>,
    mut text_reader: TextUiReader,
) {
    for (entity, accessible) in &mut query {
        let values = text_reader
            .iter(entity)
            .map(|(_, _, text, _, _, _, _)| text.into())
            .collect::<Vec<String>>();
        let label = Some(values.join(" ").into_boxed_str());
        if let Some(mut accessible) = accessible {
            accessible.set_role(Role::Label);
            if let Some(label) = label {
                accessible.set_value(label);
            } else {
                accessible.clear_value();
            }
        } else {
            let mut node = Node::new(Role::Label);
            if let Some(label) = label {
                node.set_value(label);
            }
            commands
                .entity(entity)
                .try_insert(AccessibilityNode::from(node));
        }
    }
}

/// A component which permits the a11y label to be specified independently from other a11y
/// attributes.
///
/// The content of the `label` attribute is typically application-specific, and frequently
/// originates in application code rather than library code. Because the primary mechanism of entity
/// composition in Bevy is component insertion (especially in BSN scenes), and because ``accesskit``
/// mandates that all a11y properties be stored in a single data structure, it can be cumbersome
/// to combine together a11y properties coming from different parts of the code; making the label
/// its own component makes it possible to specify the label as a mixin.
///
/// Internally, what this does is update the [`AccessibilityNode`] component, using component hooks
/// which are automatically registered when this component is used.
#[derive(Component, Debug, Default, Clone, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
#[require(AccessibilityNode)]
#[component(immutable, on_insert = on_label_inserted, on_remove = on_label_removed)]
pub struct AccessibleLabel(pub String);

impl AccessibleLabel {
    /// Makes a new [`AccessibleLabel`] component.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

fn on_label_inserted(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    if let Some(label) = world.get::<AccessibleLabel>(entity) {
        let label_text = label.0.clone().into_boxed_str();
        if let Some(mut accessible) = world.get_mut::<AccessibilityNode>(entity) {
            accessible.set_label(label_text);
        }
    }
}

fn on_label_removed(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    if let Some(mut accessible) = world.get_mut::<AccessibilityNode>(entity) {
        accessible.clear_label();
    }
}

/// `AccessKit` integration for `bevy_ui`.
pub(crate) struct AccessibilityPlugin;

impl Plugin for AccessibilityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                button_changed,
                image_changed,
                label_changed,
                sync_bounds_and_transforms
                    .after(button_changed)
                    .after(image_changed)
                    .after(label_changed),
            )
                .in_set(UiSystems::PostLayout)
                .before(AccessibilitySystems::Update),
        );
    }
}
