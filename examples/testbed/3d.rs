//! 3d testbed
//!
//! You can switch scene by pressing the spacebar

mod helpers;

use argh::FromArgs;
use bevy::prelude::*;
use helpers::Next;

#[derive(FromArgs)]
/// 3d testbed
pub struct Args {
    #[argh(positional)]
    scene: Option<Scene>,
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args: Args = Args::from_args(&[], &[]).unwrap();

    let mut app = App::new();
    app.add_plugins((DefaultPlugins,))
        .add_systems(OnEnter(Scene::Light), light::setup)
        .add_systems(OnEnter(Scene::Bloom), bloom::setup)
        .add_systems(OnEnter(Scene::Gltf), gltf::setup)
        .add_systems(OnEnter(Scene::Animation), animation::setup)
        .add_systems(OnEnter(Scene::Gizmos), gizmos::setup)
        .add_systems(
            OnEnter(Scene::GltfCoordinateConversion),
            gltf_coordinate_conversion::setup,
        )
        .add_systems(
            OnEnter(Scene::WhiteFurnaceSolidColorLight),
            white_furnace_solid_color_light::setup,
        )
        .add_systems(
            OnEnter(Scene::WhiteFurnaceEnvironmentMapLight),
            white_furnace_environment_map_light::setup,
        )
        .add_systems(OnEnter(Scene::RenderLayers), render_layers::setup)
        .add_systems(Update, switch_scene)
        .add_systems(Update, gizmos::draw_gizmos.run_if(in_state(Scene::Gizmos)))
        .add_systems(
            Update,
            gltf_coordinate_conversion::draw_gizmos
                .run_if(in_state(Scene::GltfCoordinateConversion)),
        );

    match args.scene {
        None => app.init_state::<Scene>(),
        Some(scene) => app.insert_state(scene),
    };

    #[cfg(feature = "bevy_ci_testing")]
    app.add_systems(Update, helpers::switch_scene_in_ci::<Scene>);

    app.run();
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
enum Scene {
    #[default]
    Light,
    Bloom,
    Gltf,
    Animation,
    Gizmos,
    GltfCoordinateConversion,
    WhiteFurnaceSolidColorLight,
    WhiteFurnaceEnvironmentMapLight,
    RenderLayers,
}

impl std::str::FromStr for Scene {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut isit = Self::default();
        while s.to_lowercase() != format!("{isit:?}").to_lowercase() {
            isit = isit.next();
            if isit == Self::default() {
                return Err(format!("Invalid Scene name: {s}"));
            }
        }
        Ok(isit)
    }
}

impl Next for Scene {
    fn next(&self) -> Self {
        match self {
            Scene::Light => Scene::Bloom,
            Scene::Bloom => Scene::Gltf,
            Scene::Gltf => Scene::Animation,
            Scene::Animation => Scene::Gizmos,
            Scene::Gizmos => Scene::GltfCoordinateConversion,
            Scene::GltfCoordinateConversion => Scene::WhiteFurnaceSolidColorLight,
            Scene::WhiteFurnaceSolidColorLight => Scene::WhiteFurnaceEnvironmentMapLight,
            Scene::WhiteFurnaceEnvironmentMapLight => Scene::RenderLayers,
            Scene::RenderLayers => Scene::Light,
        }
    }
}

fn switch_scene(
    keyboard: Res<ButtonInput<KeyCode>>,
    scene: Res<State<Scene>>,
    mut next_scene: ResMut<NextState<Scene>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        info!("Switching scene");
        next_scene.set(scene.get().next());
    }
}

mod light {
    use std::f32::consts::PI;

    use bevy::{
        color::palettes::css::{DEEP_PINK, LIME, RED},
        prelude::*,
    };

    const CURRENT_SCENE: super::Scene = super::Scene::Light;

    pub fn setup(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(10.0, 10.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::WHITE,
                perceptual_roughness: 1.0,
                ..default()
            })),
            DespawnOnExit(CURRENT_SCENE),
        ));

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::default())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: DEEP_PINK.into(),
                ..default()
            })),
            Transform::from_xyz(0.0, 1.0, 0.0),
            DespawnOnExit(CURRENT_SCENE),
        ));

        commands.spawn((
            PointLight {
                intensity: 100_000.0,
                color: RED.into(),
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_xyz(1.0, 2.0, 0.0),
            DespawnOnExit(CURRENT_SCENE),
        ));

        commands.spawn((
            SpotLight {
                intensity: 100_000.0,
                color: LIME.into(),
                shadow_maps_enabled: true,
                inner_angle: 0.6,
                outer_angle: 0.8,
                ..default()
            },
            Transform::from_xyz(-1.0, 2.0, 0.0).looking_at(Vec3::new(-1.0, 0.0, 0.0), Vec3::Z),
            DespawnOnExit(CURRENT_SCENE),
        ));

        commands.spawn((
            DirectionalLight {
                illuminance: light_consts::lux::OVERCAST_DAY,
                shadow_maps_enabled: true,
                ..default()
            },
            Transform {
                translation: Vec3::new(0.0, 2.0, 0.0),
                rotation: Quat::from_rotation_x(-PI / 4.),
                ..default()
            },
            DespawnOnExit(CURRENT_SCENE),
        ));

        commands.spawn((
            Camera3d::default(),
            Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            DespawnOnExit(CURRENT_SCENE),
        ));
    }
}

