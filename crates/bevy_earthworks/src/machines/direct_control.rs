//! Direct player control for machines.
//!
//! This module provides keyboard-based control for construction machines,
//! allowing players to directly drive bulldozers and excavators.
//!
//! Features:
//! - Smooth input processing with exponential interpolation
//! - Camera shake integration for game feel
//! - Engine sound with dynamic pitch
//! - Blade load visualization

use bevy_app::prelude::*;
use bevy_asset::prelude::*;
#[cfg(feature = "audio")]
use bevy_audio::{AudioPlayer, AudioSink, AudioSinkPlayback, PlaybackSettings, Volume};
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_input::prelude::*;
use bevy_math::{IVec3, Quat, Vec3};
use bevy_mesh::prelude::*;
use bevy_pbr::prelude::*;
use bevy_reflect::Reflect;
use bevy_time::Time;
use bevy_transform::components::Transform;

use super::{Machine, MachineActivity, MachineType, Mobility};
use crate::camera::CameraShake;
use crate::effects::{spawn_excavation_dust, EffectsConfig};
use crate::terrain::{
    excavate, fill, get_terrain_height_interpolated, Aabb, Chunk, MaterialId,
    TerrainModifiedEvent, VoxelTerrain,
};

/// Marker for the currently player-controlled machine.
#[derive(Component, Default, Reflect)]
pub struct PlayerControlled;

/// Control response smoothing component.
///
/// Provides exponential smoothing for responsive but not twitchy controls.
#[derive(Component, Clone, Debug, Reflect)]
pub struct ControlResponse {
    /// Smoothing time constant (lower = more responsive, higher = smoother).
    pub smoothing: f32,
    /// Current smoothed throttle value (-1 to 1).
    pub smoothed_throttle: f32,
    /// Current smoothed steering value (-1 to 1).
    pub smoothed_steering: f32,
    /// Target throttle from input.
    pub target_throttle: f32,
    /// Target steering from input.
    pub target_steering: f32,
}

impl Default for ControlResponse {
    fn default() -> Self {
        Self {
            smoothing: 0.1, // 100ms smoothing time
            smoothed_throttle: 0.0,
            smoothed_steering: 0.0,
            target_throttle: 0.0,
            target_steering: 0.0,
        }
    }
}

impl ControlResponse {
    /// Creates a control response with custom smoothing.
    pub fn with_smoothing(smoothing: f32) -> Self {
        Self {
            smoothing,
            ..Default::default()
        }
    }
}

/// Blade/bucket position state for dozers and excavators.
#[derive(Component, Debug, Clone, Reflect)]
pub struct BladeState {
    /// Vertical offset from default position (-1.0 to 1.0)
    /// -1.0 = fully lowered (digging)
    /// 0.0 = level (pushing)
    /// 1.0 = raised (carrying/traveling)
    pub height: f32,
    /// Current material load in cubic meters.
    pub load: f32,
    /// Maximum load capacity in cubic meters.
    pub capacity: f32,
}

impl Default for BladeState {
    fn default() -> Self {
        Self {
            height: 0.0,
            load: 0.0,
            capacity: 3.0,
        }
    }
}

impl BladeState {
    /// Creates a blade state with the given capacity.
    pub fn with_capacity(capacity: f32) -> Self {
        Self {
            height: 0.0,
            load: 0.0,
            capacity,
        }
    }

    /// Returns the blade position as a descriptive string.
    pub fn position_name(&self) -> &'static str {
        if self.height < -0.3 {
            "LOWERED"
        } else if self.height > 0.3 {
            "RAISED"
        } else {
            "LEVEL"
        }
    }

    /// Returns true if the blade is full.
    pub fn is_full(&self) -> bool {
        self.load >= self.capacity * 0.95
    }

    /// Returns true if the blade is empty.
    pub fn is_empty(&self) -> bool {
        self.load < 0.01
    }
}

/// Plugin for direct machine control.
pub struct DirectControlPlugin;

