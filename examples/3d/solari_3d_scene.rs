//! A simple 3D scene with light shining over a cube sitting on a plane.
//!
//! This example is intended to demonstrate the bare minimum setup required to enable Solari,
//! Bevy's real-time raytraced dynamic lighting solution.

use bevy::{
    camera::CameraMainTextureUsages, color::palettes::css, prelude::*,
    render::render_resource::TextureUsages, solari::prelude::*,
};

/// Real-time raytracing produces noisy output because it cannot trace enough rays per pixel in a single frame.
/// Instead, it distributes work stochastically across frames.
/// Therefore, a denoiser is required to achieve high-quality image.
/// DLSS Ray Reconstruction provides hardware-accelerated denoising.

#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
use bevy::anti_alias::dlss::{
    Dlss, DlssProjectId, DlssRayReconstructionFeature, DlssRayReconstructionSupported,
};

fn main() {
    let mut app = App::new();

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    app.insert_resource(DlssProjectId(bevy_asset::uuid::uuid!(
        "bd7a4665-6340-46f3-b4f8-c6d0f7101c27" // Don't copy paste this - generate your own UUID!
    )));

    app.add_plugins(DefaultPlugins)
        .add_plugins(SolariPlugins)
        .add_systems(Startup, setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))] dlss_rr_supported: Option<
        Res<DlssRayReconstructionSupported>,
    >,
) {
    let cube = meshes.add(
        Cuboid::from_length(1.0)
            .mesh()
            .build()
            // Solari requires ATTRIBUTE_TANGENT
            .with_generated_tangents()
            .expect("Cuboid mesh has ATTRIBUTE_UV_0"),
    );

    // base
    commands.spawn((
        Mesh3d(cube.clone()),
        RaytracingMesh3d(cube.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: css::DARK_GRAY.into(),
            perceptual_roughness: 0.0,
            metallic: 0.15,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.1, 0.0).with_scale(Vec3::splat(7.0).with_y(0.2)),
    ));
    // cube
    commands.spawn((
        Mesh3d(cube.clone()),
        RaytracingMesh3d(cube.clone()),
        MeshMaterial3d(materials.add(Color::from(css::LIGHT_BLUE))),
        Transform::from_xyz(1.0, 0.5, -0.25),
    ));
    // emissive light
    commands.spawn((
        Mesh3d(cube.clone()),
        RaytracingMesh3d(cube),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: scene_color().into(),
            emissive: LinearRgba::from(scene_color()) * 100_000.0,
            ..default()
        })),
        Transform::from_xyz(-1.0, 0.25, 0.25).with_scale(Vec3::splat(0.5)),
    ));
    // directional light
    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::HALLWAY,
            ..default()
        },
        Transform::from_xyz(-2.5, 4.5, -3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // camera
    let mut _camera = commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        CameraMainTextureUsages::default().with(TextureUsages::STORAGE_BINDING),
        Msaa::Off,
        SolariLighting::default(),
    ));

    // Using DLSS Ray Reconstruction for denoising (and cheaper rendering via upscaling) is _highly_ recommended when using Solari
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    if dlss_rr_supported.is_some() {
        _camera.insert(Dlss::<DlssRayReconstructionFeature> {
            perf_quality_mode: Default::default(),
            reset: Default::default(),
            _phantom_data: Default::default(),
        });
    }
}

// Solari logo color
fn scene_color() -> Srgba {
    Srgba::rgb_u8(255, 137, 4)
}
