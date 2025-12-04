//! Minimap UI component for terrain overview.
//!
//! Provides a small overhead view showing:
//! - Terrain topology (height-based coloring)
//! - Machine positions and types
//! - Camera viewport indicator

use bevy_app::prelude::*;
use bevy_camera::prelude::{Camera3d, Visibility};
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_math::Vec3;
use bevy_text::{TextColor, TextFont};
use bevy_transform::components::Transform;
use bevy_ui::prelude::*;

use crate::config::EarthworksConfig;
use crate::machines::{Machine, MachineType};
use crate::terrain::VoxelTerrain;

/// Plugin for the minimap UI.
pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MinimapState>()
            .add_systems(Startup, spawn_minimap)
            .add_systems(
                Update,
                (update_minimap_visibility, update_minimap_content).chain(),
            );
    }
}

/// Minimap configuration and state.
#[derive(Resource)]
pub struct MinimapState {
    /// Whether the minimap is visible.
    pub visible: bool,
    /// Size of the minimap in pixels.
    pub size: f32,
    /// World bounds to display (min_x, min_z, max_x, max_z).
    pub world_bounds: (f32, f32, f32, f32),
    /// Zoom level (1.0 = fit world bounds).
    pub zoom: f32,
}

impl Default for MinimapState {
    fn default() -> Self {
        Self {
            visible: true,
            size: 200.0,
            world_bounds: (-50.0, -50.0, 50.0, 50.0),
            zoom: 1.0,
        }
    }
}

/// Marker for the minimap container.
#[derive(Component)]
pub struct MinimapContainer;

/// Marker for the minimap background.
#[derive(Component)]
pub struct MinimapBackground;

/// Marker for machine blips on the minimap.
#[derive(Component)]
pub struct MinimapBlip {
    /// The entity this blip represents.
    pub entity: Entity,
}

/// Marker for the camera viewport indicator.
#[derive(Component)]
pub struct MinimapViewport;

// UI Colors
const MINIMAP_BG: Color = Color::srgba(0.1, 0.15, 0.1, 0.9);
const MINIMAP_BORDER: Color = Color::srgba(0.3, 0.4, 0.3, 1.0);
const TERRAIN_LOW: Color = Color::srgba(0.3, 0.25, 0.2, 1.0); // Brown for low terrain
const TERRAIN_HIGH: Color = Color::srgba(0.4, 0.5, 0.35, 1.0); // Green for high terrain
const BLIP_PLAYER: Color = Color::srgba(0.2, 0.8, 0.2, 1.0); // Green for player units
const BLIP_EXCAVATOR: Color = Color::srgba(1.0, 0.8, 0.2, 1.0); // Yellow
const BLIP_DOZER: Color = Color::srgba(1.0, 0.6, 0.2, 1.0); // Orange
const BLIP_LOADER: Color = Color::srgba(0.2, 0.6, 1.0, 1.0); // Blue
const BLIP_DUMP_TRUCK: Color = Color::srgba(0.8, 0.2, 0.2, 1.0); // Red
const VIEWPORT_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 0.5);

/// Spawns the minimap UI.
fn spawn_minimap(mut commands: Commands, state: Res<MinimapState>) {
    // Minimap container - top right corner
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(20.0),
                right: Val::Px(20.0),
                width: Val::Px(state.size),
                height: Val::Px(state.size),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            MinimapContainer,
        ))
        .with_children(|parent| {
            // Title bar
            parent.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(20.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                BackgroundColor(MINIMAP_BORDER),
            ))
            .with_children(|parent| {
                parent.spawn((
                    Text::new("MAP"),
                    TextFont {
                        font_size: 12.0,
                        ..Default::default()
                    },
                    TextColor(Color::WHITE),
                ));
            });

            // Map area
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        border: UiRect::all(Val::Px(2.0)),
                        position_type: PositionType::Relative,
                        ..Default::default()
                    },
                    BackgroundColor(MINIMAP_BG),
                    BorderColor::all(MINIMAP_BORDER),
                    MinimapBackground,
                ))
                .with_children(|parent| {
                    // Viewport indicator (shows camera view area)
                    parent.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            width: Val::Px(30.0),
                            height: Val::Px(20.0),
                            border: UiRect::all(Val::Px(1.0)),
                            left: Val::Percent(45.0),
                            top: Val::Percent(45.0),
                            ..Default::default()
                        },
                        BorderColor::all(VIEWPORT_COLOR),
                        MinimapViewport,
                    ));
                });
        });
}

