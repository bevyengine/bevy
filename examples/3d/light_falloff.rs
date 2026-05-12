//! Demonstrates the built-in light falloff modes for point and spot lights.

use std::f32::consts::PI;

use bevy::{
    color::palettes::css::{BLUE, SILVER, YELLOW},
    prelude::*,
};

const INSTRUCTIONS: &str = "Light Falloff";

const POINT_LIGHT_INTENSITY: f32 = 520_000.0;
const SPOT_LIGHT_INTENSITY: f32 = 760_000.0;
const LIGHT_RANGE: f32 = 21.0;
const RUNWAY_LENGTH: f32 = 20.0;
const INTENSITY_SCALE: f32 = 1.1;
const KEY_REPEAT_DELAY: f32 = 0.3;
const KEY_REPEAT_INTERVAL: f32 = 0.06;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.04, 0.04, 0.05)))
        .insert_resource(GlobalAmbientLight {
            brightness: 20.0,
            ..default()
        })
        .init_resource::<LightControls>()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (handle_keyboard_controls, sync_light_status_text))
        .run();
}

#[derive(Resource)]
struct LightControls {
    selected: SelectedLight,
}

impl Default for LightControls {
    fn default() -> Self {
        Self {
            selected: SelectedLight::Point,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SelectedLight {
    Point,
    Spot,
}

#[derive(Component)]
struct PointDemoLight;

#[derive(Component)]
struct SpotDemoLight;

#[derive(Component)]
struct SelectedLightLabel;

#[derive(Clone, Copy)]
enum UiAction {
    CycleFalloff,
    IntensityDown,
    IntensityUp,
}

#[derive(Component)]
struct LightStateLabel;

#[derive(Default)]
struct HeldKeyState {
    key: Option<KeyCode>,
    timer: Option<Timer>,
}

trait DemoLight {
    fn falloff(&self) -> LightFalloff;
    fn set_falloff(&mut self, falloff: LightFalloff);
    fn intensity(&self) -> f32;
    fn set_intensity(&mut self, intensity: f32);
}

impl DemoLight for PointLight {
    fn falloff(&self) -> LightFalloff {
        self.falloff
    }

    fn set_falloff(&mut self, falloff: LightFalloff) {
        self.falloff = falloff;
    }

    fn intensity(&self) -> f32 {
        self.intensity
    }

    fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity;
    }
}

impl DemoLight for SpotLight {
    fn falloff(&self) -> LightFalloff {
        self.falloff
    }

    fn set_falloff(&mut self, falloff: LightFalloff) {
        self.falloff = falloff;
    }

    fn intensity(&self) -> f32 {
        self.intensity
    }

    fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity;
    }
}

impl DemoLight for Mut<'_, PointLight> {
    fn falloff(&self) -> LightFalloff {
        self.falloff
    }

    fn set_falloff(&mut self, falloff: LightFalloff) {
        self.falloff = falloff;
    }

    fn intensity(&self) -> f32 {
        self.intensity
    }

    fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity;
    }
}