impl Plugin for DirectControlPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<DozerPushState>()
            .register_type::<BladeLoadPile>()
            .register_type::<ControlResponse>()
            .add_systems(
                Update,
                (
                    spawn_control_response,
                    spawn_blade_load_visual,
                    read_dozer_input,
                    smooth_controls,
                    apply_smoothed_movement,
                    update_machine_on_terrain,
                    blade_terrain_interaction,
                    animate_blade_visual,
                    update_blade_load_visual,
                )
                    .chain(),
            );

        #[cfg(feature = "audio")]
        {
            app.register_type::<EngineSound>()
                .register_type::<EngineSoundSpawned>()
                .add_systems(Update, (spawn_engine_sound, update_engine_sound));
        }
    }
}

/// Marker component for a blade mesh entity (child of dozer).
#[derive(Component, Default, Reflect)]
pub struct BladeVisual;

/// Marker for the engine audio source.
#[cfg(feature = "audio")]
#[derive(Component, Default, Reflect)]
pub struct EngineSound;

/// Marker that engine sound was already attempted to be spawned.
#[cfg(feature = "audio")]
#[derive(Component, Default, Reflect)]
pub struct EngineSoundSpawned;

/// Marker for the dirt pile mesh in front of blade.
#[derive(Component, Default, Reflect)]
pub struct BladeLoadPile;

/// Spawns control response component for player-controlled machines.
fn spawn_control_response(
    mut commands: Commands,
    query: Query<Entity, (With<PlayerControlled>, Without<ControlResponse>)>,
) {
    for entity in query.iter() {
        commands.entity(entity).insert(ControlResponse::default());
    }
}

/// Read raw keyboard input and set target values.
fn read_dozer_input(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<
        (
            &mut ControlResponse,
            &mut MachineActivity,
            &mut BladeState,
            &Machine,
            &Mobility,
        ),
        With<PlayerControlled>,
    >,
) {
    let dt = time.delta_secs();

    for (mut control, mut activity, mut blade, machine, mobility) in query.iter_mut() {
        // Only control dozers with this system
        if machine.machine_type != MachineType::Dozer {
            continue;
        }

        // Read movement input
        let mut forward: f32 = 0.0;
        let mut turn: f32 = 0.0;

        if keyboard.pressed(KeyCode::KeyW) {
            forward += 1.0;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            forward -= if mobility.can_reverse { 1.0 } else { 0.5 };
        }
        if keyboard.pressed(KeyCode::KeyA) {
            turn += 1.0;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            turn -= 1.0;
        }

        // Blade control (direct, not smoothed)
        if keyboard.pressed(KeyCode::KeyQ) {
            blade.height = (blade.height + dt * 2.0).min(1.0);
        }
        if keyboard.pressed(KeyCode::KeyE) {
            blade.height = (blade.height - dt * 2.0).max(-1.0);
        }

        // Quick stop
        if keyboard.just_pressed(KeyCode::Space) {
            forward = 0.0;
            turn = 0.0;
            control.smoothed_throttle = 0.0;
            control.smoothed_steering = 0.0;
            *activity = MachineActivity::Idle;
        }

        // Set target values for smoothing
        control.target_throttle = forward;
        control.target_steering = turn;
    }
}

/// Apply exponential smoothing to control inputs.
fn smooth_controls(time: Res<Time>, mut query: Query<&mut ControlResponse, With<PlayerControlled>>) {
    let dt = time.delta_secs();

    for mut control in query.iter_mut() {
        // Exponential smoothing: output = output + alpha * (target - output)
        // Where alpha = 1 - e^(-dt / smoothing)
        let alpha = 1.0 - (-dt / control.smoothing.max(0.001)).exp();

        control.smoothed_throttle +=
            alpha * (control.target_throttle - control.smoothed_throttle);
        control.smoothed_steering +=
            alpha * (control.target_steering - control.smoothed_steering);

        // Deadzone to fully stop when near zero
        if control.smoothed_throttle.abs() < 0.01 && control.target_throttle == 0.0 {
            control.smoothed_throttle = 0.0;
        }
        if control.smoothed_steering.abs() < 0.01 && control.target_steering == 0.0 {
            control.smoothed_steering = 0.0;
        }
    }
}

