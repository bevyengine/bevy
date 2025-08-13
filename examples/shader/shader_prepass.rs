//! Bevy has an optional prepass that is controlled per-material. A prepass is a rendering pass that runs before the main pass.
//! It will optionally generate various view textures. Currently it supports depth, normal, and motion vector textures.
//! The textures are not generated for any material using alpha blending.

use bevy::{
    core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass},
    light::NotShadowCaster,
    pbr::PbrPlugin,
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};

/// This example uses a shader source file from the assets subdirectory
const PREPASS_SHADER_ASSET_PATH: &str = "shaders/show_prepass.wgsl";
const MATERIAL_SHADER_ASSET_PATH: &str = "shaders/custom_material.wgsl";

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(PbrPlugin {
                // The prepass is enabled by default on the StandardMaterial,
                // but you can disable it if you need to.
                //
                // prepass_enabled: false,
                ..default()
            }),
            MaterialPlugin::<CustomMaterial>::default(),
            MaterialPlugin::<PrepassOutputMaterial> {
                // This material only needs to read the prepass textures,
                // but the meshes using it should not contribute to the prepass render, so we can disable it.
                prepass_enabled: false,
                ..default()
            },
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (rotate, toggle_prepass_view))
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<CustomMaterial>>,
    mut std_materials: ResMut<Assets<StandardMaterial>>,
    mut depth_materials: ResMut<Assets<PrepassOutputMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 3., 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        // Disabling MSAA for maximum compatibility. Shader prepass with MSAA needs GPU capability MULTISAMPLED_SHADING
        Msaa::Off,
        // To enable the prepass you need to add the components associated with the ones you need
        // This will write the depth buffer to a texture that you can use in the main pass
        DepthPrepass,
        // This will generate a texture containing world normals (with normal maps applied)
        NormalPrepass,
        // This will generate a texture containing screen space pixel motion vectors
        MotionVectorPrepass,
    ));

    // plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
        MeshMaterial3d(std_materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));

    // A quad that shows the outputs of the prepass
    // To make it easy, we just draw a big quad right in front of the camera.
    // For a real application, this isn't ideal.
    commands.spawn((
        Mesh3d(meshes.add(Rectangle::new(20.0, 20.0))),
        MeshMaterial3d(depth_materials.add(PrepassOutputMaterial {
            settings: ShowPrepassSettings::default(),
        })),
        Transform::from_xyz(-0.75, 1.25, 3.0).looking_at(Vec3::new(2.0, -2.5, -5.0), Vec3::Y),
        NotShadowCaster,
    ));

    // Opaque cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(CustomMaterial {
            color: LinearRgba::WHITE,
            color_texture: Some(asset_server.load("branding/icon.png")),
            alpha_mode: AlphaMode::Opaque,
        })),
        Transform::from_xyz(-1.0, 0.5, 0.0),
        Rotates,
    ));

    // Cube with alpha mask
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(std_materials.add(StandardMaterial {
            alpha_mode: AlphaMode::Mask(1.0),
            base_color_texture: Some(asset_server.load("branding/icon.png")),
            ..default()
        })),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // Cube with alpha blending.
    // Transparent materials are ignored by the prepass
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(CustomMaterial {
            color: LinearRgba::WHITE,
            color_texture: Some(asset_server.load("branding/icon.png")),
            alpha_mode: AlphaMode::Blend,
        })),
        Transform::from_xyz(1.0, 0.5, 0.0),
    ));

    // light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    commands.spawn((
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        children![
            TextSpan::new("Prepass Output: transparent\n"),
            TextSpan::new("\n\n"),
            TextSpan::new("Controls\n"),
            TextSpan::new("---------------\n"),
            TextSpan::new("Space - Change output\n"),
        ],
    ));
}

// This is the struct that will be passed to your shader
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct CustomMaterial {
    #[uniform(0)]
    color: LinearRgba,
    #[texture(1)]
    #[sampler(2)]
    color_texture: Option<Handle<Image>>,
    alpha_mode: AlphaMode,
}

/// Not shown in this example, but if you need to specialize your material, the specialize
/// function will also be used by the prepass
impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        MATERIAL_SHADER_ASSET_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }

    // You can override the default shaders used in the prepass if your material does
    // anything not supported by the default prepass
    // fn prepass_fragment_shader() -> ShaderRef {
    //     "shaders/custom_material.wgsl".into()
    // }
}

#[derive(Component)]
struct Rotates;

fn rotate(mut q: Query<&mut Transform, With<Rotates>>, time: Res<Time>) {
    for mut t in q.iter_mut() {
        let rot = (ops::sin(time.elapsed_secs()) * 0.5 + 0.5) * std::f32::consts::PI * 2.0;
        t.rotation = Quat::from_rotation_z(rot);
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
    material_handle: Single<&MeshMaterial3d<PrepassOutputMaterial>>,
    mut materials: ResMut<Assets<PrepassOutputMaterial>>,
    text: Single<Entity, With<Text>>,
    mut writer: TextUiWriter,
) {
    if keycode.just_pressed(KeyCode::Space) {
        *prepass_view = (*prepass_view + 1) % 4;

        let label = match *prepass_view {
            0 => "transparent",
            1 => "depth",
            2 => "normals",
            3 => "motion vectors",
            _ => unreachable!(),
        };
        let text = *text;
        *writer.text(text, 1) = format!("Prepass Output: {label}\n");
        writer.for_each_color(text, |mut color| {
            color.0 = Color::WHITE;
        });

        let mat = materials.get_mut(*material_handle).unwrap();
        mat.settings.show_depth = (*prepass_view == 1) as u32;
        mat.settings.show_normals = (*prepass_view == 2) as u32;
        mat.settings.show_motion_vectors = (*prepass_view == 3) as u32;
    }
}
