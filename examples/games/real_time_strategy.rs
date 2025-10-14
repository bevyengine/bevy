//! RTS Game Proof of Concept
//! Features: Unit selection, movement, resource gathering, unit spawning, basic combat

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use rand::Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<GameResources>()
        .init_resource::<SelectionBox>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                camera_movement,
                mouse_button_input,
                draw_selection_box,
                update_selection_visuals,
                update_health_bars,
                move_units,
                resolve_collisions,
                unit_attack,
                resource_gathering,
                update_ui,
                spawn_unit_on_key,
            ),
        )
        .run();
}

// ==================== Components ====================

#[derive(Component)]
struct Unit {
    max_health: f32,
    health: f32,
    speed: f32,
    attack_damage: f32,
    attack_range: f32,
    attack_cooldown: Timer,
    team: Team,
}

#[derive(Component, Clone, Copy, PartialEq)]
enum Team {
    Player,
    Enemy,
}

#[derive(Component)]
struct Selected;

#[derive(Component)]
struct SelectionRing {
    owner: Entity,
}

#[derive(Component)]
struct MoveTo {
    target: Vec3,
}

#[derive(Component)]
struct Waypoints {
    points: Vec<Vec3>,
    current_index: usize,
}

#[derive(Component)]
struct AttackTarget {
    target: Entity,
}

#[derive(Component)]
struct Resource {
    amount: i32,
}

#[derive(Component)]
struct Gatherer {
    gathering_rate: f32,
    gathering_timer: Timer,
    current_resource: Option<Entity>,
    carrying: i32,
    capacity: i32,
    assigned_resource: Option<Entity>, // Manual assignment from player command
    gathering_enabled: bool, // Only gather when explicitly enabled by player
}

#[derive(Component)]
struct MainBase {
    team: Team,
}

#[derive(Component)]
struct HealthBar {
    owner: Entity,
}

#[derive(Component)]
struct HealthBarBackground {
    owner: Entity,
}

#[derive(Component)]
struct UnitMarker;

// ==================== Resources ====================

#[derive(Resource)]
struct GameResources {
    player_resources: i32,
}

impl Default for GameResources {
    fn default() -> Self {
        Self {
            player_resources: 100,
        }
    }
}

#[derive(Resource, Default)]
struct SelectionBox {
    start: Option<Vec2>,
    end: Option<Vec2>,
}

#[derive(Component)]
struct ResourceUI;

#[derive(Component)]
struct SelectionInfoUI;

// ==================== Setup ====================

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 15.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Lighting - Sun-like directional light
    commands.spawn((
        DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 10.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ambient light for better visibility
    commands.insert_resource(AmbientLight {
        color: Color::srgb(0.7, 0.7, 0.6), // Slightly warm ambient light
        brightness: 300.0,
    });

    // Terrain ground plane - earthy brown/green mix
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(100.0, 100.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.35, 0.25), // Earthy brown
            perceptual_roughness: 0.9,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    // Add some grass patches
    let grass_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.3, 0.5, 0.2), // Darker grass green
        perceptual_roughness: 0.95,
        ..default()
    });

    // Create random grass patches for terrain variation
    let mut rng = rand::thread_rng();
    for _ in 0..15 {
        let x = rng.gen_range(-20.0..20.0);
        let z = rng.gen_range(-20.0..20.0);
        let size = rng.gen_range(2.0..5.0);

        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(size, size))),
            MeshMaterial3d(grass_material.clone()),
            Transform::from_xyz(x, 0.01, z), // Slightly above ground to prevent z-fighting
        ));
    }

    // Add some dirt patches
    let dirt_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.5, 0.4, 0.3), // Lighter dirt
        perceptual_roughness: 0.85,
        ..default()
    });

    for _ in 0..10 {
        let x = rng.gen_range(-20.0..20.0);
        let z = rng.gen_range(-20.0..20.0);
        let size = rng.gen_range(1.5..4.0);

        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(size, size))),
            MeshMaterial3d(dirt_material.clone()),
            Transform::from_xyz(x, 0.02, z),
        ));
    }

    // Player base
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3.0, 2.0, 3.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.2, 0.2, 0.8))),
        Transform::from_xyz(-10.0, 1.0, -10.0),
        MainBase { team: Team::Player },
    ));

    // Enemy base
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3.0, 2.0, 3.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.2))),
        Transform::from_xyz(10.0, 1.0, 10.0),
        MainBase { team: Team::Enemy },
    ));

    // Player units
    let player_material = materials.add(Color::srgb(0.3, 0.3, 1.0));
    let unit_mesh = meshes.add(Capsule3d::new(0.3, 1.0));

    for i in 0..3 {
        spawn_unit(
            &mut commands,
            unit_mesh.clone(),
            player_material.clone(),
            Vec3::new(-8.0 + i as f32 * 1.5, 0.5, -8.0),
            Team::Player,
            true, // Workers can gather
        );
    }

    // Enemy units
    let enemy_material = materials.add(Color::srgb(1.0, 0.3, 0.3));

    for i in 0..3 {
        spawn_unit(
            &mut commands,
            unit_mesh.clone(),
            enemy_material.clone(),
            Vec3::new(8.0 + i as f32 * 1.5, 0.5, 8.0),
            Team::Enemy,
            false, // Enemy units don't gather
        );
    }

    // Resources
    let resource_material = materials.add(Color::srgb(1.0, 0.84, 0.0));
    let resource_mesh = meshes.add(Sphere::new(0.5));

    for i in 0..5 {
        commands.spawn((
            Mesh3d(resource_mesh.clone()),
            MeshMaterial3d(resource_material.clone()),
            Transform::from_xyz(
                (i as f32 - 2.0) * 3.0,
                0.5,
                0.0,
            ),
            Resource { amount: 100 },
        ));
    }

    // UI - Resource counter
    commands.spawn((
        Text::new("Resources: 0"),
        TextFont {
            font_size: 30.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        ResourceUI,
    ));

    // UI - Selection info
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(50.0),
            left: Val::Px(10.0),
            ..default()
        },
        SelectionInfoUI,
    ));

    // UI - Controls
    commands.spawn((
        Text::new("Controls:\nLeft Click: Select\nRight Click: Move/Attack\nLeft Drag: Box Select\nQ: Spawn Worker\nW: Spawn Warrior\nArrows: Move Camera"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
    ));
}