/// Apply smoothed control values to machine movement.
fn apply_smoothed_movement(
    time: Res<Time>,
    mut query: Query<
        (
            &mut Transform,
            &Mobility,
            &mut MachineActivity,
            &ControlResponse,
            &Machine,
        ),
        With<PlayerControlled>,
    >,
) {
    let dt = time.delta_secs();

    for (mut transform, mobility, mut activity, control, machine) in query.iter_mut() {
        if machine.machine_type != MachineType::Dozer {
            continue;
        }

        let forward = control.smoothed_throttle;
        let turn = control.smoothed_steering;

        // Apply rotation (tracked vehicles can pivot in place)
        if turn.abs() > 0.01 {
            // Turn rate is slower when moving fast
            let speed_factor = if forward.abs() > 0.5 { 0.5 } else { 1.0 };
            let rotation = Quat::from_rotation_y(turn * mobility.turn_rate * speed_factor * dt);
            transform.rotation = transform.rotation * rotation;
        }

        // Apply movement
        if forward.abs() > 0.01 {
            let speed = forward * mobility.max_speed;
            let direction = transform.rotation * Vec3::NEG_Z;
            transform.translation += direction * speed * dt;

            // Update activity to show we're traveling
            *activity = MachineActivity::Traveling {
                target: transform.translation + direction * 10.0,
                progress: 0.0,
                start: transform.translation,
            };
        } else if matches!(*activity, MachineActivity::Traveling { .. }) {
            *activity = MachineActivity::Idle;
        }
    }
}

/// Keep machine on terrain surface and block movement into terrain.
fn update_machine_on_terrain(
    terrain: Res<VoxelTerrain>,
    chunks: Query<&Chunk>,
    mut machines: Query<(&mut Transform, &BladeState, Option<&ControlResponse>), With<PlayerControlled>>,
) {
    for (mut transform, blade, control) in machines.iter_mut() {
        // Get terrain height at current position
        let current_height = get_terrain_height_interpolated(
            &terrain,
            &chunks,
            transform.translation.x,
            transform.translation.z,
        );

        // Get terrain height at the front of the machine (where we're trying to go)
        let forward = transform.rotation * Vec3::NEG_Z;
        let probe_distance = 2.5; // Distance to probe ahead
        let front_pos = transform.translation + forward * probe_distance;

        let front_height = get_terrain_height_interpolated(
            &terrain,
            &chunks,
            front_pos.x,
            front_pos.z,
        );

        if let Some(height) = current_height {
            // Machine center is about 0.5m above ground
            let target_y = height + 0.5;

            // Check if we're trying to move into terrain that's too high
            if let Some(front_h) = front_height {
                let height_diff = front_h - height;
                let max_climb = if blade.height <= 0.0 {
                    // With blade down, can push through more (excavating)
                    1.5
                } else {
                    // With blade up, can only climb gentle slopes
                    0.8
                };

                // If terrain ahead is too high and we're moving forward, block movement
                if height_diff > max_climb {
                    if let Some(ctrl) = control {
                        if ctrl.smoothed_throttle > 0.1 {
                            // Push back slightly to prevent clipping
                            let pushback = forward * -0.1;
                            transform.translation += pushback;
                        }
                    }
                }
            }

            // Smoothly adjust to terrain height
            transform.translation.y = transform.translation.y * 0.85 + target_y * 0.15;
        }
    }
}

/// Track whether the dozer was pushing in the previous frame (for deposit detection).
#[derive(Component, Default, Reflect)]
pub struct DozerPushState {
    /// Was actively pushing material last frame
    pub was_pushing: bool,
    /// Previous blade height (to detect raising)
    pub prev_blade_height: f32,
}

