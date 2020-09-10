use bevy_math::Vec3;
use bevy_render::color::Color;
use bevy_transform::components::Translation;
use splines::{Interpolate, Spline};

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

pub trait Splines<T: Interpolate<f32>>: Default {
    fn vec(&self) -> Vec<&Spline<f32, T>>;
}

pub struct SplinesOne<T: Interpolate<f32>>(pub Spline<f32, T>);

impl<T: Interpolate<f32>> Default for SplinesOne<T> {
    fn default() -> Self {
        Self(Spline::from_vec(vec![]))
    }
}

impl<T: Interpolate<f32>> Splines<T> for SplinesOne<T> {
    fn vec(&self) -> Vec<&Spline<f32, T>> {
        vec![&self.0]
    }
}

pub struct SplinesVec3<T> {
    pub x: Spline<f32, T>,
    pub y: Spline<f32, T>,
    pub z: Spline<f32, T>,
}

impl<T: Interpolate<f32>> Default for SplinesVec3<T> {
    fn default() -> Self {
        Self {
            x: Spline::from_vec(vec![]),
            y: Spline::from_vec(vec![]),
            z: Spline::from_vec(vec![]),
        }
    }
}

impl<T: Interpolate<f32>> Splines<T> for SplinesVec3<T> {
    fn vec(&self) -> Vec<&Spline<f32, T>> {
        vec![&self.x, &self.y, &self.z]
    }
}

pub trait Animatable {
    type Track: Interpolate<f32>;
    type Splines: Splines<Self::Track>;
    fn anim_tracks() -> AnimTracks;
    fn set_values(&mut self, values: Vec<Self::Track>);
    fn values(&self) -> Vec<Self::Track>;
}

impl Animatable for f32 {
    type Track = f32;
    type Splines = SplinesOne<f32>;

    fn anim_tracks() -> AnimTracks {
        AnimTracks::Primitive(Color::WHITE)
    }

    fn set_values(&mut self, values: Vec<Self::Track>) {
        *self = *values.get(0).unwrap()
    }

    fn values(&self) -> Vec<Self::Track> {
        vec![*self]
    }
}

impl Animatable for Vec3 {
    type Track = f32;
    type Splines = SplinesVec3<Self::Track>;
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
    type Splines = SplinesVec3<Self::Track>;
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