/// Updates minimap visibility based on config.
fn update_minimap_visibility(
    config: Res<EarthworksConfig>,
    minimap_state: Res<MinimapState>,
    mut container_query: Query<&mut Visibility, With<MinimapContainer>>,
) {
    for mut visibility in container_query.iter_mut() {
        *visibility = if config.show_ui && minimap_state.visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// Updates minimap content (blips, viewport).
fn update_minimap_content(
    mut commands: Commands,
    state: Res<MinimapState>,
    minimap_bg_query: Query<(Entity, &ComputedNode), With<MinimapBackground>>,
    camera_query: Query<&Transform, With<Camera3d>>,
    machines_query: Query<(Entity, &Transform, &Machine)>,
    mut existing_blips: Query<(Entity, &MinimapBlip, &mut Node)>,
    mut viewport_query: Query<&mut Node, (With<MinimapViewport>, Without<MinimapBlip>)>,
) {
    let Ok((minimap_entity, minimap_node)) = minimap_bg_query.single() else {
        return;
    };

    let minimap_size = minimap_node.size();
    let (min_x, min_z, max_x, max_z) = state.world_bounds;
    let world_width = (max_x - min_x) / state.zoom;
    let world_height = (max_z - min_z) / state.zoom;

    // Update viewport position based on camera
    if let Ok(cam_transform) = camera_query.single() {
        if let Ok(mut viewport_node) = viewport_query.single_mut() {
            let cam_x = (cam_transform.translation.x - min_x) / world_width;
            let cam_z = (cam_transform.translation.z - min_z) / world_height;

            // Convert to percentage position (clamped)
            let percent_x = (cam_x * 100.0).clamp(0.0, 90.0);
            let percent_y = (cam_z * 100.0).clamp(0.0, 90.0);

            viewport_node.left = Val::Percent(percent_x);
            viewport_node.top = Val::Percent(percent_y);
        }
    }

    // Track which entities we've updated
    let mut updated_entities = Vec::new();

    // Update or create blips for machines
    for (machine_entity, transform, machine) in machines_query.iter() {
        let world_x = (transform.translation.x - min_x) / world_width;
        let world_z = (transform.translation.z - min_z) / world_height;

        // Skip if out of bounds
        if world_x < 0.0 || world_x > 1.0 || world_z < 0.0 || world_z > 1.0 {
            continue;
        }

        let percent_x = world_x * 100.0;
        let percent_y = world_z * 100.0;

        // Check if blip already exists
        let existing = existing_blips
            .iter_mut()
            .find(|(_, blip, _)| blip.entity == machine_entity);

        if let Some((blip_entity, _, mut node)) = existing {
            // Update position
            node.left = Val::Percent(percent_x);
            node.top = Val::Percent(percent_y);
            updated_entities.push(blip_entity);
        } else {
            // Create new blip
            let color = machine_type_color(machine.machine_type);
            let blip_entity = commands
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(6.0),
                        height: Val::Px(6.0),
                        left: Val::Percent(percent_x),
                        top: Val::Percent(percent_y),
                        ..Default::default()
                    },
                    BackgroundColor(color),
                    MinimapBlip {
                        entity: machine_entity,
                    },
                ))
                .id();

            commands.entity(minimap_entity).add_child(blip_entity);
            updated_entities.push(blip_entity);
        }
    }

    // Remove blips for entities that no longer exist
    for (blip_entity, blip, _) in existing_blips.iter() {
        if !updated_entities.contains(&blip_entity) {
            // Check if the machine still exists
            if machines_query.get(blip.entity).is_err() {
                commands.entity(blip_entity).despawn();
            }
        }
    }
}

/// Returns the blip color for a machine type.
fn machine_type_color(machine_type: MachineType) -> Color {
    match machine_type {
        MachineType::Excavator => BLIP_EXCAVATOR,
        MachineType::Dozer => BLIP_DOZER,
        MachineType::Loader => BLIP_LOADER,
        MachineType::DumpTruck => BLIP_DUMP_TRUCK,
    }
}