mod bloom {
    use bevy::{core_pipeline::tonemapping::Tonemapping, post_process::bloom::Bloom, prelude::*};

    const CURRENT_SCENE: super::Scene = super::Scene::Bloom;

    pub fn setup(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        commands.spawn((
            Camera3d::default(),
            Tonemapping::TonyMcMapface,
            Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            Bloom::NATURAL,
            DespawnOnExit(CURRENT_SCENE),
        ));

        let material_emissive1 = materials.add(StandardMaterial {
            emissive: LinearRgba::rgb(13.99, 5.32, 2.0),
            ..default()
        });
        let material_emissive2 = materials.add(StandardMaterial {
            emissive: LinearRgba::rgb(2.0, 13.99, 5.32),
            ..default()
        });

        let mesh = meshes.add(Sphere::new(0.5).mesh().ico(5).unwrap());

        for z in -2..3_i32 {
            let material = match (z % 2).abs() {
                0 => material_emissive1.clone(),
                1 => material_emissive2.clone(),
                _ => unreachable!(),
            };

            commands.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material),
                Transform::from_xyz(z as f32 * 2.0, 0.0, 0.0),
                DespawnOnExit(CURRENT_SCENE),
            ));
        }
    }
}

mod gltf {
    use bevy::prelude::*;

    const CURRENT_SCENE: super::Scene = super::Scene::Gltf;

    pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.spawn((
            Camera3d::default(),
            Transform::from_xyz(0.7, 0.7, 1.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
            EnvironmentMapLight {
                diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
                specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
                intensity: 250.0,
                ..default()
            },
            DespawnOnExit(CURRENT_SCENE),
        ));

        commands.spawn((
            DirectionalLight {
                shadow_maps_enabled: true,
                ..default()
            },
            DespawnOnExit(CURRENT_SCENE),
        ));
        commands.spawn((
            WorldAssetRoot(asset_server.load(
                GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf"),
            )),
            DespawnOnExit(CURRENT_SCENE),
        ));
    }
}

mod animation {
    use std::{f32::consts::PI, time::Duration};

    use bevy::{prelude::*, world_serialization::WorldInstanceReady};

    const CURRENT_SCENE: super::Scene = super::Scene::Animation;
    const FOX_PATH: &str = "models/animated/Fox.glb";

    #[derive(Resource)]
    struct Animation {
        animation: AnimationNodeIndex,
        graph: Handle<AnimationGraph>,
    }

    pub fn setup(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        mut graphs: ResMut<Assets<AnimationGraph>>,
    ) {
        let (graph, node) = AnimationGraph::from_clip(
            asset_server.load(GltfAssetLabel::Animation(2).from_asset(FOX_PATH)),
        );

        let graph_handle = graphs.add(graph);
        commands.insert_resource(Animation {
            animation: node,
            graph: graph_handle,
        });

        commands.spawn((
            Camera3d::default(),
            Transform::from_xyz(100.0, 100.0, 150.0).looking_at(Vec3::new(0.0, 20.0, 0.0), Vec3::Y),
            DespawnOnExit(CURRENT_SCENE),
        ));

        commands.spawn((
            Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.)),
            DirectionalLight {
                shadow_maps_enabled: true,
                ..default()
            },
            DespawnOnExit(CURRENT_SCENE),
        ));

        commands
            .spawn((
                WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(FOX_PATH))),
                DespawnOnExit(CURRENT_SCENE),
            ))
            .observe(pause_animation_frame);
    }

    fn pause_animation_frame(
        scene_ready: On<WorldInstanceReady>,
        children: Query<&Children>,
        mut commands: Commands,
        animation: Res<Animation>,
        mut players: Query<(Entity, &mut AnimationPlayer)>,
    ) {
        for child in children.iter_descendants(scene_ready.entity) {
            if let Ok((entity, mut player)) = players.get_mut(child) {
                let mut transitions = AnimationTransitions::new();
                transitions
                    .play(&mut player, animation.animation, Duration::ZERO)
                    .seek_to(0.5)
                    .pause();

                commands
                    .entity(entity)
                    .insert(AnimationGraphHandle(animation.graph.clone()))
                    .insert(transitions);
            }
        }
    }
}

