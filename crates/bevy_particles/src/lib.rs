use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_render::{
    prelude::{ComputedVisibility, Visibility},
    primitives::Aabb,
};
use bevy_transform::prelude::{GlobalTransform, Transform};

pub mod emitter;
pub mod material;
pub mod modifiers;
mod particles;
pub mod prelude;
mod render;

pub use emitter::*;
pub use material::*;
use modifiers::*;
pub use particles::*;
use render::ParticleRenderPlugin;

#[derive(Clone, Debug, Eq, Hash, PartialEq, SystemLabel)]
pub struct ParticleUpdate;

#[derive(Default)]
pub struct ParticlePlugin;

impl Plugin for ParticlePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ParticleMaterialPlugin)
            .add_plugin(ParticleRenderPlugin)
            .add_system(particles::update_particles.label(ParticleUpdate))
            .add_system(emitter::emit_particles.after(ParticleUpdate))
            .register_particle_modifier::<ConstantForce>();
    }
}

pub trait ParticleModifierAppExt {
    fn register_particle_modifier<T: ParticleModifier>(&mut self) -> &mut Self;
}

impl ParticleModifierAppExt for App {
    fn register_particle_modifier<T: ParticleModifier>(&mut self) -> &mut Self {
        self.add_system(modifiers::apply_particle_modifier::<T>.before(ParticleUpdate));
        self
    }
}

#[derive(Bundle, Default)]
pub struct ParticleBundle {
    pub particles: Particles,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub aabb: Aabb,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
}
