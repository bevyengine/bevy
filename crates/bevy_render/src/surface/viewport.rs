use super::SurfaceId;
use bevy_app::prelude::EventReader;
use bevy_ecs::{Changed, Query, QuerySet, Res};
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
    pub sides: Rect<SideLocation>,
    pub scale_factor: f64,
    // computed values
    origin: Vec2,
    size: Vec2,
}

impl Viewport {
    const MIN_SIZE: f32 = 1.0;

    pub fn new(descriptor: ViewportDescriptor) -> Self {
        Self {
            surface: descriptor.surface,
            sides: descriptor.sides,
            scale_factor: descriptor.scale_factor,
            origin: vec2(0.0, 0.0),
            size: vec2(Self::MIN_SIZE, Self::MIN_SIZE),
        }
    }

    pub fn origin(&self) -> Vec2 {
        self.origin
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn physical_origin(&self) -> Vec2 {
        (self.origin.as_f64() * self.scale_factor).as_f32()
    }

    pub fn physical_size(&self) -> Vec2 {
        (self.size.as_f64() * self.scale_factor).as_f32()
    }

    pub fn update_rectangle(&mut self, surface_size: Vec2) {
        let x = match self.sides.left {
            SideLocation::Absolute(value) => value,
            SideLocation::Relative(value) => value * surface_size.x,
        };
        let y = match self.sides.top {
            SideLocation::Absolute(value) => value,
            SideLocation::Relative(value) => value * surface_size.y,
        };
        let w = match self.sides.right {
            SideLocation::Absolute(value) => value - x,
            SideLocation::Relative(value) => value * surface_size.x - x,
        };
        let h = match self.sides.bottom {
            SideLocation::Absolute(value) => value - y,
            SideLocation::Relative(value) => value * surface_size.y - y,
        };
        self.origin.x = clamp(x, Self::MIN_SIZE, surface_size.x - Self::MIN_SIZE);
        self.origin.y = clamp(y, Self::MIN_SIZE, surface_size.y - Self::MIN_SIZE);
        self.size.x = clamp(w, Self::MIN_SIZE, surface_size.x - self.origin.x);
        self.size.y = clamp(h, Self::MIN_SIZE, surface_size.y - self.origin.y);
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Viewport::new(ViewportDescriptor::default())
    }
}

#[derive(Debug, Clone)]
pub struct ViewportDescriptor {
    pub surface: SurfaceId,
    pub sides: Rect<SideLocation>,
    pub scale_factor: f64,
}

impl Default for ViewportDescriptor {
    fn default() -> Self {
        Self {
            surface: WindowId::primary().into(),
            sides: Rect {
                left: SideLocation::Relative(0.0),
                right: SideLocation::Relative(1.0),
                top: SideLocation::Relative(0.0),
                bottom: SideLocation::Relative(1.0),
            },
            scale_factor: 1.0,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Reflect)]
#[reflect_value(PartialEq)]
pub enum SideLocation {
    Relative(f32),
    Absolute(f32),
}

impl Default for SideLocation {
    fn default() -> Self {
        Self::Relative(0.0)
    }
}

impl Add<f32> for SideLocation {
    type Output = SideLocation;

    fn add(self, rhs: f32) -> Self::Output {
        match self {
            SideLocation::Relative(value) => SideLocation::Relative(value + rhs),
            SideLocation::Absolute(value) => SideLocation::Absolute(value + rhs),
        }
    }
}

impl Sub<f32> for SideLocation {
    type Output = SideLocation;

    fn sub(self, rhs: f32) -> Self::Output {
        match self {
            SideLocation::Relative(value) => SideLocation::Relative(value - rhs),
            SideLocation::Absolute(value) => SideLocation::Absolute(value - rhs),
        }
    }
}

impl AddAssign<f32> for SideLocation {
    fn add_assign(&mut self, rhs: f32) {
        match self {
            SideLocation::Relative(value) => *value += rhs,
            SideLocation::Absolute(value) => *value += rhs,
        }
    }
}

impl SubAssign<f32> for SideLocation {
    fn sub_assign(&mut self, rhs: f32) {
        match self {
            SideLocation::Relative(value) => *value -= rhs,
            SideLocation::Absolute(value) => *value -= rhs,
        }
    }
}

pub fn viewport_system(
    mut window_resized_events: EventReader<WindowResized>,
    mut window_scale_change_events: EventReader<WindowScaleFactorChanged>,
    windows: Res<Windows>,
    mut queries: QuerySet<(Query<&Viewport, Changed<Viewport>>, Query<&mut Viewport>)>,
) {
    // by using a HashMap we can use insert()
    let mut changed_window_ids: HashMap<WindowId, ()> = HashMap::default();
    for event in window_resized_events.iter() {
        changed_window_ids.insert(event.id, ());
    }
    for event in window_scale_change_events.iter() {
        changed_window_ids.insert(event.id, ());
    }
    for viewport in queries.q0().iter() {
        if let Some(id) = viewport.surface.get_window() {
            changed_window_ids.insert(id, ());
        }
    }

    // update the surfaces
    for mut viewport in queries.q1_mut().iter_mut() {
        match viewport.surface {
            SurfaceId::Window(id) => {
                if changed_window_ids.contains_key(&id) {
                    let window = windows
                        .get(id)
                        .expect("Viewport surface refers to non-existent window");
                    viewport.update_rectangle(vec2(window.width(), window.height()));
                    viewport.scale_factor = window.scale_factor();
                }
            }
            SurfaceId::Texture(_id) => {
                // TODO: not implemented yet
            }
        }
    }
}