mod gizmos {
    use bevy::{color::palettes::css::*, prelude::*};

    pub fn setup(mut commands: Commands) {
        commands.spawn((
            Camera3d::default(),
            Transform::from_xyz(-1.0, 2.5, 6.5).looking_at(Vec3::ZERO, Vec3::Y),
            DespawnOnExit(super::Scene::Gizmos),
        ));
    }

    pub fn draw_gizmos(mut gizmos: Gizmos) {
        gizmos.cube(
            Transform::from_translation(Vec3::X * -1.75).with_scale(Vec3::splat(1.25)),
            RED,
        );
        gizmos
            .sphere(Isometry3d::from_translation(Vec3::X * -3.5), 0.75, GREEN)
            .resolution(30_000 / 3);

        gizmos.text(
            Isometry3d::from_translation(Vec3::Y * 1.5),
            "text gizmo",
            0.3,
            Vec2 { x: 0., y: 0. },
            Color::WHITE,
        );

        // 3d grids with all variations of outer edges on or off
        for i in 0..8 {
            let x = 1.5 * (i % 4) as f32;
            let y = 1.0 * (0.5 - (i / 4) as f32);
            let mut grid = gizmos.grid_3d(
                Isometry3d::from_translation(Vec3::new(x, y, 0.0)),
                UVec3::new(5, 4, 3),
                Vec3::splat(0.175),
                Color::WHITE,
            );
            if i & 1 > 0 {
                grid = grid.outer_edges_x();
            }
            if i & 2 > 0 {
                grid = grid.outer_edges_y();
            }
            if i & 4 > 0 {
                grid.outer_edges_z();
            }
        }
    }
}

mod gltf_coordinate_conversion {
    use bevy::{
        color::palettes::basic::*,
        gltf::{convert_coordinates::GltfConvertCoordinates, GltfLoaderSettings},
        prelude::*,
        world_serialization::WorldInstanceReady,
    };

    const CURRENT_SCENE: super::Scene = super::Scene::GltfCoordinateConversion;

    pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.spawn((
            Camera3d::default(),
            Transform::from_xyz(-4.0, 4.0, -5.0).looking_at(Vec3::ZERO, Vec3::Y),
            DespawnOnExit(CURRENT_SCENE),
        ));

        commands.spawn((
            DirectionalLight {
                color: BLUE.into(),
                ..default()
            },
            Transform::IDENTITY.looking_to(Dir3::Z, Dir3::Y),
            DespawnOnExit(CURRENT_SCENE),
        ));

        commands.spawn((
            DirectionalLight {
                color: RED.into(),
                ..default()
            },
            Transform::IDENTITY.looking_to(Dir3::X, Dir3::Y),
            DespawnOnExit(CURRENT_SCENE),
        ));

        commands.spawn((
            DirectionalLight {
                color: GREEN.into(),
                ..default()
            },
            Transform::IDENTITY.looking_to(Dir3::NEG_Y, Dir3::X),
            DespawnOnExit(CURRENT_SCENE),
        ));

        commands
            .spawn((
                WorldAssetRoot(asset_server.load_with_settings(
                    GltfAssetLabel::Scene(0).from_asset("models/Faces/faces.glb"),
                    |s: &mut GltfLoaderSettings| {
                        s.convert_coordinates = Some(GltfConvertCoordinates {
                            rotate_scene_entity: true,
                            rotate_meshes: true,
                        });
                    },
                )),
                DespawnOnExit(CURRENT_SCENE),
            ))
            .observe(show_aabbs);
    }

    pub fn show_aabbs(
        scene_ready: On<WorldInstanceReady>,
        mut commands: Commands,
        children: Query<&Children>,
        meshes: Query<(), With<Mesh3d>>,
    ) {
        for child in children
            .iter_descendants(scene_ready.entity)
            .filter(|&e| meshes.contains(e))
        {
            commands.entity(child).insert(ShowAabbGizmo {
                color: Some(BLACK.into()),
            });
        }
    }

    pub fn draw_gizmos(mut gizmos: Gizmos) {
        gizmos.axes(Transform::IDENTITY, 1.0);
    }
}

mod white_furnace_solid_color_light {
    use bevy::{
        asset::RenderAssetUsages,
        camera::{Hdr, ScalingMode},
        core_pipeline::tonemapping::Tonemapping,
        light::Skybox,
        prelude::*,
        render::render_resource::{
            Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
        },
    };

