//! Bevy has an optional prepass that is controlled per-material. A prepass is a rendering pass that runs before the main pass.
//! It will optionally generate various view textures. Currently it supports depth and normal textures.
//! The textures are not generated for any material using alpha blending.
//!
//! # WARNING
//! The prepass currently doesn't work on `WebGL`.

use bevy::{
    core_pipeline::prepass::{DepthPrepass, NormalPrepass},
    pbr::{NotShadowCaster, PbrPlugin},
    prelude::*,
    reflect::TypeUuid,
    render::render_resource::{AsBindGroup, ShaderRef},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(PbrPlugin {
            // The prepass is enabled by default on the StandardMaterial,
            // but you can disable it if you need to.
            // prepass_enabled: false,
            ..default()
        }))
        .add_plugin(MaterialPlugin::<CustomMaterial>::default())
        .add_plugin(MaterialPlugin::<PrepassOutputMaterial> {
            // This material only needs to read the prepass textures,
            // but the meshes using it should not contribute to the prepass render, so we can disable it.
            prepass_enabled: false,
            ..default()
        })
        .add_startup_system(setup)
        .add_system(rotate)
        .add_system(update)
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
            ..default()
        },
        // To enable the prepass you need to add the components associated with the ones you need
        // This will write the depth buffer to a texture that you can use in the main pass
        DepthPrepass,
        // This will generate a texture containing world normals (with normal maps applied)
        NormalPrepass,
    ));

    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(5.0).into()),
        material: std_materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });

    // A quad that shows the outputs of the prepass
    // To make it easy, we just draw a big quad right in front of the camera. For a real application, this isn't ideal.
    commands.spawn((
        MaterialMeshBundle {
            mesh: meshes.add(shape::Quad::new(Vec2::new(20.0, 20.0)).into()),
            material: depth_materials.add(PrepassOutputMaterial {
                show_depth: 0.0,
                show_normal: 0.0,
            }),
            transform: Transform::from_xyz(-0.75, 1.25, 3.0)
                .looking_at(Vec3::new(2.0, -2.5, -5.0), Vec3::Y),
            ..default()
        },
        NotShadowCaster,
    ));

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

    let style = TextStyle {
        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
        font_size: 18.0,
        color: Color::WHITE,
    };

    commands.spawn(
        TextBundle::from_sections(vec![
            TextSection::new("Prepass Output: transparent\n", style.clone()),
            TextSection::new("\n\n", style.clone()),
            TextSection::new("Controls\n", style.clone()),
            TextSection::new("---------------\n", style.clone()),
            TextSection::new("Space - Change output\n", style),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: Val::Px(10.0),
                left: Val::Px(10.0),
                ..default()
            },
            ..default()
        }),
    );
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
/// function will also be used by the prepass
impl Material for CustomMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/custom_material.wgsl".into()
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
        let rot = (time.elapsed_seconds().sin() * 0.5 + 0.5) * std::f32::consts::PI * 2.0;
        t.rotation = Quat::from_rotation_z(rot);
    }
}

// This shader simply loads the prepass texture and outputs it directly
#[derive(AsBindGroup, TypeUuid, Debug, Clone)]
#[uuid = "0af99895-b96e-4451-bc12-c6b1c1c52750"]
pub struct PrepassOutputMaterial {
    #[uniform(0)]
    show_depth: f32,
    #[uniform(1)]
    show_normal: f32,
}

impl Material for PrepassOutputMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/show_prepass.wgsl".into()
    }

    // This needs to be transparent in order to show the scene behind the mesh
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

fn update(
    keycode: Res<Input<KeyCode>>,
    material_handle: Query<&Handle<PrepassOutputMaterial>>,
    mut materials: ResMut<Assets<PrepassOutputMaterial>>,
    mut text: Query<&mut Text>,
) {
    if keycode.just_pressed(KeyCode::Space) {
        let handle = material_handle.single();
        let mut mat = materials.get_mut(handle).unwrap();
        let out_text;
        if mat.show_depth == 1.0 {
            out_text = "normal";
            mat.show_depth = 0.0;
            mat.show_normal = 1.0;
        } else if mat.show_normal == 1.0 {
            out_text = "transparent";
            mat.show_depth = 0.0;
            mat.show_normal = 0.0;
        } else {
            out_text = "depth";
            mat.show_depth = 1.0;
            mat.show_normal = 0.0;
        }

        let mut text = text.single_mut();
        text.sections[0].value = format!("Prepass Output: {out_text}\n");
    }
}
