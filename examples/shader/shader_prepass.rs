//! Bevy has an optional prepass that is controlled per-material. A prepass is a rendering pass that runs before the main pass.
//! It will optionally generate various view textures. Currently it supports depth and normal textures.
//! The textures are not generated for any material using alpha blending.

use bevy::{
    core_pipeline::prepass::DepthPrepass,
    pbr::PbrPlugin,
    prelude::*,
    reflect::TypeUuid,
    render::render_resource::{AsBindGroup, ShaderRef},
};

fn main() {
    App::new()
        // To enable the prepass on the StandardMaterial, we need to enable it on the PbrPlugin
        .add_plugins(DefaultPlugins.set(PbrPlugin {
            prepass_enabled: true,
        }))
        .add_plugin(MaterialPlugin::<CustomMaterial> {
            // The prepass is enabled per material and is disabled by default
            // So we need to enable it on our custom material.
            prepass_enabled: true,
            ..default()
        })
        // This material only needs to read the prepass textures, so it doesn't need to be enabled.
        .add_plugin(MaterialPlugin::<PrepassOutputMaterial>::default())
        .add_startup_system(setup)
        .add_system(rotate)
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
        Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 3., 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            // You can configure which textures is going to be used by the prepass.
            ..default()
        },
        // To enable the prepass you need to add the components associated with the ones you need
        // This will enable the depth prepass
        DepthPrepass,
        // This will enable the normal prepass
        // NormalPrepass
    ));

    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: std_materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });

    // A plane that shows the output of the depth prepass
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Quad {
            flip: false,
            size: Vec2::new(1.5, 1.5),
        })),
        material: depth_materials.add(PrepassOutputMaterial {}),
        transform: Transform::from_xyz(-0.75, 1.25, 2.0)
            .looking_at(Vec3::new(2.0, -2.5, -5.0), Vec3::Y),
        ..default()
    });

    // Opaque cube using the StandardMaterial
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: std_materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(-1.0, 0.5, 0.0),
            ..default()
        },
        Rotates,
    ));

    // Cube with alpha mask
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: std_materials.add(StandardMaterial {
            alpha_mode: AlphaMode::Mask(1.0),
            base_color_texture: Some(asset_server.load("branding/icon.png")),
            ..default()
        }),

        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });

    // Cube with alpha blending.
    // Transparent materials are ignored by the prepass
    commands.spawn(MaterialMeshBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(CustomMaterial {
            color: Color::WHITE,
            color_texture: Some(asset_server.load("branding/icon.png")),
            alpha_mode: AlphaMode::Blend,
        }),
        transform: Transform::from_xyz(1.0, 0.5, 0.0),
        ..default()
    });

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
}

// This is the struct that will be passed to your shader
#[derive(AsBindGroup, TypeUuid, Debug, Clone)]
#[uuid = "f690fdae-d598-45ab-8225-97e2a3f056e0"]
pub struct CustomMaterial {
    #[uniform(0)]
    color: Color,
    #[texture(1)]
    #[sampler(2)]
    color_texture: Option<Handle<Image>>,
    alpha_mode: AlphaMode,
}

/// Not shown in this example, but if you need to specialize your material, the specialize
/// function will also be used in the prepass
impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/custom_material.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }

    // You can override the default shaders used in the prepass if your material does
    // anything not supported by default
    // fn prepass_fragment_shader() -> ShaderRef {
    //     "shaders/custom_material.wgsl".into()
    // }
}

#[derive(Component)]
struct Rotates;

fn rotate(mut q: Query<&mut Transform, With<Rotates>>, time: Res<Time>) {
    for mut t in q.iter_mut() {
        let rot = (time.elapsed_seconds().sin() * 0.5 + 0.5) * std::f32::consts::PI * 2.0;
        t.rotation = Quat::from_rotation_z(rot);
    }
}

// This shader simply loads the prepass texture and outputs it directly
#[derive(AsBindGroup, TypeUuid, Debug, Clone)]
#[uuid = "0af99895-b96e-4451-bc12-c6b1c1c52750"]
pub struct PrepassOutputMaterial {}

impl Material for PrepassOutputMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/show_depth.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}