    const CURRENT_SCENE: super::Scene = super::Scene::WhiteFurnaceSolidColorLight;

    /// Creates a pure white cubemap
    fn create_white_cubemap(size: u32) -> Image {
        // f16 bytes for 1.0 (white): [0, 60] in little-endian
        const WHITE_F16: [u8; 2] = [0, 60];
        const WHITE_PIXEL: [u8; 8] = [
            WHITE_F16[0],
            WHITE_F16[1], // R
            WHITE_F16[0],
            WHITE_F16[1], // G
            WHITE_F16[0],
            WHITE_F16[1], // B
            WHITE_F16[0],
            WHITE_F16[1], // A
        ];

        let pixel_data: Vec<u8> = (0..6 * size * size).flat_map(|_| WHITE_PIXEL).collect();

        Image {
            texture_view_descriptor: Some(TextureViewDescriptor {
                dimension: Some(TextureViewDimension::Cube),
                ..Default::default()
            }),
            ..Image::new(
                Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: 6,
                },
                TextureDimension::D2,
                pixel_data,
                TextureFormat::Rgba16Float,
                RenderAssetUsages::RENDER_WORLD,
            )
        }
    }

    pub fn setup(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut images: ResMut<Assets<Image>>,
    ) {
        let sphere_mesh = meshes.add(Sphere::new(0.45));

        // Light should come from the environment map only
        commands.insert_resource(GlobalAmbientLight::NONE);

        // add entities to the world
        for y in -2..=2 {
            for x in -5..=5 {
                let x01 = (x + 5) as f32 / 10.0;
                let y01 = (y + 2) as f32 / 4.0;
                // sphere
                commands.spawn((
                    Mesh3d(sphere_mesh.clone()),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: LinearRgba::WHITE.into(),
                        // vary key PBR parameters on a grid of spheres to show the effect
                        metallic: y01,
                        perceptual_roughness: x01,
                        ..default()
                    })),
                    Transform::from_xyz(x as f32, y as f32 + 0.5, 0.0),
                    DespawnOnExit(CURRENT_SCENE),
                ));
            }
        }

        // Create a pure white cubemap
        let white_cubemap = create_white_cubemap(256);
        let white_cubemap_handle = images.add(white_cubemap);

        let mut solid_color_light = EnvironmentMapLight::solid_color(&mut images, Color::WHITE);
        solid_color_light.intensity = 500.0;

        // camera
        commands.spawn((
            Camera3d::default(),
            Hdr,
            Tonemapping::None,
            Transform::from_xyz(0.0, 0.0, 8.0).looking_at(Vec3::default(), Vec3::Y),
            Projection::from(OrthographicProjection {
                scale: 0.01,
                scaling_mode: ScalingMode::WindowSize,
                ..OrthographicProjection::default_3d()
            }),
            Skybox {
                image: Some(white_cubemap_handle),
                // middle gray
                brightness: 500.0,
                ..default()
            },
            solid_color_light,
            DespawnOnExit(CURRENT_SCENE),
        ));
    }
}

mod white_furnace_environment_map_light {
    use bevy::{
        asset::RenderAssetUsages,
        camera::{Hdr, ScalingMode},
        core_pipeline::tonemapping::Tonemapping,
        light::Skybox,
        prelude::*,
        render::render_resource::{
            Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
        },
    };

    const CURRENT_SCENE: super::Scene = super::Scene::WhiteFurnaceEnvironmentMapLight;

    /// Creates a pure white cubemap
    fn create_white_cubemap(size: u32) -> Image {
        // f16 bytes for 1.0 (white): [0, 60] in little-endian
        const WHITE_F16: [u8; 2] = [0, 60];
        const WHITE_PIXEL: [u8; 8] = [
            WHITE_F16[0],
            WHITE_F16[1], // R
            WHITE_F16[0],
            WHITE_F16[1], // G
            WHITE_F16[0],
            WHITE_F16[1], // B
            WHITE_F16[0],
            WHITE_F16[1], // A
        ];

        let pixel_data: Vec<u8> = (0..6 * size * size).flat_map(|_| WHITE_PIXEL).collect();

        Image {
            texture_view_descriptor: Some(TextureViewDescriptor {
                dimension: Some(TextureViewDimension::Cube),
                ..Default::default()
            }),
            ..Image::new(
                Extent3d {
                    width: size,
                    height: size,
                    depth_or_array_layers: 6,
                },
                TextureDimension::D2,
                pixel_data,
                TextureFormat::Rgba16Float,
                RenderAssetUsages::RENDER_WORLD,
            )
        }
    }

