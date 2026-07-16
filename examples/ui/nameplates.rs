//! Nameplates: 2D name + health-bar labels that float above 3D characters, as seen
//! in many RPGs, RTSs and MMORPGs.
//!
//! A pack of foxes trots in a circle; each has a nameplate that tracks it, shrinks
//! with distance (down to a readable floor), fades out far away, and hides when its
//! fox passes behind the camera. The 3D scene and the UI are drawn by separate
//! cameras, so the nameplates always sit on top.
//!
//! The reusable pieces are `spawn_nameplate`, `place_nameplates`, `fade_descendants`
//! and the `Nameplate` component; the rest is scene setup.

use bevy::math::ops;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;

const HEAD_HEIGHT: f32 = 1.15;
const PLATE_WIDTH: f32 = 200.0;
const FADE_START: f32 = 12.0;
const FADE_END: f32 = 19.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        // Move and update the foxes, then place the nameplates over them.
        .add_systems(
            Update,
            (
                orbit_foxes,
                damage_and_heal,
                update_health_bars,
                place_nameplates,
            )
                .chain(),
        )
        .run();
}

#[derive(Component)]
struct Fox {
    angle: f32,
    radius: f32,
    speed: f32,
}

#[derive(Component)]
struct Health {
    current: f32,
    max: f32,
    trend: f32,
}

/// Nameplate root, pointing at the entity it labels (any entity with a `GlobalTransform`).
#[derive(Component)]
struct Nameplate {
    target: Entity,
}

#[derive(Component)]
struct HealthBarFill {
    target: Entity,
}

#[derive(Resource)]
struct FoxAnimation {
    index: AnimationNodeIndex,
    graph: Handle<AnimationGraph>,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    // Fox walk animation, started per fox by `start_fox_animation` once its scene loads.
    let (graph, index) = AnimationGraph::from_clip(
        asset_server.load(GltfAssetLabel::Animation(1).from_asset("models/animated/Fox.glb")),
    );
    commands.insert_resource(FoxAnimation {
        index,
        graph: graphs.add(graph),
    });
    let fox_scene =
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/animated/Fox.glb"));

    // The 3D camera draws the world; its projection also places the nameplates. A 2D
    // camera ordered above it draws the UI. A single camera works too — the second
    // just makes the ordering explicit; `ClearColorConfig::None` keeps the 3D visible.
    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::srgb(0.55, 0.63, 0.72)),
            ..default()
        },
        Transform::from_xyz(0.0, 3.0, 8.0).looking_at(Vec3::new(0.0, 0.7, 0.0), Vec3::Y),
    ));
    let ui_camera = commands
        .spawn((
            Camera2d,
            Camera {
                order: 1,
                clear_color: ClearColorConfig::None,
                ..default()
            },
        ))
        .id();

    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 10.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(200.0, 200.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.45, 0.32))),
    ));

    let names = ["Ariceli", "Bracken", "Cinder", "Dusk", "Ember", "Fennec"];
    for (i, name) in names.iter().enumerate() {
        let fox = commands
            .spawn((
                WorldAssetRoot(fox_scene.clone()),
                Transform::from_scale(Vec3::splat(0.01)),
                Fox {
                    angle: i as f32 / names.len() as f32 * std::f32::consts::TAU,
                    radius: 3.0 + i as f32 * 1.7,
                    speed: 0.32 - i as f32 * 0.028,
                },
                Health {
                    current: 100.0,
                    max: 100.0,
                    trend: if i % 2 == 0 { -1.0 } else { 1.0 },
                },
            ))
            .observe(start_fox_animation)
            .id();

        spawn_nameplate(&mut commands, ui_camera, fox, name);
    }
}

// Scene setup: start a fox's walk animation once its glTF scene has loaded.
fn start_fox_animation(
    ready: On<WorldInstanceReady>,
    animation: Res<FoxAnimation>,
    children: Query<&Children>,
    mut players: Query<&mut AnimationPlayer>,
    mut commands: Commands,
) {
    for child in children.iter_descendants(ready.entity) {
        if let Ok(mut player) = players.get_mut(child) {
            player.play(animation.index).repeat();
            commands
                .entity(child)
                .insert(AnimationGraphHandle(animation.graph.clone()));
        }
    }
}

// Walk the foxes around a circle, each facing its direction of travel.
fn orbit_foxes(time: Res<Time>, mut query: Query<(&mut Transform, &mut Fox)>) {
    for (mut transform, mut fox) in &mut query {
        fox.angle += fox.speed * time.delta_secs();
        let (sin, cos) = ops::sin_cos(fox.angle);
        transform.translation = Vec3::new(cos * fox.radius, 0.0, sin * fox.radius);
        transform.rotation = Quat::from_rotation_y(-fox.angle); // model faces +z
        transform.scale = Vec3::splat(0.01);
    }
}