fn spawn_unit(
    commands: &mut Commands,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    position: Vec3,
    team: Team,
    is_worker: bool,
) {
    let mut entity_commands = commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(position),
        Unit {
            max_health: 100.0,
            health: 100.0,
            speed: 3.0,
            attack_damage: 10.0,
            attack_range: 2.0,
            attack_cooldown: Timer::from_seconds(1.0, TimerMode::Repeating),
            team,
        },
        UnitMarker,
    ));

    if is_worker {
        entity_commands.insert(Gatherer {
            gathering_rate: 5.0,
            gathering_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
            current_resource: None,
            carrying: 0,
            capacity: 10,
            assigned_resource: None,
            gathering_enabled: false, // Workers don't auto-gather on spawn
        });
    }
}

// ==================== Camera System ====================

fn camera_movement(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
) {
    let mut transform = camera.single_mut();
    let speed = 10.0 * time.delta_secs();

    if keyboard.pressed(KeyCode::ArrowUp) {
        transform.translation.z -= speed;
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        transform.translation.z += speed;
    }
    if keyboard.pressed(KeyCode::ArrowLeft) {
        transform.translation.x -= speed;
    }
    if keyboard.pressed(KeyCode::ArrowRight) {
        transform.translation.x += speed;
    }
}

// ==================== Selection System ====================
// ==================== Pathfinding System ====================

fn find_path_around_obstacles(
    start: Vec3,
    goal: Vec3,
    buildings: &Query<&Transform, (With<MainBase>, Without<UnitMarker>)>,
    resources: &Query<&Transform, (With<Resource>, Without<UnitMarker>)>,
) -> Vec<Vec3> {
    let mut waypoints = Vec::new();

    // Check if direct path is clear
    if is_path_clear(start, goal, buildings, resources) {
        return waypoints; // Direct path is fine, no waypoints needed
    }

    // Find blocking obstacle
    if let Some(obstacle_pos) = find_blocking_obstacle(start, goal, buildings, resources) {
        // Generate two candidate waypoints: going around left and right
        // Use obstacle position on XZ plane only (flatten to ground level)
        let obstacle_pos_flat = Vec3::new(obstacle_pos.x, 0.0, obstacle_pos.z);
        let start_flat = Vec3::new(start.x, 0.0, start.z);
        let to_obstacle = (obstacle_pos_flat - start_flat).normalize();
        let perpendicular = Vec3::new(-to_obstacle.z, 0.0, to_obstacle.x);

        // Increased clearance to avoid collision zones
        // Buildings: 3.0 collision distance + safety margin = 6.0
        // Resources: 1.2 collision distance + safety margin = 2.5
        let clearance = 6.5; // Distance to go around obstacle
        let left_point = obstacle_pos_flat + perpendicular * clearance;
        let right_point = obstacle_pos_flat - perpendicular * clearance;

        // Choose the path where both segments are clear
        let left_clear = is_path_clear(start, left_point, buildings, resources)
            && is_path_clear(left_point, goal, buildings, resources);
        let right_clear = is_path_clear(start, right_point, buildings, resources)
            && is_path_clear(right_point, goal, buildings, resources);

        let waypoint = if left_clear && !right_clear {
            left_point
        } else if right_clear && !left_clear {
            right_point
        } else if left_clear && right_clear {
            // Both clear - choose shorter
            let left_distance = (left_point - start).length() + (goal - left_point).length();
            let right_distance = (right_point - start).length() + (goal - right_point).length();
            if left_distance < right_distance { left_point } else { right_point }
        } else {
            // Both blocked - try with even more clearance
            let extra_clearance = clearance * 1.5; // 50% more clearance
            let left_far = obstacle_pos_flat + perpendicular * extra_clearance;
            let right_far = obstacle_pos_flat - perpendicular * extra_clearance;

            let left_far_clear = is_path_clear(start, left_far, buildings, resources)
                && is_path_clear(left_far, goal, buildings, resources);
            let right_far_clear = is_path_clear(start, right_far, buildings, resources)
                && is_path_clear(right_far, goal, buildings, resources);

            if left_far_clear {
                left_far
            } else if right_far_clear {
                right_far
            } else {
                // Still blocked - just use shorter original path and let collision handle it
                let left_distance = (left_point - start).length() + (goal - left_point).length();
                let right_distance = (right_point - start).length() + (goal - right_point).length();
                if left_distance < right_distance { left_point } else { right_point }
            }
        };

        waypoints.push(waypoint);

        // If path from waypoint to goal is still blocked, recursively find more waypoints
        if !is_path_clear(waypoint, goal, buildings, resources) {
            let remaining_waypoints = find_path_around_obstacles(waypoint, goal, buildings, resources);
            waypoints.extend(remaining_waypoints);
        }
    }

    waypoints
}

fn is_path_clear(
    start: Vec3,
    goal: Vec3,
    buildings: &Query<&Transform, (With<MainBase>, Without<UnitMarker>)>,
    resources: &Query<&Transform, (With<Resource>, Without<UnitMarker>)>,
) -> bool {
    let direction = goal - start;
    let distance = direction.length();

    if distance < 0.1 {
        return true;
    }

    let dir_normalized = direction.normalize();

    // Check buildings
    for building in buildings.iter() {
        let to_building = building.translation - start;
        let projection = to_building.dot(dir_normalized);

        if projection > 0.0 && projection < distance {
            let closest_point = start + dir_normalized * projection;
            let dist_to_path = (building.translation - closest_point).length();

            if dist_to_path < 3.5 {
                return false; // Path blocked by building
            }
        }
    }

    // Check resources (smaller clearance)
    for resource in resources.iter() {
        let to_resource = resource.translation - start;
        let projection = to_resource.dot(dir_normalized);

        if projection > 0.0 && projection < distance {
            let closest_point = start + dir_normalized * projection;
            let dist_to_path = (resource.translation - closest_point).length();

            if dist_to_path < 1.5 {
                return false; // Path blocked by resource
            }
        }
    }

    true
}

