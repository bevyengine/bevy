//! A simple 3D scene with light shining over a cube sitting on a plane
//! and a force field around the cube

use std::f32::consts::FRAC_PI_2;

use bevy::{
    color::palettes::css::MEDIUM_SEA_GREEN,
    core_pipeline::{
        bloom::BloomSettings,
        prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass},
    },
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderRef, ShaderType},
};

const PREPASS_SHADER_ASSET_PATH: &str = "shaders/show_prepass.wgsl";
const MATERIAL_SHADER_ASSET_PATH: &str = "shaders/force_field.wgsl";

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MaterialPlugin::<ForceFieldMaterial>::default(),
            MaterialPlugin::<PrepassOutputMaterial> {
                prepass_enabled: false,
                ..default()
            },
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, toggle_prepass_view)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut force_field_materials: ResMut<Assets<ForceFieldMaterial>>,
    mut depth_materials: ResMut<Assets<PrepassOutputMaterial>>,
) {
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Rectangle::from_size(Vec2::splat(5.0))),
        material: materials.add(Color::from(MEDIUM_SEA_GREEN)),
        transform: Transform::from_rotation(Quat::from_rotation_x(-FRAC_PI_2)),
        ..default()
    });
    //cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::from_size(Vec3::splat(0.5))),
        material: materials.add(Color::WHITE),
        transform: Transform::from_xyz(0.0, 0.25, 0.0),
        ..default()
    });
    // sphere
    commands.spawn((
        MaterialMeshBundle {
            mesh: meshes.add(Sphere::new(1.25).mesh().uv(64, 64)),
            material: force_field_materials.add(ForceFieldMaterial {}),
            transform: Transform::from_xyz(0.0, 0.5, 0.0)
                .with_rotation(Quat::from_axis_angle(Vec3::X, std::f32::consts::FRAC_PI_2)),
            ..default()
        },
        NotShadowReceiver,
        NotShadowCaster,
    ));
    // Quad to show the depth prepass
    commands.spawn((
        MaterialMeshBundle {
            mesh: meshes.add(Rectangle::from_size(Vec2::new(20.0, 20.0))),
            material: depth_materials.add(PrepassOutputMaterial {
                settings: ShowPrepassSettings::default(),
            }),
            transform: Transform::from_xyz(-0.75, 1.25, 3.0)
                .looking_at(Vec3::new(2.0, -2.5, -5.0), Vec3::Y),
            ..default()
        },
        NotShadowCaster,
    ));
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    // camera
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        DepthPrepass,
        // This will generate a texture containing world normals (with normal maps applied)
        NormalPrepass,
        // This will generate a texture containing screen space pixel motion vectors
        MotionVectorPrepass,
        BloomSettings::default(),
    ));
}

// This is the struct that will be passed to your shader
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct ForceFieldMaterial {}

impl Material for ForceFieldMaterial {
    fn fragment_shader() -> ShaderRef {
        MATERIAL_SHADER_ASSET_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::render::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
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

// This shader simply loads the prepass texture and outputs it directly
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct PrepassOutputMaterial {
    #[uniform(0)]
    settings: ShowPrepassSettings,
}

impl Material for PrepassOutputMaterial {
    fn fragment_shader() -> ShaderRef {
        PREPASS_SHADER_ASSET_PATH.into()
    }

    // This needs to be transparent in order to show the scene behind the mesh
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

/// Every time you press space, it will cycle between transparent, depth and normals view
fn toggle_prepass_view(
    mut prepass_view: Local<u32>,
    keycode: Res<ButtonInput<KeyCode>>,
    material_handle: Query<&Handle<PrepassOutputMaterial>>,
    mut materials: ResMut<Assets<PrepassOutputMaterial>>,
) {
    if keycode.just_pressed(KeyCode::Space) {
        *prepass_view = (*prepass_view + 1) % 4;

        let handle = material_handle.single();
        let mat = materials.get_mut(handle).unwrap();
        mat.settings.show_depth = (*prepass_view == 1) as u32;
        mat.settings.show_normals = (*prepass_view == 2) as u32;
        mat.settings.show_motion_vectors = (*prepass_view == 3) as u32;
    }
}
