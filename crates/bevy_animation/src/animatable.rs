use bevy_math::Vec3;
use bevy_property::Property;
use bevy_render::color::Color;
use bevy_transform::components::Translation;
use splines::Interpolate;

pub enum AnimTracks {
    Primitive(Color),
    Enum(Vec<Color>),
    Struct(Vec<(&'static str, Color)>),
}

impl AnimTracks {
    pub fn len(&self) -> usize {
        match self {
            Self::Primitive(_) => 1,
            Self::Enum(v) => v.len(),
            Self::Struct(v) => v.len(),
        }
    }
}

pub trait Animatable {
    type Track: Interpolate<f32> + Property;
    fn anim_tracks() -> AnimTracks;
    fn set_values(&mut self, values: Vec<Self::Track>);
    fn values(&self) -> Vec<Self::Track>;
}

impl Animatable for f32 {
    type Track = f32;

    fn anim_tracks() -> AnimTracks {
        AnimTracks::Primitive(Color::WHITE)
    }

    fn set_values(&mut self, values: Vec<Self::Track>) {
        *self = *values.get(0).unwrap();
    }

    fn values(&self) -> Vec<Self::Track> {
        vec![*self]
    }
}

impl Animatable for Vec3 {
    type Track = f32;
    fn anim_tracks() -> AnimTracks {
        AnimTracks::Struct(vec![
            ("X", Color::BLUE),
            ("Y", Color::GREEN),
            ("Z", Color::RED),
        ])
    }

    fn set_values(&mut self, values: Vec<Self::Track>) {
        self.set_x(*values.get(0).unwrap());
        self.set_y(*values.get(1).unwrap());
        self.set_z(*values.get(2).unwrap());
    }

    fn values(&self) -> Vec<Self::Track> {
        vec![self.x(), self.y(), self.z()]
    }
}

impl Animatable for Translation {
    type Track = f32;
    fn anim_tracks() -> AnimTracks {
        AnimTracks::Struct(vec![
            ("X", Color::BLUE),
            ("Y", Color::GREEN),
            ("Z", Color::RED),
        ])
    }

    fn set_values(&mut self, values: Vec<Self::Track>) {
        self.set_x(*values.get(0).unwrap());
        self.set_y(*values.get(1).unwrap());
        self.set_z(*values.get(2).unwrap());
    }

    fn values(&self) -> Vec<Self::Track> {
        vec![self.x(), self.y(), self.z()]
    }
}