fn find_blocking_obstacle(
    start: Vec3,
    goal: Vec3,
    buildings: &Query<&Transform, (With<MainBase>, Without<UnitMarker>)>,
    resources: &Query<&Transform, (With<Resource>, Without<UnitMarker>)>,
) -> Option<Vec3> {
    let direction = goal - start;
    let distance = direction.length();
    let dir_normalized = direction.normalize();

    let mut closest_obstacle: Option<(Vec3, f32)> = None;

    // Check buildings first (higher priority)
    for building in buildings.iter() {
        let to_building = building.translation - start;
        let projection = to_building.dot(dir_normalized);

        if projection > 0.0 && projection < distance {
            let closest_point = start + dir_normalized * projection;
            let dist_to_path = (building.translation - closest_point).length();

            if dist_to_path < 3.5 {
                if closest_obstacle.is_none() || projection < closest_obstacle.unwrap().1 {
                    closest_obstacle = Some((building.translation, projection));
                }
            }
        }
    }

    // Check resources
    for resource in resources.iter() {
        let to_resource = resource.translation - start;
        let projection = to_resource.dot(dir_normalized);

        if projection > 0.0 && projection < distance {
            let closest_point = start + dir_normalized * projection;
            let dist_to_path = (resource.translation - closest_point).length();

            if dist_to_path < 1.5 {
                if closest_obstacle.is_none() || projection < closest_obstacle.unwrap().1 {
                    closest_obstacle = Some((resource.translation, projection));
                }
            }
        }
    }

    closest_obstacle.map(|(pos, _)| pos)
}

fn mouse_button_input(
    mouse_button: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut selection_box: ResMut<SelectionBox>,
    mut commands: Commands,
    units_query: Query<(Entity, &Transform, &Unit), With<UnitMarker>>,
    selected_query: Query<Entity, With<Selected>>,
    mut gatherers_query: Query<&mut Gatherer>,
    resources_query: Query<(Entity, &Transform), With<Resource>>,
    buildings_query: Query<&Transform, (With<MainBase>, Without<UnitMarker>)>,
    resources_transform_query: Query<&Transform, (With<Resource>, Without<UnitMarker>)>,
) {
    let window = windows.single();
    let (camera, camera_transform) = camera_query.single();

    if let Some(cursor_pos) = window.cursor_position() {
        // Left click - Start selection
        if mouse_button.just_pressed(MouseButton::Left) {
            selection_box.start = Some(cursor_pos);
            selection_box.end = None;
        }

        // Update selection box while dragging
        if mouse_button.pressed(MouseButton::Left) && selection_box.start.is_some() {
            selection_box.end = Some(cursor_pos);
        }

        // Left release - Complete selection
        if mouse_button.just_released(MouseButton::Left) {
            if let Some(start) = selection_box.start {
                let end = cursor_pos;

                // Clear previous selection
                for entity in selected_query.iter() {
                    commands.entity(entity).remove::<Selected>();
                }

                // Check if it's a click or drag
                let is_drag = start.distance(end) > 5.0;

                if is_drag {
                    // Box selection
                    let min_x = start.x.min(end.x);
                    let max_x = start.x.max(end.x);
                    let min_y = start.y.min(end.y);
                    let max_y = start.y.max(end.y);

                    for (entity, transform, unit) in units_query.iter() {
                        if unit.team != Team::Player {
                            continue;
                        }

                        // Project 3D position to screen space
                        if let Ok(screen_pos) = camera.world_to_viewport(camera_transform, transform.translation) {
                            if screen_pos.x >= min_x
                                && screen_pos.x <= max_x
                                && screen_pos.y >= min_y
                                && screen_pos.y <= max_y
                            {
                                commands.entity(entity).insert(Selected);
                            }
                        }
                    }
                } else {
                    // Single selection via raycast
                    if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) {
                        let mut closest_unit: Option<(Entity, f32)> = None;

                        for (entity, transform, unit) in units_query.iter() {
                            if unit.team != Team::Player {
                                continue;
                            }

                            let distance = ray_sphere_intersection(
                                ray.origin,
                                *ray.direction,
                                transform.translation,
                                0.5,
                            );

                            if let Some(dist) = distance {
                                if closest_unit.is_none() || dist < closest_unit.unwrap().1 {
                                    closest_unit = Some((entity, dist));
                                }
                            }
                        }

                        if let Some((entity, _)) = closest_unit {
                            commands.entity(entity).insert(Selected);
                        }
                    }
                }

                selection_box.start = None;
                selection_box.end = None;
            }
        }

        // Right click - Command selected units
        if mouse_button.just_pressed(MouseButton::Right) {
            if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) {
                // Find intersection with ground plane (y = 0)
                if let Some(ground_point) = ray_plane_intersection(ray.origin, *ray.direction) {
                    // Check if clicking on resource node
                    let mut target_resource: Option<Entity> = None;
                    for (entity, transform) in resources_query.iter() {
                        let distance = ray_sphere_intersection(
                            ray.origin,
                            *ray.direction,
                            transform.translation,
                            0.5,
                        );
                        if distance.is_some() {
                            target_resource = Some(entity);
                            break;
                        }
                    }

                    // Check if clicking on enemy unit
                    let mut target_enemy: Option<Entity> = None;
                    if target_resource.is_none() {
                        for (entity, transform, unit) in units_query.iter() {
                            if unit.team == Team::Enemy {
                                let distance = ray_sphere_intersection(
                                    ray.origin,
                                    *ray.direction,
                                    transform.translation,
                                    0.5,
                                );
                                if distance.is_some() {
                                    target_enemy = Some(entity);
                                    break;
                                }
                            }
                        }
                    }

                    // Command selected units
                    for entity in selected_query.iter() {
                        commands.entity(entity).remove::<MoveTo>();
                        commands.entity(entity).remove::<Waypoints>();
                        commands.entity(entity).remove::<AttackTarget>();

                        // Get unit transform for obstacle avoidance
                        let unit_transform = units_query
                            .iter()
                            .find(|(e, _, _)| *e == entity)
                            .map(|(_, t, _)| t);

                        if let Some(resource) = target_resource {
                            // Assign worker to gather from this specific resource
                            if let Ok(mut gatherer) = gatherers_query.get_mut(entity) {
                                gatherer.assigned_resource = Some(resource);
                                gatherer.current_resource = None;
                                gatherer.carrying = 0;
                                gatherer.gathering_enabled = true; // Enable gathering when assigned to resource
                            }
                        } else if let Some(enemy) = target_enemy {
                            // Attack command - clear gathering assignments
                            if let Ok(mut gatherer) = gatherers_query.get_mut(entity) {
                                gatherer.assigned_resource = None;
                                gatherer.current_resource = None;
                                gatherer.carrying = 0;
                                gatherer.gathering_enabled = false; // Disable gathering when attacking
                            }
                            commands.entity(entity).insert(AttackTarget { target: enemy });
                        } else {
                            // Manual move command - clear gathering assignments
                            if let Ok(mut gatherer) = gatherers_query.get_mut(entity) {
                                gatherer.assigned_resource = None;
                                gatherer.current_resource = None;
                                gatherer.carrying = 0;
                                gatherer.gathering_enabled = false; // Disable gathering when manually moving
                            }
                            // Calculate path with waypoints if needed
                            if let Some(unit_transform) = unit_transform {
                                let waypoints = find_path_around_obstacles(
                                    unit_transform.translation,
                                    ground_point,
                                    &buildings_query,
                                    &resources_transform_query,
                                );

                                if !waypoints.is_empty() {
                                    commands.entity(entity).insert(Waypoints {
                                        points: waypoints,
                                        current_index: 0,
                                    });
                                }
                                commands.entity(entity).insert(MoveTo { target: ground_point });
                            } else {
                                // Fallback if we can't find the unit (shouldn't happen)
                                commands.entity(entity).insert(MoveTo { target: ground_point });
                            }
                        }
                    }
                }
            }
        }
    }
}