/// Handle blade-terrain interaction using realistic push mechanics.
///
/// Bulldozer blade mechanics:
/// - Blade DOWN + moving FORWARD = excavate at blade, accumulate load (material pushed in front)
/// - STOP or REVERSE with load = deposit pile where it is (stop pushing)
/// - RAISE blade with load = deposit pile at blade position
/// - Blade UP = travel mode, no terrain interaction
fn blade_terrain_interaction(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    effects_config: Res<EffectsConfig>,
    mut terrain: ResMut<VoxelTerrain>,
    mut chunks: Query<&mut Chunk>,
    mut events: MessageWriter<TerrainModifiedEvent>,
    mut cameras: Query<&mut CameraShake>,
    mut dozers: Query<
        (
            Entity,
            &Transform,
            &mut BladeState,
            &MachineActivity,
            &Machine,
            Option<&mut DozerPushState>,
        ),
        With<PlayerControlled>,
    >,
) {
    for (entity, transform, mut blade, activity, machine, push_state) in dozers.iter_mut() {
        if machine.machine_type != MachineType::Dozer {
            continue;
        }

        // Determine movement state
        let is_moving_forward = matches!(activity, MachineActivity::Traveling { .. });

        // Check if blade is in working position (at or below ground level)
        let blade_down = blade.height <= 0.0;
        let is_pushing = blade_down && is_moving_forward;

        // Get or create push state
        let (was_pushing, prev_blade_height) = if let Some(ref state) = push_state {
            (state.was_pushing, state.prev_blade_height)
        } else {
            // First frame - add component
            commands.entity(entity).insert(DozerPushState::default());
            (false, 0.0)
        };

        // Detect blade being raised (transition from down to up)
        let blade_just_raised = prev_blade_height <= 0.0 && blade.height > 0.0;

        // Get blade world position (front of dozer)
        let forward = transform.rotation * Vec3::NEG_Z;
        let blade_offset = 2.0; // Distance from dozer center to blade
        let blade_center = transform.translation + forward * blade_offset;

        let voxel_size = terrain.voxel_size();

        // Blade dimensions in voxels (~3m wide)
        let blade_half_width = 5;

        if is_pushing {
            // PUSHING MODE: Blade is down and moving forward
            // 1. Excavate terrain at blade position
            // 2. Accumulate into blade load

            // Calculate dig depth based on blade height (0 = ground level, -1 = max dig)
            let dig_depth = (-blade.height).max(0.0); // 0 to 1
            let voxels_deep = ((dig_depth * 3.0) + 1.0).ceil() as i32; // 1-4 voxels

            // Convert blade position to voxel coordinates
            let blade_voxel_x = (blade_center.x / voxel_size).floor().clamp(-10000.0, 10000.0) as i32;
            let blade_voxel_z = (blade_center.z / voxel_size).floor().clamp(-10000.0, 10000.0) as i32;

            // Scan a vertical column to find terrain surface
            let max_scan_y = 20i32;
            let mut surface_voxel_y = 0i32;

            'scan: for test_y in (0..max_scan_y).rev() {
                let test_pos = IVec3::new(blade_voxel_x, test_y, blade_voxel_z);
                let chunk_coord = terrain.voxel_to_chunk(test_pos);
                if let Some(chunk_entity) = terrain.get_chunk_entity(chunk_coord) {
                    if let Ok(chunk) = chunks.get(chunk_entity) {
                        let local = terrain.voxel_to_local(test_pos);
                        if local.x >= 0
                            && (local.x as usize) < 16
                            && local.y >= 0
                            && (local.y as usize) < 16
                            && local.z >= 0
                            && (local.z as usize) < 16
                        {
                            let voxel =
                                chunk.get(local.x as usize, local.y as usize, local.z as usize);
                            if voxel.is_solid() {
                                surface_voxel_y = test_y;
                                break 'scan;
                            }
                        }
                    }
                }
            }

            // Only excavate if we found terrain and are in reasonable range
            if surface_voxel_y > 0
                && blade_voxel_x.abs() < 1000
                && blade_voxel_z.abs() < 1000
            {
                let excavate_bounds = Aabb::new(
                    IVec3::new(
                        blade_voxel_x.saturating_sub(blade_half_width),
                        (surface_voxel_y - voxels_deep + 1).max(0),
                        blade_voxel_z.saturating_sub(1),
                    ),
                    IVec3::new(
                        blade_voxel_x.saturating_add(blade_half_width),
                        surface_voxel_y,
                        blade_voxel_z.saturating_add(1),
                    ),
                );

                if blade.load < blade.capacity {
                    let excavated = excavate(
                        &mut commands,
                        &mut terrain,
                        &mut chunks,
                        excavate_bounds,
                        &mut events,
                    );

                    if excavated > 0 {
                        let volume = excavated as f32 * voxel_size.powi(3);
                        blade.load = (blade.load + volume).min(blade.capacity);

                        // Spawn dirt particles
                        spawn_excavation_dust(
                            &mut commands,
                            &mut meshes,
                            &mut materials,
                            blade_center,
                            forward,
                            volume,
                            &effects_config,
                        );

                        // Add camera shake proportional to excavation
                        let trauma = (volume * 0.05).min(0.1);
                        for mut shake in cameras.iter_mut() {
                            shake.add_trauma(trauma);
                        }

                        bevy_log::debug!(
                            "EXCAVATING: {} voxels, load: {:.2}/{:.2} m³",
                            excavated, blade.load, blade.capacity
                        );
                    }
                }
            }
        } else if blade.load > 0.01 {
            // NOT PUSHING but have load - need to deposit

            // Deposit when:
            // 1. Stopped (was pushing, now not)
            // 2. Reversing (not moving forward)
            // 3. Blade raised (blade_just_raised)

            let should_deposit = was_pushing || blade_just_raised;

            if should_deposit {
                // Deposit the pile at the blade's current position
                let deposit_voxel_x =
                    (blade_center.x / voxel_size).floor().clamp(-10000.0, 10000.0) as i32;
                let deposit_voxel_y =
                    ((blade_center.y - 0.3) / voxel_size).floor().clamp(-10000.0, 10000.0) as i32;
                let deposit_voxel_z =
                    (blade_center.z / voxel_size).floor().clamp(-10000.0, 10000.0) as i32;

                // Pile dimensions based on load
                let pile_height = ((blade.load / blade.capacity) * 3.0).ceil() as i32;
                let pile_width = blade_half_width;

                // Only deposit if we're in a reasonable coordinate range
                if deposit_voxel_x.abs() < 1000
                    && deposit_voxel_y >= 0
                    && deposit_voxel_z.abs() < 1000
                {
                    let deposit_bounds = Aabb::new(
                        IVec3::new(
                            deposit_voxel_x.saturating_sub(pile_width),
                            deposit_voxel_y,
                            deposit_voxel_z.saturating_sub(1),
                        ),
                        IVec3::new(
                            deposit_voxel_x.saturating_add(pile_width),
                            deposit_voxel_y.saturating_add(pile_height),
                            deposit_voxel_z.saturating_add(1),
                        ),
                    );

                    let filled = fill(
                        &mut commands,
                        &mut terrain,
                        &mut chunks,
                        deposit_bounds,
                        MaterialId::Dirt,
                        true, // disturbed soil
                        &mut events,
                    );

                    if filled > 0 {
                        let volume_deposited = filled as f32 * voxel_size.powi(3);
                        blade.load = (blade.load - volume_deposited).max(0.0);

                        // Camera shake on dump
                        for mut shake in cameras.iter_mut() {
                            shake.add_trauma(0.12);
                        }

                        bevy_log::debug!(
                            "DEPOSITED: {} voxels at blade, remaining load: {:.2} m³",
                            filled, blade.load
                        );
                    } else if blade_just_raised {
                        // If we raised blade but couldn't deposit (blocked), just drop load
                        bevy_log::debug!(
                            "DROPPED: {:.2} m³ (blade raised, nowhere to deposit)",
                            blade.load
                        );
                        blade.load = 0.0;
                    }
                } else if blade_just_raised {
                    // Outside valid range, just drop the load
                    blade.load = 0.0;
                }
            }
        }

        // Update push state for next frame
        if let Some(mut state) = push_state {
            state.was_pushing = is_pushing;
            state.prev_blade_height = blade.height;
        }
    }
}

