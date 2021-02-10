use super::SurfaceId;
use bevy_app::prelude::EventReader;
use bevy_ecs::{Query, Res};
use bevy_math::{clamp, vec2, Rect, Vec2};
use bevy_reflect::{Reflect, ReflectComponent};
use bevy_utils::HashMap;
use bevy_window::{WindowId, WindowResized, WindowScaleFactorChanged, Windows};
use std::ops::{Add, AddAssign, Sub, SubAssign};

#[derive(Debug, PartialEq, Clone, Reflect)]
#[reflect(Component)]
pub struct Viewport {
    #[reflect(ignore)]
    pub surface: SurfaceId,
    pub sides: Rect<ViewportSideLocation>,
    pub scale_factor: f64,
    // computed values
    pub origin: Vec2,
    pub size: Vec2,
}

impl Viewport {
    pub fn physical_origin(&self) -> Vec2 {
        (self.origin.as_f64() * self.scale_factor).as_f32()
    }

    pub fn physical_size(&self) -> Vec2 {
        (self.size.as_f64() * self.scale_factor).as_f32()
    }

    pub fn update_rectangle(&mut self, surface_size: Vec2) {
        self.origin.x = match self.sides.left {
            ViewportSideLocation::Absolute(value) => value,
            ViewportSideLocation::Relative(value) => value * surface_size.x,
        };
        self.origin.y = match self.sides.top {
            ViewportSideLocation::Absolute(value) => value,
            ViewportSideLocation::Relative(value) => value * surface_size.y,
        };
        self.size.x = match self.sides.right {
            ViewportSideLocation::Absolute(value) => value - self.origin.x,
            ViewportSideLocation::Relative(value) => value * surface_size.x - self.origin.x,
        };
        self.size.y = match self.sides.bottom {
            ViewportSideLocation::Absolute(value) => value - self.origin.y,
            ViewportSideLocation::Relative(value) => value * surface_size.y - self.origin.y,
        };
        self.origin = clamp(self.origin, Vec2::zero(), surface_size);
        self.size = clamp(self.size, Vec2::one(), surface_size - self.origin);
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            surface: WindowId::primary().into(),
            sides: Rect {
                left: ViewportSideLocation::Relative(0.0),
                right: ViewportSideLocation::Relative(1.0),
                top: ViewportSideLocation::Relative(0.0),
                bottom: ViewportSideLocation::Relative(1.0),
            },
            scale_factor: 1.0,
            origin: Vec2::zero(),
            size: Vec2::one(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Reflect)]
#[reflect_value(PartialEq)]
pub enum ViewportSideLocation {
    Relative(f32),
    Absolute(f32),
}

impl Default for ViewportSideLocation {
    fn default() -> Self {
        Self::Relative(0.0)
    }
}

impl Add<f32> for ViewportSideLocation {
    type Output = ViewportSideLocation;

    fn add(self, rhs: f32) -> Self::Output {
        match self {
            ViewportSideLocation::Relative(value) => ViewportSideLocation::Relative(value + rhs),
            ViewportSideLocation::Absolute(value) => ViewportSideLocation::Absolute(value + rhs),
        }
    }
}

impl Sub<f32> for ViewportSideLocation {
    type Output = ViewportSideLocation;

    fn sub(self, rhs: f32) -> Self::Output {
        match self {
            ViewportSideLocation::Relative(value) => ViewportSideLocation::Relative(value - rhs),
            ViewportSideLocation::Absolute(value) => ViewportSideLocation::Absolute(value - rhs),
        }
    }
}

impl AddAssign<f32> for ViewportSideLocation {
    fn add_assign(&mut self, rhs: f32) {
        match self {
            ViewportSideLocation::Relative(value) => *value += rhs,
            ViewportSideLocation::Absolute(value) => *value += rhs,
        }
    }
}

impl SubAssign<f32> for ViewportSideLocation {
    fn sub_assign(&mut self, rhs: f32) {
        match self {
            ViewportSideLocation::Relative(value) => *value -= rhs,
            ViewportSideLocation::Absolute(value) => *value -= rhs,
        }
    }
}

pub fn viewport_system(
    mut window_resized_events: EventReader<WindowResized>,
    mut window_scale_change_events: EventReader<WindowScaleFactorChanged>,
    windows: Res<Windows>,
    mut query: Query<&mut Viewport>,
) {
    // by using a HashMap we can use insert()
    let mut changed_window_ids: HashMap<WindowId, ()> = HashMap::default();
    for event in window_resized_events.iter() {
        changed_window_ids.insert(event.id, ());
    }
    for event in window_scale_change_events.iter() {
        changed_window_ids.insert(event.id, ());
    }
    // update the window surfaces
    for (id, _) in changed_window_ids.iter() {
        if let Some(window) = windows.get(*id) {
            for mut viewport in query.iter_mut() {
                if viewport.surface.get_window() == Some(*id) {
                    viewport.update_rectangle(vec2(window.width(), window.height()));
                    viewport.scale_factor = window.scale_factor();
                }
            }
        }
    }
}