fn ray_plane_intersection(ray_origin: Vec3, ray_dir: Vec3) -> Option<Vec3> {
    let plane_normal = Vec3::Y;
    let plane_point = Vec3::ZERO;

    let denom = plane_normal.dot(ray_dir);
    if denom.abs() > 1e-6 {
        let t = (plane_point - ray_origin).dot(plane_normal) / denom;
        if t >= 0.0 {
            return Some(ray_origin + ray_dir * t);
        }
    }
    None
}

fn ray_sphere_intersection(ray_origin: Vec3, ray_dir: Vec3, sphere_center: Vec3, sphere_radius: f32) -> Option<f32> {
    let oc = ray_origin - sphere_center;
    let a = ray_dir.dot(ray_dir);
    let b = 2.0 * oc.dot(ray_dir);
    let c = oc.dot(oc) - sphere_radius * sphere_radius;
    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        None
    } else {
        let t = (-b - discriminant.sqrt()) / (2.0 * a);
        if t >= 0.0 {
            Some(t)
        } else {
            None
        }
    }
}

fn draw_selection_box(
    _selection_box: Res<SelectionBox>,
) {
    // Selection box drawing would require UI overlay rendering
    // Simplified for this POC
}

// Helper function to create a ring mesh (circle outline on the ground)
fn create_ring_mesh(radius: f32, thickness: f32, segments: u32) -> Mesh {
    use bevy::render::mesh::{Indices, PrimitiveTopology};
    use bevy::render::render_asset::RenderAssetUsages;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    // Create vertices for a ring (circle outline) on the XZ plane
    for i in 0..segments {
        let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let cos = angle.cos();
        let sin = angle.sin();

        // Outer edge
        positions.push([cos * (radius + thickness / 2.0), 0.0, sin * (radius + thickness / 2.0)]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([1.0, i as f32 / segments as f32]);

        // Inner edge
        positions.push([cos * (radius - thickness / 2.0), 0.0, sin * (radius - thickness / 2.0)]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([0.0, i as f32 / segments as f32]);
    }

    // Create triangles
    for i in 0..segments {
        let next = (i + 1) % segments;

        let outer_current = i * 2;
        let inner_current = i * 2 + 1;
        let outer_next = next * 2;
        let inner_next = next * 2 + 1;

        // First triangle
        indices.push(outer_current);
        indices.push(inner_current);
        indices.push(outer_next);

        // Second triangle
        indices.push(inner_current);
        indices.push(inner_next);
        indices.push(outer_next);
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

fn update_selection_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    selected_query: Query<(Entity, &Transform), (With<Selected>, With<UnitMarker>)>,
    unselected_query: Query<Entity, (Without<Selected>, With<UnitMarker>)>,
    mut ring_query: Query<(Entity, &SelectionRing, &mut Transform), Without<UnitMarker>>,
    unit_transforms: Query<&Transform, With<UnitMarker>>,
) {
    // Add selection rings for selected units
    for (entity, unit_transform) in selected_query.iter() {
        let has_ring = ring_query.iter().any(|(_, ring, _)| ring.owner == entity);

        if !has_ring {
            // Create a ring mesh: radius of 0.7, thickness of 0.08, 64 segments for smooth circle
            let ring_mesh = create_ring_mesh(0.7, 0.08, 64);

            commands.spawn((
                Mesh3d(meshes.add(ring_mesh)),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.0, 1.0, 0.0),
                    emissive: Color::srgb(0.0, 2.5, 0.0).into(), // Bright glow
                    ..default()
                })),
                Transform::from_xyz(unit_transform.translation.x, 0.02, unit_transform.translation.z),
                SelectionRing { owner: entity },
            ));
        }
    }

    // Update ring positions to follow units
    for (_, ring, mut ring_transform) in ring_query.iter_mut() {
        if let Ok(unit_transform) = unit_transforms.get(ring.owner) {
            ring_transform.translation.x = unit_transform.translation.x;
            ring_transform.translation.z = unit_transform.translation.z;
            ring_transform.translation.y = 0.02; // Just above ground
        }
    }

    // Remove rings for unselected units
    let mut to_despawn = Vec::new();
    for (ring_entity, ring, _) in ring_query.iter() {
        if unselected_query.get(ring.owner).is_ok() {
            to_despawn.push(ring_entity);
        }
    }
    for entity in to_despawn {
        commands.entity(entity).despawn();
    }
}