impl DemoLight for Mut<'_, SpotLight> {
    fn falloff(&self) -> LightFalloff {
        self.falloff
    }

    fn set_falloff(&mut self, falloff: LightFalloff) {
        self.falloff = falloff;
    }

    fn intensity(&self) -> f32 {
        self.intensity
    }

    fn set_intensity(&mut self, intensity: f32) {
        self.intensity = intensity;
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    controls: Res<LightControls>,
) {
    let runway_mesh = meshes.add(Cuboid::new(RUNWAY_LENGTH, 0.2, 3.0));
    let end_wall_mesh = meshes.add(Cuboid::new(0.2, 2.8, 3.0));
    let marker_mesh = meshes.add(Cuboid::new(0.3, 0.06, 1.9));
    let pillar_mesh = meshes.add(Cuboid::new(0.6, 1.4, 0.6));
    let sphere_mesh = meshes.add(Sphere::new(0.55).mesh().uv(32, 18));
    let runway_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.12, 0.12, 0.13),
        perceptual_roughness: 1.0,
        ..default()
    });
    let wall_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.82, 0.82, 0.84),
        perceptual_roughness: 0.92,
        ..default()
    });
    let marker_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.43, 0.48),
        perceptual_roughness: 0.8,
        ..default()
    });
    let pillar_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.48, 0.50, 0.56),
        perceptual_roughness: 0.72,
        ..default()
    });
    let sphere_material = materials.add(StandardMaterial {
        base_color: SILVER.into(),
        metallic: 0.05,
        perceptual_roughness: 0.25,
        ..default()
    });
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 8.8, 20.0).looking_at(Vec3::new(0.0, 1.4, 0.0), Vec3::Y),
    ));

    spawn_runway(
        &mut commands,
        &runway_mesh,
        &end_wall_mesh,
        &marker_mesh,
        &pillar_mesh,
        &sphere_mesh,
        &runway_material,
        &wall_material,
        &marker_material,
        &pillar_material,
        &sphere_material,
        Vec3::new(0.0, 0.0, -4.2),
    );
    spawn_runway(
        &mut commands,
        &runway_mesh,
        &end_wall_mesh,
        &marker_mesh,
        &pillar_mesh,
        &sphere_mesh,
        &runway_material,
        &wall_material,
        &marker_material,
        &pillar_material,
        &sphere_material,
        Vec3::new(0.0, 0.0, 4.2),
    );

    commands.spawn((
        PointLight {
            color: YELLOW.into(),
            intensity: POINT_LIGHT_INTENSITY,
            range: LIGHT_RANGE,
            falloff: LightFalloff::InverseSquare,
            shadow_maps_enabled: true,
            ..default()
        },
        PointDemoLight,
        Transform::from_xyz(-9.2, 1.6, -4.2),
    ));

    commands.spawn((
        SpotLight {
            color: BLUE.into(),
            intensity: SPOT_LIGHT_INTENSITY,
            range: LIGHT_RANGE,
            falloff: LightFalloff::InverseSquare,
            inner_angle: PI / 14.0,
            outer_angle: PI / 10.0,
            shadow_maps_enabled: true,
            ..default()
        },
        SpotDemoLight,
        Transform::from_xyz(-9.2, 1.8, 5.1).looking_at(Vec3::new(9.2, 0.35, 3.8), Vec3::Y),
    ));

    commands.spawn((
        Text2d::new("Point light"),
        TextFont {
            font_size: FontSize::Px(28.0),
            ..default()
        },
        TextColor(YELLOW.into()),
        Transform::from_xyz(-11.5, 2.8, -4.2),
    ));
    commands.spawn((
        Text2d::new("Spot light"),
        TextFont {
            font_size: FontSize::Px(28.0),
            ..default()
        },
        TextColor(BLUE.into()),
        Transform::from_xyz(-11.0, 2.8, 4.2),
    ));

    commands.spawn((
        Text::new(INSTRUCTIONS),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));

    spawn_status_ui(&mut commands, controls.selected);
}

fn spawn_runway(
    commands: &mut Commands,
    runway_mesh: &Handle<Mesh>,
    end_wall_mesh: &Handle<Mesh>,
    marker_mesh: &Handle<Mesh>,
    pillar_mesh: &Handle<Mesh>,
    sphere_mesh: &Handle<Mesh>,
    runway_material: &Handle<StandardMaterial>,
    wall_material: &Handle<StandardMaterial>,
    marker_material: &Handle<StandardMaterial>,
    pillar_material: &Handle<StandardMaterial>,
    sphere_material: &Handle<StandardMaterial>,
    origin: Vec3,
) {
    commands.spawn((
        Mesh3d(runway_mesh.clone()),
        MeshMaterial3d(runway_material.clone()),
        Transform::from_translation(origin),
    ));

    commands.spawn((
        Mesh3d(end_wall_mesh.clone()),
        MeshMaterial3d(wall_material.clone()),
        Transform::from_translation(origin + Vec3::new(10.1, 1.4, 0.0)),
    ));

    for marker_x in [-6.0, -2.0, 2.0, 6.0] {
        commands.spawn((
            Mesh3d(marker_mesh.clone()),
            MeshMaterial3d(marker_material.clone()),
            Transform::from_translation(origin + Vec3::new(marker_x, 0.14, 0.0)),
        ));
    }

    commands.spawn((
        Mesh3d(pillar_mesh.clone()),
        MeshMaterial3d(pillar_material.clone()),
        Transform::from_translation(origin + Vec3::new(2.8, 0.7, -0.8)),
    ));

    commands.spawn((
        Mesh3d(sphere_mesh.clone()),
        MeshMaterial3d(sphere_material.clone()),
        Transform::from_translation(origin + Vec3::new(6.0, 0.8, 0.6)),
    ));
}

fn spawn_status_ui(commands: &mut Commands, selected: SelectedLight) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: px(16),
                top: px(16),
                width: px(320),
                padding: UiRect::all(px(14)),
                flex_direction: FlexDirection::Column,
                row_gap: px(10),
                border_radius: BorderRadius::all(px(12)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.07, 0.07, 0.09, 0.88)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Selected"),
                TextFont {
                    font_size: FontSize::Px(24.0),
                    ..default()
                },
            ));
            parent.spawn((
                Text::new(match selected {
                    SelectedLight::Point => "Point light",
                    SelectedLight::Spot => "Spot light",
                }),
                SelectedLightLabel,
            ));
            parent.spawn((Text::new(""), LightStateLabel));
        });
}

