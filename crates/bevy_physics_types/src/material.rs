use bevy_ecs::prelude::Component;

/// USD `physics:staticFriction` doc: "Static friction coefficient. Unitless."
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct StaticFriction(pub f32);

impl Default for StaticFriction {
    fn default() -> Self {
        Self(0.6)
    }
}

/// USD `physics:dynamicFriction` doc: "Dynamic friction coefficient. Unitless."
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct DynamicFriction(pub f32);

impl Default for DynamicFriction {
    fn default() -> Self {
        Self(0.4)
    }
}

/// USD `physics:restitution` doc: "Restitution coefficient. Unitless."
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Restitution(pub f32);

impl Default for Restitution {
    fn default() -> Self {
        Self(0.2)
    }
}