fn update_health_bars(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    units_query: Query<(Entity, &Unit, &Transform, Has<Selected>), With<UnitMarker>>,
    health_bar_query: Query<(Entity, &HealthBar)>,
    health_bar_bg_query: Query<(Entity, &HealthBarBackground)>,
    mut health_bar_transforms: Query<&mut Transform, Without<UnitMarker>>,
    mut material_handles: Query<&mut MeshMaterial3d<StandardMaterial>>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
) {
    // Add health bars to units that don't have them
    for (entity, unit, _unit_transform, is_selected) in units_query.iter() {
        let has_health_bar = health_bar_query.iter().any(|(_, health_bar)| health_bar.owner == entity);

        if !has_health_bar {
            // Determine team colors based on team and selection
            let (bg_color, fg_color) = match unit.team {
                Team::Player => {
                    if is_selected {
                        // Selected player unit: Green
                        (
                            Color::srgb(0.0, 0.3, 0.0),  // Dark green background
                            Color::srgb(0.0, 1.0, 0.0),  // Bright green foreground
                        )
                    } else {
                        // Unselected player unit: Blue
                        (
                            Color::srgb(0.0, 0.0, 0.3),  // Dark blue background
                            Color::srgb(0.0, 0.5, 1.0),  // Bright blue foreground
                        )
                    }
                }
                Team::Enemy => {
                    // Enemy unit: Red
                    (
                        Color::srgb(0.3, 0.0, 0.0),  // Dark red background
                        Color::srgb(1.0, 0.0, 0.0),  // Bright red foreground
                    )
                }
            };

            // Background bar (darker)
            let bg_emissive = if let Color::Srgba(srgba) = bg_color {
                Color::srgb(srgba.red * 1.5, srgba.green * 1.5, srgba.blue * 1.5)
            } else {
                bg_color
            };

            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(1.0, 0.15, 0.08))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: bg_color,
                    emissive: bg_emissive.into(),
                    ..default()
                })),
                Transform::from_xyz(0.0, 1.3, 0.0),
                HealthBarBackground { owner: entity },
            ));

            // Foreground bar (brighter)
            let fg_emissive = if let Color::Srgba(srgba) = fg_color {
                Color::srgb(srgba.red * 1.5, srgba.green * 1.5, srgba.blue * 1.5)
            } else {
                fg_color
            };

            commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(1.0, 0.18, 0.1))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: fg_color,
                    emissive: fg_emissive.into(),
                    ..default()
                })),
                Transform::from_xyz(0.0, 1.3, 0.0),
                HealthBar { owner: entity },
            ));
        }
    }

    // Update health bar positions, rotations, scales and colors
    // Get camera transform for billboarding and calculate single rotation for all bars
    let camera_transform = camera_query.get_single().ok();

    // Calculate screen-aligned rotation once based on camera's forward direction
    let screen_aligned_rotation = if let Some(cam_transform) = camera_transform {
        // Get camera's forward direction
        let camera_forward = cam_transform.forward();

        // Calculate rotation to face camera (project to Y plane for upright bars)
        // All bars use the same rotation, making them parallel to the screen plane
        Some(Quat::from_rotation_y((-camera_forward.x).atan2(-camera_forward.z)))
    } else {
        None
    };

    for (unit_entity, unit, unit_transform, is_selected) in units_query.iter() {
        // Update background health bar position and rotation
        for (bg_bar_entity, bg_bar) in health_bar_bg_query.iter() {
            if bg_bar.owner == unit_entity {
                if let Ok(mut bg_transform) = health_bar_transforms.get_mut(bg_bar_entity) {
                    // Update position to follow owner unit
                    bg_transform.translation = unit_transform.translation + Vec3::new(0.0, 1.3, 0.0);

                    // Apply screen-aligned rotation (same for all bars)
                    if let Some(rotation) = screen_aligned_rotation {
                        bg_transform.rotation = rotation;
                    }
                }
            }
        }

        // Update foreground health bar position, rotation, scale, and color
        for (health_bar_entity, health_bar) in health_bar_query.iter() {
            if health_bar.owner == unit_entity {
                if let Ok(mut fg_transform) = health_bar_transforms.get_mut(health_bar_entity) {
                    // Update position to follow owner unit
                    let health_percent = (unit.health / unit.max_health).max(0.0).min(1.0);
                    fg_transform.translation = unit_transform.translation + Vec3::new(0.0, 1.3, 0.0);

                    // Apply screen-aligned rotation (same for all bars)
                    if let Some(rotation) = screen_aligned_rotation {
                        fg_transform.rotation = rotation;
                    }

                    // Update scale based on health
                    fg_transform.scale.x = health_percent;

                    // Offset to align left (needs to be applied in local space after rotation)
                    let offset = (1.0 - health_percent) * -0.5;
                    fg_transform.translation.x += offset;
                }

                // Update color based on health and team
                if let Ok(material_handle) = material_handles.get_mut(health_bar_entity) {
                    let health_percent = (unit.health / unit.max_health).max(0.0).min(1.0);

                    // Get base color based on team and selection
                    let base_color = match unit.team {
                        Team::Player => {
                            if is_selected {
                                Color::srgb(0.0, 1.0, 0.0)  // Green for selected
                            } else {
                                Color::srgb(0.0, 0.5, 1.0)  // Blue for unselected
                            }
                        }
                        Team::Enemy => {
                            Color::srgb(1.0, 0.0, 0.0)  // Red for enemy
                        }
                    };

                    // Darken color based on missing health (more damage = darker)
                    let color_multiplier = 0.3 + (health_percent * 0.7);  // Range: 0.3 to 1.0

                    let (final_color, final_emissive) = if let Color::Srgba(srgba) = base_color {
                        let final_r = srgba.red * color_multiplier;
                        let final_g = srgba.green * color_multiplier;
                        let final_b = srgba.blue * color_multiplier;
                        (
                            Color::srgb(final_r, final_g, final_b),
                            Color::srgb(final_r * 1.5, final_g * 1.5, final_b * 1.5)
                        )
                    } else {
                        (base_color, base_color)
                    };

                    // Update material
                    if let Some(material) = materials.get_mut(material_handle.0.id()) {
                        material.base_color = final_color;
                        material.emissive = final_emissive.into();
                    }
                }
            }
        }
    }

    // Cleanup health bars for despawned units
    let mut to_despawn = Vec::new();

    // Cleanup foreground health bars
    for (health_bar_entity, health_bar) in health_bar_query.iter() {
        if units_query.get(health_bar.owner).is_err() {
            to_despawn.push(health_bar_entity);
        }
    }

    // Cleanup background health bars
    for (health_bar_bg_entity, health_bar_bg) in health_bar_bg_query.iter() {
        if units_query.get(health_bar_bg.owner).is_err() {
            to_despawn.push(health_bar_bg_entity);
        }
    }

    for entity in to_despawn {
        commands.entity(entity).despawn();
    }
}

