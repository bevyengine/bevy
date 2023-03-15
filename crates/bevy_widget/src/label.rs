use bevy_a11y::accesskit::{NodeBuilder, Role};
use bevy_a11y::AccessibilityNode;
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::{Component, Entity};
use bevy_ecs::query::Changed;
use bevy_ecs::reflect::ReflectComponent;
use bevy_ecs::system::{Commands, Query};
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_text::Text;

/// Marker struct for labels
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct Label;

fn label_changed(
    mut commands: Commands,
    mut query: Query<(Entity, &Text, Option<&mut AccessibilityNode>), Changed<Label>>,
) {
    for (entity, text, accessible) in &mut query {
        let values = text
            .sections
            .iter()
            .map(|v| v.value.to_string())
            .collect::<Vec<String>>();
        let name = Some(values.join(" ").into_boxed_str());
        if let Some(mut accessible) = accessible {
            accessible.set_role(Role::LabelText);
            if let Some(name) = name {
                accessible.set_name(name);
            } else {
                accessible.clear_name();
            }
        } else {
            let mut node = NodeBuilder::new(Role::LabelText);
            if let Some(name) = name {
                node.set_name(name);
            }
            commands
                .entity(entity)
                .insert(AccessibilityNode::from(node));
        }
    }
}

/// A plugin for button widgets
#[derive(Default)]
pub struct LabelPlugin;

impl Plugin for LabelPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Label>().add_system(label_changed);
    }
}
