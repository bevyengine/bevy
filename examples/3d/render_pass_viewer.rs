//! Demonstrates the `RenderPasses` / `RenderPassMask` API using a simple, keyboard-driven
//! "render pass viewer".
//!
//! This is inspired by editor tooling (like Unreal Engine's pass visualization), but keeps the UX
//! intentionally lightweight: you can toggle which passes meshes participate in, and cycle a simple
//! prepass output overlay.
//!
//! ## Controls
//!
//! | Key Binding   | Action |
//! |:-------------|:-------|
//! | `Space`      | Cycle overlay: main color / depth / normals / motion vectors |
//! | `1`          | Toggle `OPAQUE_MAIN` participation |
//! | `2`          | Toggle `ALPHA_MASK_MAIN` participation |
//! | `3`          | Toggle `TRANSPARENT_MAIN` participation |
//! | `4`          | Toggle `TRANSMISSIVE_MAIN` participation |
//! | `P`          | Toggle `PREPASS` participation (depth/normals/motion) |
//! | `S`          | Toggle `SHADOW` participation |
//! | `R`          | Reset to `RenderPassMask::ALL` |

use bevy::{
    color::palettes::basic::{BLUE, GRAY, GREEN, RED, YELLOW},
    core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass},
    light::NotShadowCaster,
    math::ops,
    prelude::*,
    reflect::TypePath,
    render::{
        render_resource::{AsBindGroup, ShaderType},
        RenderPassMask, RenderPasses,
    },
    shader::ShaderRef,
};

/// This example uses the same WGSL helper as `examples/shader/shader_prepass.rs`.
const PREPASS_SHADER_ASSET_PATH: &str = "shaders/show_prepass.wgsl";

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MaterialPlugin::<PrepassOutputMaterial>::default(),
        ))
        .init_resource::<ViewerState>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                handle_input,
                apply_pass_mask_to_controlled,
                update_overlay_material,
                update_ui_text,
                animate,
            ),
        )
        .run();
}

#[derive(Resource, Debug, Clone)]
struct ViewerState {
    overlay_mode: OverlayMode,

    opaque_main: bool,
    alpha_mask_main: bool,
    transparent_main: bool,
    transmissive_main: bool,
    prepass: bool,
    shadow: bool,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            overlay_mode: OverlayMode::MainColor,
            opaque_main: true,
            alpha_mask_main: true,
            transparent_main: true,
            transmissive_main: true,
            prepass: true,
            shadow: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlayMode {
    MainColor,
    Depth,
    Normals,
    MotionVectors,
}

impl OverlayMode {
    fn label(self) -> &'static str {
        match self {
            OverlayMode::MainColor => "main color",
            OverlayMode::Depth => "depth",
            OverlayMode::Normals => "normals",
            OverlayMode::MotionVectors => "motion vectors",
        }
    }

    fn next(self) -> Self {
        match self {
            OverlayMode::MainColor => OverlayMode::Depth,
            OverlayMode::Depth => OverlayMode::Normals,
            OverlayMode::Normals => OverlayMode::MotionVectors,
            OverlayMode::MotionVectors => OverlayMode::MainColor,
        }
    }
}

#[derive(Component)]
struct Controlled;

#[derive(Component)]
struct OverlayQuad;

#[derive(Component)]
struct UiText;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut overlay_materials: ResMut<Assets<PrepassOutputMaterial>>,
) {
    // Camera with prepass enabled.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-3.0, 3.0, 7.0).looking_at(Vec3::new(0.0, 0.75, 0.0), Vec3::Y),
        Msaa::Off,
        DepthPrepass,
        NormalPrepass,
        MotionVectorPrepass,
        Name::new("Camera"),
    ));

    // Lighting.
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: light_consts::lux::OVERCAST_DAY,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            std::f32::consts::FRAC_PI_2,
            -std::f32::consts::FRAC_PI_4,
        )),
        Name::new("Directional Light"),
    ));

    // Ground.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(14.0, 14.0))),
        MeshMaterial3d(materials.add(Color::from(GRAY))),
        Name::new("Ground"),
    ));

    // Controlled meshes: one per main pass category.
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    // Opaque.
    commands.spawn((
        Mesh3d(cube.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::from(RED),
            ..default()
        })),
        Transform::from_xyz(-2.4, 0.5, 0.0),
        RenderPasses::default(),
        Controlled,
        Name::new("Opaque (red)"),
    ));

    // Alpha mask.
    commands.spawn((
        Mesh3d(cube.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::from(GREEN),
            alpha_mode: AlphaMode::Mask(0.5),
            ..default()
        })),
        Transform::from_xyz(-0.8, 0.5, 0.0),
        RenderPasses::default(),
        Controlled,
        Name::new("Alpha-mask (green)"),
    ));

    // Transparent.
    commands.spawn((
        Mesh3d(cube.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::from(BLUE).with_alpha(0.35),
            alpha_mode: AlphaMode::Blend,
            ..default()
        })),
        Transform::from_xyz(0.8, 0.5, 0.0),
        RenderPasses::default(),
        Controlled,
        Name::new("Transparent (blue)"),
    ));

    // Transmissive.
    commands.spawn((
        Mesh3d(cube.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::from(YELLOW),
            specular_transmission: 0.9,
            diffuse_transmission: 1.0,
            thickness: 1.2,
            ior: 1.5,
            perceptual_roughness: 0.12,
            ..default()
        })),
        Transform::from_xyz(2.4, 0.5, 0.0),
        RenderPasses::default(),
        Controlled,
        Name::new("Transmissive (yellow)"),
    ));

    // A big quad that visualizes prepass outputs.
    // This draws as transparent so the main scene stays visible.
    commands.spawn((
        Mesh3d(meshes.add(Rectangle::new(20.0, 20.0))),
        MeshMaterial3d(overlay_materials.add(PrepassOutputMaterial {
            settings: ShowPrepassSettings::default(),
        })),
        Transform::from_xyz(-0.9, 1.4, 3.0).looking_at(Vec3::new(2.0, -2.5, -5.0), Vec3::Y),
        NotShadowCaster,
        OverlayQuad,
        Name::new("Prepass overlay quad"),
    ));

    // UI.
    commands.spawn((
        Text::new(initial_ui_text(&ViewerState::default())),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
        UiText,
    ));
}

