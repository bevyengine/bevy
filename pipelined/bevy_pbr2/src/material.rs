use bevy_reflect::{Reflect, TypeUuid};
use bevy_render2::color::Color;

#[derive(Debug, Default, Clone, TypeUuid, Reflect)]
#[uuid = "7494888b-c082-457b-aacf-517228cc0c22"]
pub struct StandardMaterial {
    pub color: Color,
}

impl From<Color> for StandardMaterial {
    fn from(color: Color) -> Self {
        StandardMaterial {
            color,
            ..Default::default()
        }
    }
}
