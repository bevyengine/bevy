use super::SurfaceId;
use bevy_app::prelude::EventReader;
use bevy_ecs::{Query, Res};
use bevy_math::{clamp, vec2, BVec2, Vec2};
use bevy_reflect::{Reflect, ReflectComponent};
use bevy_utils::HashMap;
use bevy_window::{WindowId, WindowResized, WindowScaleFactorChanged, Windows};

#[derive(Debug, PartialEq, Clone, Reflect)]
#[reflect(Component)]
pub struct Viewport {
    #[reflect(ignore)]
    pub surface: SurfaceId,
    // relative values and respective use mask
    #[reflect(ignore)]
    pub use_relative_origin: BVec2,
    pub relative_origin: Vec2,
    #[reflect(ignore)]
    pub use_relative_size: BVec2,
    pub relative_size: Vec2,
    // absolute values, possibly computed
    pub origin: Vec2,
    pub size: Vec2,
    pub scale_factor: f64,
}

impl Viewport {
    pub fn physical_origin(&self) -> Vec2 {
        (self.origin.as_f64() * self.scale_factor).as_f32()
    }

    pub fn physical_size(&self) -> Vec2 {
        (self.size.as_f64() * self.scale_factor).as_f32()
    }

    pub fn set_scale_factor(&mut self, scale_factor: f64) {
        self.scale_factor = scale_factor;
    }

    pub fn update_rectangle(&mut self, surface_size: Vec2) {
        self.origin = Vec2::select(
            self.use_relative_origin,
            self.relative_origin * surface_size,
            self.origin,
        );
        self.size = Vec2::select(
            self.use_relative_size,
            self.relative_size * surface_size,
            self.size,
        );
        clamp(self.origin, Vec2::zero(), surface_size);
        clamp(self.size, self.origin, surface_size);
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            surface: WindowId::primary().into(),
            use_relative_origin: BVec2::new(true, true),
            relative_origin: Vec2::zero(),
            use_relative_size: BVec2::new(true, true),
            relative_size: Vec2::one(),
            origin: Vec2::zero(),
            size: Vec2::one(),
            scale_factor: 1.0,
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
                    viewport.set_scale_factor(window.scale_factor());
                }
            }
        }
    }
}