    pub fn setup(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut images: ResMut<Assets<Image>>,
    ) {
        let sphere_mesh = meshes.add(Sphere::new(0.45));

        // Light should come from the environment map only
        commands.insert_resource(GlobalAmbientLight::NONE);

        // add entities to the world
        for y in -2..=2 {
            for x in -5..=5 {
                let x01 = (x + 5) as f32 / 10.0;
                let y01 = (y + 2) as f32 / 4.0;
                // sphere
                commands.spawn((
                    Mesh3d(sphere_mesh.clone()),
                    MeshMaterial3d(materials.add(StandardMaterial {
                        base_color: LinearRgba::WHITE.into(),
                        // vary key PBR parameters on a grid of spheres to show the effect
                        metallic: y01,
                        perceptual_roughness: x01,
                        ..default()
                    })),
                    Transform::from_xyz(x as f32, y as f32 + 0.5, 0.0),
                    DespawnOnExit(CURRENT_SCENE),
                ));
            }
        }

        // Create a pure white cubemap
        let white_cubemap = create_white_cubemap(256);
        let white_cubemap_handle = images.add(white_cubemap);

        let generated_light = GeneratedEnvironmentMapLight {
            environment_map: white_cubemap_handle.clone(),
            intensity: 500.0,
            ..default()
        };

        // camera
        commands.spawn((
            Camera3d::default(),
            Hdr,
            Tonemapping::None,
            Transform::from_xyz(0.0, 0.0, 8.0).looking_at(Vec3::default(), Vec3::Y),
            Projection::from(OrthographicProjection {
                scale: 0.01,
                scaling_mode: ScalingMode::WindowSize,
                ..OrthographicProjection::default_3d()
            }),
            Skybox {
                image: Some(white_cubemap_handle),
                // middle gray
                brightness: 500.0,
                ..default()
            },
            generated_light,
            DespawnOnExit(CURRENT_SCENE),
        ));
    }
}

mod render_layers {
    const CURRENT_SCENE: super::Scene = super::Scene::RenderLayers;

    use bevy::{
        camera::{visibility::RenderLayers, Viewport},
        prelude::*,
        window::PrimaryWindow,
    };

    pub fn setup(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        window: Single<&Window, With<PrimaryWindow>>,
    ) {
        // circular base
        commands.spawn((
            Mesh3d(meshes.add(Circle::new(4.0))),
            MeshMaterial3d(materials.add(Color::WHITE)),
            Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            RenderLayers::layer(0).with(1).with(2),
            DespawnOnExit(CURRENT_SCENE),
        ));

        // cubes
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(Color::srgb(1.0, 0.0, 0.0))),
            Transform::from_xyz(-1.5, 0.5, 0.0),
            // No render layer for this one to test the default case
            DespawnOnExit(CURRENT_SCENE),
        ));
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(Color::srgb(0.0, 1.0, 0.0))),
            Transform::from_xyz(0.0, 0.5, 0.0),
            RenderLayers::layer(1),
            DespawnOnExit(CURRENT_SCENE),
        ));
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(Color::srgb(0.0, 0.0, 1.0))),
            Transform::from_xyz(1.5, 0.5, 0.0),
            RenderLayers::layer(2),
            DespawnOnExit(CURRENT_SCENE),
        ));

        // Light
        commands.spawn((
            PointLight {
                shadow_maps_enabled: true,
                ..default()
            },
            Transform::from_xyz(4.0, 8.0, 4.0),
            DespawnOnExit(CURRENT_SCENE),
        ));

        let window_half_size = window.physical_size() / 2;

        // Split the screen in 4 different viewports with each of them having a specific render
        // layer
        for index in 0..4 {
            let viewport_pos = UVec2::new((index % 2) as u32, (index / 2) as u32);
            let mut entity_cmds = commands.spawn((
                Camera3d::default(),
                Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
                Camera {
                    // Renders cameras with different priorities to prevent ambiguities
                    order: index as isize,
                    viewport: Some(Viewport {
                        physical_position: viewport_pos * window_half_size,
                        physical_size: window_half_size,
                        ..default()
                    }),
                    ..default()
                },
                DespawnOnExit(CURRENT_SCENE),
            ));
            match index {
                0 => {}
                1 => {
                    entity_cmds.insert(RenderLayers::layer(1));
                }
                2 => {
                    entity_cmds.insert(RenderLayers::layer(2));
                }
                3 => {
                    entity_cmds.insert(RenderLayers::layer(0).with(1).with(2));
                }
                _ => warn!("Unexpected index {index}"),
            }
        }
    }
}