// Drift each fox's health up and down so the bars visibly change.
fn damage_and_heal(time: Res<Time>, mut query: Query<&mut Health>) {
    for mut health in &mut query {
        health.current =
            (health.current + health.trend * 25.0 * time.delta_secs()).clamp(0.0, health.max);
        if health.current == 0.0 {
            health.trend = 1.0;
        } else if health.current == health.max {
            health.trend = -1.0;
        }
    }
}

fn update_health_bars(
    healths: Query<&Health>,
    mut fills: Query<(&mut Node, &mut BackgroundColor, &HealthBarFill)>,
) {
    for (mut node, mut color, fill) in &mut fills {
        if let Ok(health) = healths.get(fill.target) {
            let fraction = health.current / health.max;
            node.width = percent(fraction * 100.0);
            let green = Color::srgb(0.3, 0.85, 0.35);
            let red = Color::srgb(0.9, 0.3, 0.2);
            *color = BackgroundColor(green.mix(&red, 1.0 - fraction));
        }
    }
}

// Spawn a nameplate (name chip + health bar) that follows `target`. Reusable: works
// for any entity with a `GlobalTransform`, not just foxes.
fn spawn_nameplate(
    commands: &mut Commands,
    ui_camera: Entity,
    target: Entity,
    name: &str,
) -> Entity {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute, // moved in screen space each frame
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: px(3),
                width: px(PLATE_WIDTH),
                ..default()
            },
            UiTargetCamera(ui_camera),
            Nameplate { target },
            children![
                // Name on a dark chip so it reads over any background.
                (
                    Text::new(name),
                    TextFont {
                        font_size: FontSize::Px(26.0),
                        ..default()
                    },
                    TextColor(Color::WHITE),
                    TextLayout::justify(Justify::Center),
                    Node {
                        padding: UiRect::axes(px(8), px(2)),
                        border_radius: BorderRadius::all(px(4)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.07, 0.07, 0.09)),
                ),
                // Health bar: dark track with a colored fill. Drop it (and
                // `HealthBarFill` / `update_health_bars`) for a name-only plate.
                (
                    Node {
                        width: px(150),
                        height: px(10),
                        border: UiRect::all(px(1)),
                        ..default()
                    },
                    BorderColor::all(Color::BLACK),
                    BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
                    children![(
                        Node {
                            width: percent(100),
                            height: percent(100),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.3, 0.85, 0.35)),
                        HealthBarFill { target },
                    )],
                ),
            ],
        ))
        .id()
}

// Project each target to the screen, then position, scale and fade its nameplate.
fn place_nameplates(
    camera: Single<(&Camera, &GlobalTransform), With<Camera3d>>, // the world camera
    targets: Query<&GlobalTransform>,
    mut nameplates: Query<(
        Entity,
        &mut Node,
        &mut UiTransform,
        &mut Visibility,
        &Nameplate,
    )>,
    child_query: Query<&Children>,
    mut text_colors: Query<&mut TextColor>,
    mut bg_colors: Query<&mut BackgroundColor>,
) {
    let (camera, camera_transform) = *camera;

    for (root, mut node, mut ui_transform, mut visibility, nameplate) in &mut nameplates {
        let Ok(target) = targets.get(nameplate.target) else {
            continue;
        };
        let anchor = target.translation() + Vec3::Y * HEAD_HEIGHT;

        // Project the anchor: `x`/`y` are pixels, `z` is distance; `Err` = behind camera.
        let Ok(screen) = camera.world_to_viewport_with_depth(camera_transform, anchor) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        let depth = screen.z;

        let alpha = 1.0 - ((depth - FADE_START) / (FADE_END - FADE_START)).clamp(0.0, 1.0);
        if alpha <= 0.0 {
            *visibility = Visibility::Hidden;
            continue;
        }
        *visibility = Visibility::Visible;

        // Shrink with distance, clamped so distant plates stay readable.
        let scale = (6.0 / depth).clamp(0.5, 1.3);
        node.left = px(screen.x - PLATE_WIDTH * 0.5); // center over the target
        node.top = px(screen.y - 48.0 * scale);
        ui_transform.scale = Vec2::splat(scale);

        fade_descendants(root, alpha, &child_query, &mut text_colors, &mut bg_colors);
    }
}

// Apply `alpha` to every text and background color under `entity`, so the whole
// nameplate fades as one.
fn fade_descendants(
    entity: Entity,
    alpha: f32,
    child_query: &Query<&Children>,
    text_colors: &mut Query<&mut TextColor>,
    bg_colors: &mut Query<&mut BackgroundColor>,
) {
    let Ok(children) = child_query.get(entity) else {
        return;
    };
    for child in children.iter() {
        if let Ok(mut text_color) = text_colors.get_mut(child) {
            text_color.set_alpha(alpha);
        }
        if let Ok(mut bg) = bg_colors.get_mut(child) {
            bg.0.set_alpha(alpha);
        }
        fade_descendants(child, alpha, child_query, text_colors, bg_colors);
    }
}