fn initial_ui_text(state: &ViewerState) -> String {
    let on_off = |b: bool| if b { "on" } else { "off" };
    format!(
        "Render Pass Viewer\n\n\
Overlay: {}\n\n\
Toggles\n--------\n\
1 Opaque main: {}\n\
2 Alpha-mask main: {}\n\
3 Transparent main: {}\n\
4 Transmissive main: {}\n\
P Prepass: {}\n\
S Shadows: {}\n\
R Reset\n\
Space Cycle overlay\n",
        state.overlay_mode.label(),
        on_off(state.opaque_main),
        on_off(state.alpha_mask_main),
        on_off(state.transparent_main),
        on_off(state.transmissive_main),
        on_off(state.prepass),
        on_off(state.shadow),
    )
}

fn handle_input(keys: Res<ButtonInput<KeyCode>>, mut state: ResMut<ViewerState>) {
    if keys.just_pressed(KeyCode::Space) {
        state.overlay_mode = state.overlay_mode.next();
    }

    if keys.just_pressed(KeyCode::Digit1) {
        state.opaque_main = !state.opaque_main;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        state.alpha_mask_main = !state.alpha_mask_main;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        state.transparent_main = !state.transparent_main;
    }
    if keys.just_pressed(KeyCode::Digit4) {
        state.transmissive_main = !state.transmissive_main;
    }
    if keys.just_pressed(KeyCode::KeyP) {
        state.prepass = !state.prepass;
    }
    if keys.just_pressed(KeyCode::KeyS) {
        state.shadow = !state.shadow;
    }

    if keys.just_pressed(KeyCode::KeyR) {
        *state = ViewerState::default();
    }
}

fn apply_pass_mask_to_controlled(
    mut query: Query<&mut RenderPasses, With<Controlled>>,
    state: Res<ViewerState>,
) {
    if !state.is_changed() {
        return;
    }

    let mut mask = RenderPassMask::ALL;

    if !state.opaque_main {
        mask.remove(RenderPassMask::OPAQUE_MAIN);
    }
    if !state.alpha_mask_main {
        mask.remove(RenderPassMask::ALPHA_MASK_MAIN);
    }
    if !state.transparent_main {
        mask.remove(RenderPassMask::TRANSPARENT_MAIN);
    }
    if !state.transmissive_main {
        mask.remove(RenderPassMask::TRANSMISSIVE_MAIN);
    }
    if !state.prepass {
        mask.remove(RenderPassMask::PREPASS);
    }
    if !state.shadow {
        mask.remove(RenderPassMask::SHADOW);
    }

    for mut passes in &mut query {
        passes.0 = mask;
    }
}

fn update_overlay_material(
    state: Res<ViewerState>,
    overlay_material: Single<&MeshMaterial3d<PrepassOutputMaterial>, With<OverlayQuad>>,
    mut materials: ResMut<Assets<PrepassOutputMaterial>>,
) {
    if !state.is_changed() {
        return;
    }

    let Some(mat) = materials.get_mut(*overlay_material) else {
        return;
    };

    mat.settings.show_depth = (state.overlay_mode == OverlayMode::Depth) as u32;
    mat.settings.show_normals = (state.overlay_mode == OverlayMode::Normals) as u32;
    mat.settings.show_motion_vectors = (state.overlay_mode == OverlayMode::MotionVectors) as u32;
}

fn update_ui_text(state: Res<ViewerState>, mut text_query: Single<&mut Text, With<UiText>>) {
    if !state.is_changed() {
        return;
    }

    text_query.0 = initial_ui_text(state.as_ref());
}

fn animate(time: Res<Time>, mut query: Query<&mut Transform, With<Controlled>>) {
    // Move / rotate the controlled meshes so the motion vector overlay is more obvious.
    let t = time.elapsed_secs();
    let wobble = ops::sin(t) * 0.35;

    for (i, mut transform) in query.iter_mut().enumerate() {
        transform.rotation = Quat::from_rotation_y(t * 0.8 + i as f32);
        transform.translation.z = wobble + i as f32 * 0.15;
    }
}

#[derive(Debug, Clone, Default, ShaderType)]
struct ShowPrepassSettings {
    show_depth: u32,
    show_normals: u32,
    show_motion_vectors: u32,
    padding_1: u32,
    padding_2: u32,
}

/// A very small helper material that visualizes the camera's prepass view textures.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct PrepassOutputMaterial {
    #[uniform(0)]
    settings: ShowPrepassSettings,
}

impl Material for PrepassOutputMaterial {
    fn fragment_shader() -> ShaderRef {
        PREPASS_SHADER_ASSET_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        // Transparent so we can see the main scene behind the overlay quad.
        AlphaMode::Blend
    }

    fn enable_prepass() -> bool {
        // The overlay quad should not contribute to the prepass.
        false
    }
}