// ==================== Movement System ====================

fn move_units(
    time: Res<Time>,
    mut commands: Commands,
    mut units: Query<(Entity, &mut Transform, &Unit, &MoveTo, Option<&mut Waypoints>)>,
) {
    for (entity, mut transform, unit, move_to, mut waypoints_opt) in units.iter_mut() {
        let mut continue_moving = true;

        while continue_moving {
            // Check if we have waypoints and if the current index is valid
            let (has_more_waypoints, target) = if let Some(ref waypoints) = waypoints_opt {
                if waypoints.current_index < waypoints.points.len() {
                    (true, waypoints.points[waypoints.current_index])
                } else {
                    // All waypoints exhausted, use final target
                    (false, move_to.target)
                }
            } else {
                // No waypoints component, use final target
                (false, move_to.target)
            };

            // Calculate distance on XZ plane only (ignore Y difference)
            let direction_xz = Vec3::new(target.x - transform.translation.x, 0.0, target.z - transform.translation.z);
            let distance = direction_xz.length();
            let direction = if distance > 0.01 { direction_xz.normalize() } else { Vec3::ZERO };

            if distance > 0.1 {
                let move_delta = direction * unit.speed * time.delta_secs();

                if move_delta.length() >= distance {
                    // Reached current waypoint or target
                    transform.translation.x = target.x;
                    transform.translation.z = target.z;
                    transform.translation.y = 0.5;

                    // If we just reached a waypoint, advance to the next one
                    if has_more_waypoints {
                        if let Some(waypoints) = waypoints_opt.as_mut() {
                            waypoints.current_index += 1;
                            // Check if there are more waypoints after incrementing
                            if waypoints.current_index >= waypoints.points.len() {
                                // No more waypoints, drop the option and continue to final target
                                commands.entity(entity).remove::<Waypoints>();
                                waypoints_opt = None;
                            }
                        }
                        continue_moving = true; // Continue to next waypoint or final target
                    } else {
                        // Reached the final target (no more waypoints)
                        continue_moving = false;
                    }
                } else {
                    // Still moving, don't continue the loop this frame
                    transform.translation += move_delta;
                    transform.translation.y = 0.5;

                    // Rotate to face movement direction
                    if direction.length() > 0.01 {
                        let target_rotation = Quat::from_rotation_y(-direction.x.atan2(direction.z));
                        transform.rotation = target_rotation;
                    }
                    continue_moving = false; // Exit loop - we're moving but haven't reached target yet
                }
            } else {
                // Within threshold distance - consider reached
                // If we have more waypoints, advance to next one
                if has_more_waypoints {
                    if let Some(waypoints) = waypoints_opt.as_mut() {
                        waypoints.current_index += 1;
                        // Check if there are more waypoints after incrementing
                        if waypoints.current_index >= waypoints.points.len() {
                            // No more waypoints, drop the option and continue to final target
                            commands.entity(entity).remove::<Waypoints>();
                            waypoints_opt = None;
                        }
                    }
                    continue_moving = true; // Continue to next waypoint or final target
                } else {
                    // Reached final target, stop moving
                    commands.entity(entity).remove::<MoveTo>();
                    commands.entity(entity).remove::<Waypoints>();
                    continue_moving = false;
                }
            }
        }
    }
}

// ==================== Collision System ====================