/// Animate blade visual position based on BladeState.height.
///
/// Blade height mapping:
/// - height = -1.0: Blade fully lowered (digging) -> Y offset = -0.8
/// - height =  0.0: Blade at ground level -> Y offset = -0.3
/// - height =  1.0: Blade raised (travel) -> Y offset = 0.3
fn animate_blade_visual(
    dozers: Query<(&BladeState, &Children), With<PlayerControlled>>,
    mut blade_visuals: Query<&mut Transform, With<BladeVisual>>,
) {
    for (blade_state, children) in dozers.iter() {
        for child in children.iter() {
            if let Ok(mut blade_transform) = blade_visuals.get_mut(child) {
                // Map blade.height (-1 to 1) to Y offset (-0.8 to 0.3)
                let target_y = -0.3 + blade_state.height * 0.5;

                // Smooth interpolation using easing
                let current_y = blade_transform.translation.y;
                let eased = current_y + (target_y - current_y) * 0.15;
                blade_transform.translation.y = eased;
            }
        }
    }
}

/// Spawns engine sound for player-controlled machines.
#[cfg(feature = "audio")]
fn spawn_engine_sound(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    player_query: Query<Entity, (With<PlayerControlled>, Without<EngineSoundSpawned>)>,
) {
    for entity in player_query.iter() {
        // Mark that we tried to spawn engine sound (prevents repeated attempts)
        commands.entity(entity).insert(EngineSoundSpawned);

        // Try to load engine sound - gracefully skip if not found
        let sound_handle = asset_server.load("sounds/engine_idle.ogg");

        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                AudioPlayer::new(sound_handle),
                PlaybackSettings::LOOP.with_volume(Volume::Linear(0.3)),
                EngineSound,
            ));
        });
    }
}

