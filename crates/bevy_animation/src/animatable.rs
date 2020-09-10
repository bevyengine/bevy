use bevy_render::color::Color;
use bevy_transform::components::Translation;
use splines::Spline;

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

pub trait Splines: Default {
    fn vec(&self) -> Vec<&Spline<f32, f32>>;
}

pub struct SplinesVec3 {
    pub x: Spline<f32, f32>,
    pub y: Spline<f32, f32>,
    pub z: Spline<f32, f32>,
}

impl Default for SplinesVec3 {
    fn default() -> Self {
        Self {
            x: Spline::from_vec(vec![]),
            y: Spline::from_vec(vec![]),
            z: Spline::from_vec(vec![]),
        }
    }
}

impl Splines for SplinesVec3 {
    fn vec(&self) -> Vec<&Spline<f32, f32>> {
        vec![&self.x, &self.y, &self.z]
    }
}

pub struct SplinesOne(pub Spline<f32, f32>);

impl Default for SplinesOne {
    fn default() -> Self {
        Self(Spline::from_vec(vec![]))
    }
}

impl Splines for SplinesOne {
    fn vec(&self) -> Vec<&Spline<f32, f32>> {
        vec![&self.0]
    }
}

pub trait Animatable {
    type Splines: Splines;
    fn anim_tracks() -> AnimTracks;
    fn set_values(&mut self, values: Vec<f32>);
    fn values(&self) -> Vec<f32>;
}

impl Animatable for Translation {
    type Splines = SplinesVec3;
    fn anim_tracks() -> AnimTracks {
        AnimTracks::Struct(vec![
            ("X", Color::BLUE),
            ("Y", Color::GREEN),
            ("Z", Color::RED),
        ])
    }

    fn set_values(&mut self, values: Vec<f32>) {
        self.set_x(*values.get(0).unwrap());
        self.set_y(*values.get(1).unwrap());
        self.set_z(*values.get(2).unwrap());
    }

    fn values(&self) -> Vec<f32> {
        vec![self.x(), self.y(), self.z()]
    }
}
