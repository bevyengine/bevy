//! Test meshlets by showing them next to the mesh they were created from.

use bevy::{
    asset::RenderAssetUsages,
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    image::{ImageAddressMode, ImageFilterMode},
    input::common_conditions::input_just_pressed,
    pbr::experimental::meshlet::{
        MeshletMesh, MeshletMesh3d, MeshletPlugin,
        MESHLET_DEFAULT_VERTEX_POSITION_QUANTIZATION_FACTOR,
    },
    prelude::*,
    render::render_resource::{AsBindGroup, Extent3d, TextureDimension, TextureFormat},
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MeshletPlugin {
                cluster_buffer_slots: 1 << 14,
            },
            MaterialPlugin::<MeshletDebugMaterial>::default(),
            FreeCameraPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            toggle_materials.run_if(input_just_pressed(KeyCode::KeyM)),
        )
        .insert_resource(GlobalAmbientLight {
            brightness: 500.0,
            ..default()
        })
        .run();
}

fn setup(
    mut commands: Commands,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut debug_materials: ResMut<Assets<MeshletDebugMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut meshlets: ResMut<Assets<MeshletMesh>>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.spawn_scene_list(bsn_list! {
        Camera3d
        Transform::from_translation(Vec3::new(0.0, 1.5, -3.0))
            .looking_at(vec3(0.0, 0.0, 1.5), Vec3::Y)
        Msaa::Off
        FreeCamera
        --
        DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT
        }
        Transform::from_xyz(0.0, 1.0, -0.5).looking_at(Vec3::ZERO, Vec3::Z)
        --
        Text::new("M: toggle debug material")
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
        }
        --
        Text::new("meshlet")
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: percent(25),
        }
        --
        Text::new("regular")
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            right: percent(25),
        }
    });

    let sphere_mesh = Sphere::default().mesh().uv(48, 27);
    let cube_mesh = Cuboid::from_length(0.8).mesh().build();

    let sphere_meshlet = MeshletMesh::from_mesh(
        &sphere_mesh,
        MESHLET_DEFAULT_VERTEX_POSITION_QUANTIZATION_FACTOR,
    )
    .unwrap();

    let cube_meshlet = MeshletMesh::from_mesh(
        &cube_mesh,
        MESHLET_DEFAULT_VERTEX_POSITION_QUANTIZATION_FACTOR,
    )
    .unwrap();

    let sphere_mesh_asset = meshes.add(sphere_mesh);
    let cube_mesh_asset = meshes.add(cube_mesh);

    let sphere_meshlet_asset = meshlets.add(sphere_meshlet);
    let cube_meshlet_asset = meshlets.add(cube_meshlet);

    let mipmap_material = mipmap_material(&mut standard_materials, &mut images);
    let debug_material = debug_materials.add(MeshletDebugMaterial::default());

    for distance in [0.0, 8.0, 32.0, 128.0] {
        commands.spawn_scene_list(bsn_list! {
            Mesh3d({sphere_mesh_asset.clone()})
            MeshMaterial3d::<StandardMaterial>({mipmap_material.clone()})
            Transform::from_xyz(-0.5, 0.0, distance)
            --
            MeshletMesh3d({sphere_meshlet_asset.clone()})
            MeshMaterial3d::<StandardMaterial>({mipmap_material.clone()})
            Transform::from_xyz(0.5, 0.0, distance)
            --
            Mesh3d({cube_mesh_asset.clone()})
            MeshMaterial3d::<StandardMaterial>({mipmap_material.clone()})
            Transform::from_xyz(-1.5, 0.0, distance)
            --
            MeshletMesh3d({cube_meshlet_asset.clone()})
            MeshMaterial3d::<StandardMaterial>({mipmap_material.clone()})
            Transform::from_xyz(1.5, 0.0, distance)
        });
    }

    commands.insert_resource(Materials {
        mipmap: mipmap_material,
        debug: debug_material,
    });
}

fn toggle_materials(
    mut commands: Commands,
    materials: Res<Materials>,
    meshlet_materials: Query<
        (
            Entity,
            Option<&MeshMaterial3d<StandardMaterial>>,
            Option<&MeshMaterial3d<MeshletDebugMaterial>>,
        ),
        With<MeshletMesh3d>,
    >,
) {
    for (entity, mipmap, debug) in meshlet_materials {
        if mipmap.is_some() {
            commands
                .entity(entity)
                .remove::<MeshMaterial3d<StandardMaterial>>()
                .insert(MeshMaterial3d(materials.debug.clone()));
        } else if debug.is_some() {
            commands
                .entity(entity)
                .remove::<MeshMaterial3d<MeshletDebugMaterial>>()
                .insert(MeshMaterial3d(materials.mipmap.clone()));
        }
    }
}

#[derive(Resource)]
struct Materials {
    mipmap: Handle<StandardMaterial>,
    debug: Handle<MeshletDebugMaterial>,
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Default)]
struct MeshletDebugMaterial {
    _dummy: (),
}

impl Material for MeshletDebugMaterial {}

fn checkerboard(size: usize, color: Srgba) -> Vec<u8> {
    let color = color.to_u8_array();
    let black = Srgba::BLACK.to_u8_array();
    let shift = size.ilog2().saturating_sub(3);

    (0..(size * size))
        .into_iter()
        .flat_map(|i| {
            let x = i.rem_euclid(size) >> shift;
            let y = (i / size) >> shift;
            if ((x + y) & 1) == 0 {
                color
            } else {
                black
            }
        })
        .collect()
}

// Create a material where each mip has a different color.
fn mipmap_material(
    standard_materials: &mut Assets<StandardMaterial>,
    images: &mut Assets<Image>,
) -> Handle<StandardMaterial> {
    let mip_colors = [
        Srgba::rgb(1.0, 0.0, 0.0),
        Srgba::rgb(0.0, 1.0, 0.0),
        Srgba::rgb(0.0, 0.0, 1.0),
        Srgba::rgb(1.0, 1.0, 0.0),
        Srgba::rgb(0.0, 1.0, 1.0),
        Srgba::rgb(1.0, 0.0, 1.0),
    ];

    let size = 256;

    let mut image = Image::new_uninit(
        Extent3d {
            width: size as u32,
            height: size as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );

    image.texture_descriptor.mip_level_count = mip_colors.len() as u32;

    image
        .sampler
        .get_or_init_descriptor()
        .set_filter(ImageFilterMode::Linear)
        .set_address_mode(ImageAddressMode::Repeat);

    image.data = Some(
        mip_colors
            .iter()
            .enumerate()
            .flat_map(|(level, color)| checkerboard(size / 2_usize.pow(level as u32), *color))
            .collect(),
    );

    standard_materials.add(StandardMaterial {
        base_color_texture: Some(images.add(image)),
        perceptual_roughness: 1.0,
        ..default()
    })
}