/// Updates engine sound pitch based on movement speed.
#[cfg(feature = "audio")]
fn update_engine_sound(
    player_query: Query<(&MachineActivity, Option<&ControlResponse>), With<PlayerControlled>>,
    engine_query: Query<&AudioSink, With<EngineSound>>,
) {
    let Ok((activity, control)) = player_query.single() else {
        return;
    };
    let Ok(sink) = engine_query.single() else {
        return;
    };

    // Base pitch when idle, higher when moving
    let base_factor = match activity {
        MachineActivity::Traveling { .. } => 1.3,
        MachineActivity::Idle => 0.9,
        _ => 1.0,
    };

    // Add variation based on throttle for more dynamic sound
    let throttle_factor = control.map(|c| c.smoothed_throttle.abs() * 0.2).unwrap_or(0.0);

    sink.set_speed(base_factor + throttle_factor);
}

/// Spawns dirt pile visual for player-controlled dozers.
fn spawn_blade_load_visual(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    blade_query: Query<Entity, (With<BladeVisual>, Without<BladeLoadPile>)>,
) {
    use bevy_math::prelude::Sphere;

    for blade_entity in blade_query.iter() {
        let pile_mesh = meshes.add(Sphere::new(0.5));
        let pile_material = materials.add(StandardMaterial {
            base_color: Color::srgb(0.55, 0.4, 0.25),
            perceptual_roughness: 1.0,
            ..Default::default()
        });

        commands.entity(blade_entity).with_children(|parent| {
            parent.spawn((
                Mesh3d(pile_mesh),
                MeshMaterial3d(pile_material),
                Transform::from_xyz(0.0, -0.3, -0.5).with_scale(Vec3::ZERO),
                BladeLoadPile,
            ));
        });
    }
}

/// Updates dirt pile size based on blade load.
fn update_blade_load_visual(
    blade_query: Query<&BladeState, With<PlayerControlled>>,
    mut pile_query: Query<&mut Transform, With<BladeLoadPile>>,
) {
    let Ok(blade) = blade_query.single() else {
        return;
    };

    for mut transform in pile_query.iter_mut() {
        // Scale pile based on load (0 to capacity maps to 0 to 1.5 scale)
        let load_ratio = (blade.load / blade.capacity).min(1.0);
        let target_scale = load_ratio * 1.5;

        // Smooth transition with easing
        let current = transform.scale.x;
        let eased = current + (target_scale - current) * 0.1;
        transform.scale = Vec3::splat(eased.max(0.01));
    }
}
