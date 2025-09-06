use bevy_ecs::{component::Component, entity::Entity, event::EntityEvent};

/// Represents one or more text inputs in a form being submitted
#[derive(EntityEvent, Clone, Debug, Component)]
pub struct TextSubmission {
    /// content from the textbox
    pub text: String,
    /// entity
    pub entity: Entity,
}