fn handle_keyboard_controls(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut held_key_state: Local<HeldKeyState>,
    mut controls: ResMut<LightControls>,
    mut point_light: Single<&mut PointLight, With<PointDemoLight>>,
    mut spot_light: Single<&mut SpotLight, With<SpotDemoLight>>,
) {
    if input.just_pressed(KeyCode::Tab) {
        controls.selected = match controls.selected {
            SelectedLight::Point => SelectedLight::Spot,
            SelectedLight::Spot => SelectedLight::Point,
        };
    }

    let mut action = None;

    if input.just_pressed(KeyCode::KeyF) {
        action = Some(UiAction::CycleFalloff);
        start_key_repeat(&mut held_key_state, KeyCode::KeyF);
    } else if input.just_pressed(KeyCode::Minus) {
        action = Some(UiAction::IntensityDown);
        start_key_repeat(&mut held_key_state, KeyCode::Minus);
    } else if input.just_pressed(KeyCode::Equal) {
        action = Some(UiAction::IntensityUp);
        start_key_repeat(&mut held_key_state, KeyCode::Equal);
    } else if let (Some(key), Some(timer)) = (held_key_state.key, held_key_state.timer.as_mut()) {
        if input.pressed(key) {
            timer.tick(time.delta());
            if timer.just_finished() {
                action = match key {
                    KeyCode::KeyF => Some(UiAction::CycleFalloff),
                    KeyCode::Minus => Some(UiAction::IntensityDown),
                    KeyCode::Equal => Some(UiAction::IntensityUp),
                    _ => None,
                };
                timer.set_duration(core::time::Duration::from_secs_f32(KEY_REPEAT_INTERVAL));
                timer.reset();
            }
        } else {
            held_key_state.key = None;
            held_key_state.timer = None;
        }
    }

    if let Some(action) = action {
        match controls.selected {
            SelectedLight::Point => apply_action(&mut *point_light, action),
            SelectedLight::Spot => apply_action(&mut *spot_light, action),
        }
    }
}

fn start_key_repeat(held_key_state: &mut HeldKeyState, key: KeyCode) {
    held_key_state.key = Some(key);
    held_key_state.timer = Some(Timer::from_seconds(KEY_REPEAT_DELAY, TimerMode::Once));
}

fn apply_action<T: DemoLight>(light: &mut T, action: UiAction) {
    match action {
        UiAction::CycleFalloff => light.set_falloff(next_falloff(light.falloff())),
        UiAction::IntensityDown => {
            light.set_intensity((light.intensity() / INTENSITY_SCALE).max(0.0));
        }
        UiAction::IntensityUp => {
            light.set_intensity(light.intensity() * INTENSITY_SCALE);
        }
    }
}

fn next_falloff(falloff: LightFalloff) -> LightFalloff {
    match falloff {
        LightFalloff::InverseSquare => LightFalloff::Linear,
        LightFalloff::Linear => LightFalloff::Exponential,
        LightFalloff::Exponential => LightFalloff::InverseSquare,
    }
}

fn sync_light_status_text(
    controls: Res<LightControls>,
    point_light: Single<&PointLight, With<PointDemoLight>>,
    spot_light: Single<&SpotLight, With<SpotDemoLight>>,
    mut selected_text: Single<&mut Text, (With<SelectedLightLabel>, Without<LightStateLabel>)>,
    mut state_text: Single<&mut Text, (With<LightStateLabel>, Without<SelectedLightLabel>)>,
) {
    let (falloff, intensity) = match controls.selected {
        SelectedLight::Point => (point_light.falloff, point_light.intensity),
        SelectedLight::Spot => (spot_light.falloff, spot_light.intensity),
    };

    selected_text.0 = match controls.selected {
        SelectedLight::Point => "Point light".to_string(),
        SelectedLight::Spot => "Spot light".to_string(),
    };
    state_text.0 = format!(
        "Falloff: {}\nIntensity: {:.0}\n\nTab switch  F cycle  -/= intensity x{:.2}",
        falloff_label(falloff),
        intensity,
        INTENSITY_SCALE
    );
}

fn falloff_label(falloff: LightFalloff) -> &'static str {
    match falloff {
        LightFalloff::InverseSquare => "Inverse-square",
        LightFalloff::Linear => "Linear",
        LightFalloff::Exponential => "Exponential",
    }
}