fn resolve_collisions(
    mut commands: Commands,
    mut units: Query<(Entity, &mut Transform, Option<&MoveTo>, Option<&Waypoints>), With<UnitMarker>>,
    resources: Query<&Transform, (With<Resource>, Without<UnitMarker>)>,
    buildings: Query<&Transform, (With<MainBase>, Without<UnitMarker>)>,
) {
    // Store transforms, move targets, and waypoint status to avoid borrow checker issues
    let mut positions: Vec<(Entity, Vec3, Option<Vec3>, bool)> = units
        .iter()
        .map(|(e, t, move_to, waypoints)| (e, t.translation, move_to.map(|m| m.target), waypoints.is_some()))
        .collect();

    // Unit-unit collisions
    for i in 0..positions.len() {
        for j in (i+1)..positions.len() {
            let (_entity_a, pos_a, _, _) = positions[i];
            let (_entity_b, pos_b, _, _) = positions[j];

            let delta = pos_a - pos_b;
            let distance = delta.length();
            let min_distance = 0.8; // Combined radius (0.4 + 0.4)

            if distance < min_distance && distance > 0.01 {
                let push = delta.normalize() * (min_distance - distance) * 0.5;
                positions[i].1 += push;
                positions[j].1 -= push;
            }
        }
    }

    // Unit-resource collisions (smaller distance for small gold nodes)
    for (entity, mut pos, mut move_target, has_waypoints) in positions.iter_mut() {
        for resource_transform in resources.iter() {
            // Calculate distance on XZ plane only
            let delta_xz = Vec2::new(
                pos.x - resource_transform.translation.x,
                pos.z - resource_transform.translation.z
            );
            let distance = delta_xz.length();
            // Resource sphere radius is 0.5, unit radius is ~0.4, plus buffer = 1.2
            let min_distance = 1.2;

            if distance < min_distance && distance > 0.01 {
                let push_dir = delta_xz.normalize();
                // Double the push force to make collisions more effective
                let push_amount = (min_distance - distance) * 2.0;
                pos.x += push_dir.x * push_amount;
                pos.z += push_dir.y * push_amount; // Vec2.y is Z in 3D space

                // Only remove MoveTo if unit is moving TOWARD the obstacle AND not following waypoints
                if let Some(target) = move_target.as_ref() {
                    let to_target = Vec2::new(
                        target.x - pos.x,
                        target.z - pos.z
                    );
                    let to_obstacle = Vec2::new(
                        resource_transform.translation.x - pos.x,
                        resource_transform.translation.z - pos.z
                    );

                    // Normalize vectors and compute dot product
                    // Dot product > 0 means moving toward obstacle
                    let to_target_len = to_target.length();
                    let to_obstacle_len = to_obstacle.length();

                    if to_target_len > 0.01 && to_obstacle_len > 0.01 {
                        let dot = to_target.normalize().dot(to_obstacle.normalize());

                        // Only remove MoveTo if moving toward obstacle AND very close AND not following waypoints
                        if dot > 0.3 && distance < min_distance * 0.9 && !*has_waypoints {
                            commands.entity(*entity).remove::<MoveTo>();
                            move_target.take(); // Clear it in our local copy too
                        }
                    }
                }
            }
        }
    }

    // Unit-building collisions (larger distance for 3x3 buildings)
    for (entity, mut pos, mut move_target, has_waypoints) in positions.iter_mut() {
        for building_transform in buildings.iter() {
            // Calculate distance on XZ plane only
            let delta_xz = Vec2::new(
                pos.x - building_transform.translation.x,
                pos.z - building_transform.translation.z
            );
            let distance = delta_xz.length();
            // MainBase is 3x3 cuboid, so half-diagonal is ~2.12
            // Unit radius is ~0.4, plus buffer = 2.5-3.0
            let min_distance = 3.0;

            if distance < min_distance && distance > 0.01 {
                let push_dir = delta_xz.normalize();
                // Double the push force to make collisions more effective
                let push_amount = (min_distance - distance) * 2.0;
                pos.x += push_dir.x * push_amount;
                pos.z += push_dir.y * push_amount; // Vec2.y is Z in 3D space

                // Only remove MoveTo if unit is moving TOWARD the obstacle AND not following waypoints
                if let Some(target) = move_target.as_ref() {
                    let to_target = Vec2::new(
                        target.x - pos.x,
                        target.z - pos.z
                    );
                    let to_obstacle = Vec2::new(
                        building_transform.translation.x - pos.x,
                        building_transform.translation.z - pos.z
                    );

                    // Normalize vectors and compute dot product
                    // Dot product > 0 means moving toward obstacle
                    let to_target_len = to_target.length();
                    let to_obstacle_len = to_obstacle.length();

                    if to_target_len > 0.01 && to_obstacle_len > 0.01 {
                        let dot = to_target.normalize().dot(to_obstacle.normalize());

                        // Only remove MoveTo if moving toward obstacle AND very close AND not following waypoints
                        if dot > 0.3 && distance < min_distance * 0.9 && !*has_waypoints {
                            commands.entity(*entity).remove::<MoveTo>();
                            move_target.take(); // Clear it in our local copy too
                        }
                    }
                }
            }
        }
    }

    // Apply updated positions
    for (entity, new_pos, _, _) in positions {
        if let Ok((_, mut transform, _, _)) = units.get_mut(entity) {
            transform.translation.x = new_pos.x;
            transform.translation.z = new_pos.z;
            // Keep Y at 0.5
            transform.translation.y = 0.5;
        }
    }
}

// ==================== Combat System ====================

fn unit_attack(
    time: Res<Time>,
    mut commands: Commands,
    mut units: Query<(Entity, &Transform, &mut Unit), Without<AttackTarget>>,
    mut attackers: Query<(Entity, &Transform, &mut Unit, &AttackTarget)>,
) {
    for (attacker_entity, attacker_transform, mut attacker, attack_target) in attackers.iter_mut() {
        attacker.attack_cooldown.tick(time.delta());

        // Check if target still exists
        if let Ok((target_entity, target_transform, mut target)) = units.get_mut(attack_target.target) {
            let distance = attacker_transform.translation.distance(target_transform.translation);

            // Move towards target if out of range
            if distance > attacker.attack_range {
                commands.entity(attacker_entity).insert(MoveTo {
                    target: target_transform.translation,
                });
            } else {
                // In range - attack
                commands.entity(attacker_entity).remove::<MoveTo>();

                if attacker.attack_cooldown.just_finished() {
                    target.health -= attacker.attack_damage;

                    if target.health <= 0.0 {
                        commands.entity(target_entity).despawn();
                        commands.entity(attacker_entity).remove::<AttackTarget>();
                    }
                }
            }
        } else {
            // Target doesn't exist anymore
            commands.entity(attacker_entity).remove::<AttackTarget>();
        }
    }
}

// ==================== Resource Gathering System ====================

