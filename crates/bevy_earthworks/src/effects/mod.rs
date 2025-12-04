//! Visual effects for earthworks simulation.
//!
//! This module provides particle effects and visual feedback for:
//! - Excavation dust clouds
//! - Material dumps
//! - Machine movement trails
//! - Achievement celebrations

use bevy_app::prelude::*;
use bevy_asset::prelude::*;
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_math::prelude::Sphere;
use bevy_math::Vec3;
use bevy_mesh::prelude::*;
use bevy_pbr::prelude::*;
use bevy_time::Time;
use bevy_transform::prelude::*;

/// Plugin for visual effects.
pub struct EffectsPlugin;

impl Plugin for EffectsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EffectsConfig>()
            .add_systems(Update, (update_particles, cleanup_expired_particles));
    }
}

/// Configuration for visual effects.
#[derive(Resource, Clone, Debug)]
pub struct EffectsConfig {
    /// Whether effects are enabled.
    pub enabled: bool,
    /// Dust particle count multiplier.
    pub dust_multiplier: f32,
    /// Trail length.
    pub trail_length: u32,
    /// Achievement particle count.
    pub achievement_particle_count: u32,
}

impl Default for EffectsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            dust_multiplier: 1.0,
            trail_length: 100,
            achievement_particle_count: 20,
        }
    }
}

/// Component for particle entities.
#[derive(Component)]
pub struct Particle {
    /// Velocity in world units per second.
    pub velocity: Vec3,
    /// Lifetime remaining in seconds.
    pub lifetime: f32,
    /// Maximum lifetime for alpha calculation.
    pub max_lifetime: f32,
    /// Gravity multiplier.
    pub gravity: f32,
}

/// Spawns achievement celebration particles at a world position.
pub fn spawn_achievement_particles(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    config: &EffectsConfig,
) {
    if !config.enabled {
        return;
    }

    let particle_mesh = meshes.add(Sphere::new(0.1));

    // Gold and purple colors for achievement particles
    let colors = [
        Color::srgb(1.0, 0.85, 0.2), // Gold
        Color::srgb(0.8, 0.5, 1.0),  // Purple
        Color::srgb(1.0, 1.0, 0.5),  // Light yellow
    ];

    for i in 0..config.achievement_particle_count {
        let angle = (i as f32 / config.achievement_particle_count as f32) * core::f32::consts::TAU;
        let speed = 2.0 + (i as f32 * 0.5) % 3.0;
        let upward = 3.0 + (i as f32 * 0.3) % 2.0;

        let velocity = Vec3::new(
            bevy_math::ops::cos(angle) * speed,
            upward,
            bevy_math::ops::sin(angle) * speed,
        );

        let color = colors[i as usize % colors.len()];
        let material = materials.add(StandardMaterial {
            base_color: color,
            emissive: color.into(),
            unlit: true,
            ..Default::default()
        });

        commands.spawn((
            Mesh3d(particle_mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(position),
            Particle {
                velocity,
                lifetime: 1.5,
                max_lifetime: 1.5,
                gravity: 5.0,
            },
        ));
    }
}

/// Updates particle positions and lifetimes.
fn update_particles(time: Res<Time>, mut query: Query<(&mut Transform, &mut Particle)>) {
    let dt = time.delta_secs();

    for (mut transform, mut particle) in query.iter_mut() {
        // Apply gravity
        particle.velocity.y -= particle.gravity * dt;

        // Update position
        transform.translation += particle.velocity * dt;

        // Update lifetime
        particle.lifetime -= dt;

        // Scale down as lifetime decreases
        let scale = (particle.lifetime / particle.max_lifetime).max(0.0);
        transform.scale = Vec3::splat(scale);
    }
}

/// Removes expired particles.
fn cleanup_expired_particles(mut commands: Commands, query: Query<(Entity, &Particle)>) {
    for (entity, particle) in query.iter() {
        if particle.lifetime <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Spawns dirt/dust particles at excavation location.
pub fn spawn_excavation_dust(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
    direction: Vec3,
    volume: f32,
    config: &EffectsConfig,
) {
    if !config.enabled {
        return;
    }

    let particle_mesh = meshes.add(Sphere::new(0.08));

    // Earthy colors
    let colors = [
        Color::srgb(0.5, 0.35, 0.2),  // Dark brown
        Color::srgb(0.6, 0.45, 0.3),  // Medium brown
        Color::srgb(0.4, 0.3, 0.2),   // Darker
    ];

    let count = ((volume * 30.0 * config.dust_multiplier) as u32).min(40);

    for i in 0..count {
        // Scatter pattern - mostly forward and up
        let spread = (i as f32 * 1.618) % 1.0; // Golden ratio for good distribution
        let angle = spread * core::f32::consts::TAU;
        let lateral = Vec3::new(
            bevy_math::ops::cos(angle) * spread * 2.0,
            0.0,
            bevy_math::ops::sin(angle) * spread * 2.0,
        );

        let up_speed = 1.5 + spread * 2.0;
        let forward_speed = 1.0 + spread * 1.5;

        let velocity = direction * forward_speed + Vec3::Y * up_speed + lateral * 0.5;

        let color = colors[i as usize % colors.len()];
        let material = materials.add(StandardMaterial {
            base_color: color,
            perceptual_roughness: 1.0,
            ..Default::default()
        });

        commands.spawn((
            Mesh3d(particle_mesh.clone()),
            MeshMaterial3d(material),
            Transform::from_translation(position + Vec3::Y * 0.2),
            Particle {
                velocity,
                lifetime: 0.6 + spread * 0.4,
                max_lifetime: 1.0,
                gravity: 8.0,
            },
        ));
    }
}