fn resource_gathering(
    time: Res<Time>,
    mut commands: Commands,
    mut game_resources: ResMut<GameResources>,
    mut gatherers: Query<(Entity, &Transform, &mut Gatherer, &Unit)>,
    mut resources: Query<(Entity, &Transform, &mut Resource)>,
    bases: Query<(&Transform, &MainBase)>,
) {
    for (gatherer_entity, gatherer_transform, mut gatherer, unit) in gatherers.iter_mut() {
        if unit.team != Team::Player {
            continue;
        }

        gatherer.gathering_timer.tick(time.delta());

        // If carrying resources, return to base
        if gatherer.carrying > 0 {
            if let Some((base_transform, _)) = bases.iter().find(|(_, base)| base.team == Team::Player) {
                let distance = gatherer_transform.translation.distance(base_transform.translation);

                if distance < 3.0 {
                    game_resources.player_resources += gatherer.carrying;
                    gatherer.carrying = 0;
                    gatherer.current_resource = None;
                } else {
                    commands.entity(gatherer_entity).insert(MoveTo {
                        target: base_transform.translation,
                    });
                }
            }
            continue;
        }

        // Find resource to gather from if not currently gathering
        if gatherer.current_resource.is_none() {
            // If we have a manually assigned resource, use it
            if let Some(assigned_entity) = gatherer.assigned_resource {
                // Check if assigned resource still exists and has resources
                if let Ok((_, resource_transform, resource)) = resources.get(assigned_entity) {
                    if resource.amount > 0 {
                        let distance = gatherer_transform.translation.distance(resource_transform.translation);
                        if distance < 1.5 {
                            gatherer.current_resource = Some(assigned_entity);
                        } else {
                            // Move towards the assigned resource
                            commands.entity(gatherer_entity).insert(MoveTo {
                                target: resource_transform.translation,
                            });
                        }
                    } else {
                        // Assigned resource is depleted, clear assignment
                        gatherer.assigned_resource = None;
                    }
                } else {
                    // Assigned resource doesn't exist anymore, clear assignment
                    gatherer.assigned_resource = None;
                }
            } else if gatherer.gathering_enabled {
                // Only auto-find nearest resource if gathering is enabled
                let mut nearest: Option<(Entity, Vec3, f32)> = None;

                for (resource_entity, resource_transform, resource) in resources.iter() {
                    if resource.amount <= 0 {
                        continue;
                    }

                    let distance = gatherer_transform.translation.distance(resource_transform.translation);
                    if nearest.is_none() || distance < nearest.as_ref().unwrap().2 {
                        nearest = Some((resource_entity, resource_transform.translation, distance));
                    }
                }

                if let Some((resource_entity, resource_position, distance)) = nearest {
                    if distance < 1.5 {
                        gatherer.current_resource = Some(resource_entity);
                    } else {
                        // Move towards the resource
                        commands.entity(gatherer_entity).insert(MoveTo {
                            target: resource_position,
                        });
                    }
                }
            }
        }

        // Gather from current resource
        if let Some(resource_entity) = gatherer.current_resource {
            if let Ok((_, resource_transform, mut resource)) = resources.get_mut(resource_entity) {
                let distance = gatherer_transform.translation.distance(resource_transform.translation);

                if distance > 1.5 {
                    commands.entity(gatherer_entity).insert(MoveTo {
                        target: resource_transform.translation,
                    });
                } else {
                    commands.entity(gatherer_entity).remove::<MoveTo>();

                    if gatherer.gathering_timer.just_finished() && gatherer.carrying < gatherer.capacity {
                        let gather_amount = gatherer.gathering_rate as i32;
                        let actual_amount = gather_amount.min(resource.amount).min(gatherer.capacity - gatherer.carrying);

                        resource.amount -= actual_amount;
                        gatherer.carrying += actual_amount;

                        if resource.amount <= 0 {
                            commands.entity(resource_entity).despawn();
                            gatherer.current_resource = None;
                        }

                        if gatherer.carrying >= gatherer.capacity {
                            gatherer.current_resource = None;
                        }
                    }
                }
            } else {
                gatherer.current_resource = None;
            }
        }
    }
}

// ==================== Unit Spawning ====================

fn spawn_unit_on_key(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut game_resources: ResMut<GameResources>,
    bases: Query<(&Transform, &MainBase)>,
) {
    let worker_cost = 25;
    let warrior_cost = 50;

    if keyboard.just_pressed(KeyCode::KeyQ) && game_resources.player_resources >= worker_cost {
        game_resources.player_resources -= worker_cost;

        if let Some((base_transform, _)) = bases.iter().find(|(_, base)| base.team == Team::Player) {
            let mut rng = rand::thread_rng();
            // Spawn in a ring around the base to avoid spawning inside the 3x3 building
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let distance = rng.gen_range(3.0..5.0); // 3-5 units from center
            let offset_x = angle.cos() * distance;
            let offset_z = angle.sin() * distance;
            let spawn_pos = base_transform.translation + Vec3::new(offset_x, 0.0, offset_z);
            spawn_unit(
                &mut commands,
                meshes.add(Capsule3d::new(0.3, 1.0)),
                materials.add(Color::srgb(0.3, 0.3, 1.0)),
                spawn_pos,
                Team::Player,
                true, // Q = Worker (can gather)
            );
        }
    }

    if keyboard.just_pressed(KeyCode::KeyW) && game_resources.player_resources >= warrior_cost {
        game_resources.player_resources -= warrior_cost;

        if let Some((base_transform, _)) = bases.iter().find(|(_, base)| base.team == Team::Player) {
            let mut rng = rand::thread_rng();
            // Spawn in a ring around the base to avoid spawning inside the 3x3 building
            let angle = rng.gen_range(0.0..std::f32::consts::TAU);
            let distance = rng.gen_range(3.0..5.0); // 3-5 units from center
            let offset_x = angle.cos() * distance;
            let offset_z = angle.sin() * distance;
            let spawn_pos = base_transform.translation + Vec3::new(offset_x, 0.0, offset_z);
            spawn_unit(
                &mut commands,
                meshes.add(Capsule3d::new(0.3, 1.0)),
                materials.add(Color::srgb(0.5, 0.3, 1.0)),
                spawn_pos,
                Team::Player,
                false, // W = Warrior (cannot gather)
            );
        }
    }
}

// ==================== UI System ====================

fn update_ui(
    game_resources: Res<GameResources>,
    mut resource_ui: Query<&mut Text, (With<ResourceUI>, Without<SelectionInfoUI>)>,
    mut selection_ui: Query<&mut Text, (With<SelectionInfoUI>, Without<ResourceUI>)>,
    selected_units: Query<&Unit, With<Selected>>,
) {
    // Update resource counter
    if let Ok(mut text) = resource_ui.get_single_mut() {
        text.0 = format!("Resources: {}", game_resources.player_resources);
    }

    // Update selection info
    if let Ok(mut text) = selection_ui.get_single_mut() {
        let count = selected_units.iter().count();
        if count > 0 {
            let total_health: f32 = selected_units.iter().map(|u| u.health).sum();
            text.0 = format!("Selected: {} units | Total HP: {:.0}", count, total_health);
        } else {
            text.0 = String::new();
        }
    }
}
