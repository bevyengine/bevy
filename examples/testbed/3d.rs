//! 3d testbed
//!
//! You can switch scene by pressing the spacebar

mod helpers;

use bevy::prelude::*;
use helpers::Next;

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins,))
        .init_state::<Scene>()
        .add_systems(OnEnter(Scene::Light), light::setup)
        .add_systems(OnEnter(Scene::Bloom), bloom::setup)
        .add_systems(OnEnter(Scene::Gltf), gltf::setup)
        .add_systems(OnEnter(Scene::Animation), animation::setup)
        .add_systems(OnEnter(Scene::Gizmos), gizmos::setup)
        .add_systems(OnEnter(Scene::Forward), deferred::setup)
        .add_systems(
            OnEnter(Scene::ForwardPrepass),
            (deferred::setup, deferred::forward_prepass_camera_setup).chain(),
        )
        .add_systems(
            OnEnter(Scene::Deferred),
            (deferred::setup, deferred::deferred_camera_setup).chain(),
        )
        .add_systems(
            OnEnter(Scene::RemoveForwardPrepass),
            (
                deferred::setup,
                deferred::forward_prepass_camera_setup,
                deferred::remove_prepass_timer_init,
            )
                .chain(),
        )
        .add_systems(
            OnEnter(Scene::RemoveDeferredPrepass),
            (
                deferred::setup,
                deferred::deferred_camera_setup,
                deferred::remove_prepass_timer_init,
            )
                .chain(),
        )
        .add_systems(Update, switch_scene)
        .add_systems(Update, gizmos::draw_gizmos.run_if(in_state(Scene::Gizmos)))
        .add_systems(
            Update,
            (
                deferred::remove_prepass.run_if(resource_removed::<deferred::RemovePrepassTimer>),
                deferred::remove_prepass_timer_tick
                    .run_if(resource_exists::<deferred::RemovePrepassTimer>),
            )
                .run_if(
                    in_state(Scene::RemoveDeferredPrepass)
                        .or(in_state(Scene::RemoveForwardPrepass)),
                ),
        );

    #[cfg(feature = "bevy_ci_testing")]
    app.add_systems(Update, helpers::switch_scene_in_ci::<Scene>);

    app.run();
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, States, Default)]
#[states(scoped_entities)]
enum Scene {
    #[default]
    Light,
    Bloom,
    Gltf,
    Animation,
    Gizmos,
    Forward,
    ForwardPrepass,
    Deferred,
    RemoveForwardPrepass,
    RemoveDeferredPrepass,
}

impl Next for Scene {
    fn next(&self) -> Self {
        match self {
            Scene::Light => Scene::Bloom,
            Scene::Bloom => Scene::Gltf,
            Scene::Gltf => Scene::Animation,
            Scene::Animation => Scene::Gizmos,
            Scene::Gizmos => Scene::Forward,
            Scene::Forward => Scene::ForwardPrepass,
            Scene::ForwardPrepass => Scene::Deferred,
            Scene::Deferred => Scene::RemoveForwardPrepass,
            Scene::RemoveForwardPrepass => Scene::RemoveDeferredPrepass,
            Scene::RemoveDeferredPrepass => Scene::Light,
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
            DespawnOnExitState(CURRENT_SCENE),
        ));

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::default())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: DEEP_PINK.into(),
                ..default()
            })),
            Transform::from_xyz(0.0, 1.0, 0.0),
            DespawnOnExitState(CURRENT_SCENE),
        ));

        commands.spawn((
            PointLight {
                intensity: 100_000.0,
                color: RED.into(),
                shadows_enabled: true,
                ..default()
            },
            Transform::from_xyz(1.0, 2.0, 0.0),
            DespawnOnExitState(CURRENT_SCENE),
        ));

        commands.spawn((
            SpotLight {
                intensity: 100_000.0,
                color: LIME.into(),
                shadows_enabled: true,
                inner_angle: 0.6,
                outer_angle: 0.8,
                ..default()
            },
            Transform::from_xyz(-1.0, 2.0, 0.0).looking_at(Vec3::new(-1.0, 0.0, 0.0), Vec3::Z),
            DespawnOnExitState(CURRENT_SCENE),
        ));

        commands.spawn((
            DirectionalLight {
                illuminance: light_consts::lux::OVERCAST_DAY,
                shadows_enabled: true,
                ..default()
            },
            Transform {
                translation: Vec3::new(0.0, 2.0, 0.0),
                rotation: Quat::from_rotation_x(-PI / 4.),
                ..default()
            },
            DespawnOnExitState(CURRENT_SCENE),
        ));

        commands.spawn((
            Camera3d::default(),
            Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            DespawnOnExitState(CURRENT_SCENE),
        ));
    }
}

mod bloom {
    use bevy::{
        core_pipeline::{bloom::Bloom, tonemapping::Tonemapping},
        prelude::*,
    };

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
            DespawnOnExitState(CURRENT_SCENE),
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
                DespawnOnExitState(CURRENT_SCENE),
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
            DespawnOnExitState(CURRENT_SCENE),
        ));

        commands.spawn((
            DirectionalLight {
                shadows_enabled: true,
                ..default()
            },
            DespawnOnExitState(CURRENT_SCENE),
        ));
        commands.spawn((
            SceneRoot(asset_server.load(
                GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf"),
            )),
            DespawnOnExitState(CURRENT_SCENE),
        ));
    }
}

mod animation {
    use std::{f32::consts::PI, time::Duration};

    use bevy::{prelude::*, scene::SceneInstanceReady};

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
            DespawnOnExitState(CURRENT_SCENE),
        ));

        commands.spawn((
            Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.)),
            DirectionalLight {
                shadows_enabled: true,
                ..default()
            },
            DespawnOnExitState(CURRENT_SCENE),
        ));

        commands
            .spawn((
                SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(FOX_PATH))),
                DespawnOnExitState(CURRENT_SCENE),
            ))
            .observe(pause_animation_frame);
    }

    fn pause_animation_frame(
        trigger: Trigger<SceneInstanceReady>,
        children: Query<&Children>,
        mut commands: Commands,
        animation: Res<Animation>,
        mut players: Query<(Entity, &mut AnimationPlayer)>,
    ) {
        for child in children.iter_descendants(trigger.target()) {
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
            Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            DespawnOnExitState(super::Scene::Gizmos),
        ));
    }

    pub fn draw_gizmos(mut gizmos: Gizmos) {
        gizmos.cuboid(
            Transform::from_translation(Vec3::X * 2.0).with_scale(Vec3::splat(2.0)),
            RED,
        );
        gizmos
            .sphere(Isometry3d::from_translation(Vec3::X * -2.0), 1.0, GREEN)
            .resolution(30_000 / 3);
    }
}

mod deferred {
    use bevy::{
        anti_aliasing::fxaa::Fxaa,
        asset::{AssetServer, Assets},
        color::{Color, Srgba},
        core_pipeline::prepass::{
            DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass,
        },
        gltf::GltfAssetLabel,
        image::ImageLoaderSettings,
        math::{EulerRot, Quat, Vec3},
        pbr::{
            CascadeShadowConfigBuilder, DirectionalLight, DistanceFog, FogFalloff, MeshMaterial3d,
            NotShadowCaster, NotShadowReceiver, OpaqueRendererMethod, ParallaxMappingMethod,
            PointLight, StandardMaterial,
        },
        prelude::{
            Camera, Camera3d, Commands, Component, Cuboid, Deref, DerefMut, Entity,
            EnvironmentMapLight, Mesh, Mesh3d, Meshable, Msaa, Plane3d, Res, ResMut, Resource,
            Single, Sphere, State, Transform, With,
        },
        scene::SceneRoot,
        state::state_scoped::DespawnOnExitState,
        time::{Time, Timer},
        utils::default,
    };

    #[derive(Resource, Deref, DerefMut)]
    pub struct RemovePrepassTimer(Timer);

    #[derive(Component)]
    pub struct ParallaxCube;

    pub fn setup(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut meshes: ResMut<Assets<Mesh>>,
        scene: Res<State<super::Scene>>,
    ) {
        commands.spawn((
            Camera3d::default(),
            Camera {
                // Deferred both supports both hdr: true and hdr: false
                hdr: false,
                ..default()
            },
            Transform::from_xyz(0.7, 0.7, 1.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
            // MSAA needs to be off for Deferred rendering
            Msaa::Off,
            DistanceFog {
                color: Color::srgb_u8(43, 44, 47),
                falloff: FogFalloff::Linear {
                    start: 1.0,
                    end: 8.0,
                },
                ..default()
            },
            EnvironmentMapLight {
                diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
                specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
                intensity: 2000.0,
                ..default()
            },
            Fxaa::default(),
            DespawnOnExitState(*scene.get()),
        ));

        commands.spawn((
            DirectionalLight {
                illuminance: 15_000.,
                shadows_enabled: true,
                ..default()
            },
            CascadeShadowConfigBuilder {
                num_cascades: 3,
                maximum_distance: 10.0,
                ..default()
            }
            .build(),
            Transform::from_rotation(Quat::from_euler(
                EulerRot::ZYX,
                0.0,
                0.0,
                -std::f32::consts::FRAC_PI_4,
            )),
            DespawnOnExitState(*scene.get()),
        ));

        // FlightHelmet
        let helmet_scene = asset_server
            .load(GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf"));

        commands.spawn((SceneRoot(helmet_scene), DespawnOnExitState(*scene.get())));

        let mut forward_mat: StandardMaterial = Color::srgb(0.1, 0.2, 0.1).into();
        forward_mat.opaque_render_method = OpaqueRendererMethod::Forward;
        let forward_mat_h = materials.add(forward_mat);

        // Plane
        commands.spawn((
            Mesh3d(meshes.add(Plane3d::default().mesh().size(50.0, 50.0))),
            MeshMaterial3d(forward_mat_h.clone()),
            DespawnOnExitState(*scene.get()),
        ));

        let cube_h = meshes.add(Cuboid::new(0.1, 0.1, 0.1));
        let sphere_h = meshes.add(Sphere::new(0.125).mesh().uv(32, 18));

        // Cubes
        commands.spawn((
            Mesh3d(cube_h.clone()),
            MeshMaterial3d(forward_mat_h.clone()),
            Transform::from_xyz(-0.3, 0.5, -0.2),
            DespawnOnExitState(*scene.get()),
        ));
        commands.spawn((
            Mesh3d(cube_h),
            MeshMaterial3d(forward_mat_h),
            Transform::from_xyz(0.2, 0.5, 0.2),
            DespawnOnExitState(*scene.get()),
        ));

        let sphere_color = Color::srgb(10.0, 4.0, 1.0);
        let sphere_pos = Transform::from_xyz(0.4, 0.5, -0.8);
        // Emissive sphere
        let mut unlit_mat: StandardMaterial = sphere_color.into();
        unlit_mat.unlit = true;
        commands.spawn((
            Mesh3d(sphere_h.clone()),
            MeshMaterial3d(materials.add(unlit_mat)),
            sphere_pos,
            NotShadowCaster,
            DespawnOnExitState(*scene.get()),
        ));
        // Light
        commands.spawn((
            PointLight {
                intensity: 800.0,
                radius: 0.125,
                shadows_enabled: true,
                color: sphere_color,
                ..default()
            },
            sphere_pos,
            DespawnOnExitState(*scene.get()),
        ));

        // sky
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(2.0, 1.0, 1.0))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Srgba::hex("888888").unwrap().into(),
                unlit: true,
                cull_mode: None,
                ..default()
            })),
            Transform::from_scale(Vec3::splat(1_000_000.0)),
            NotShadowCaster,
            NotShadowReceiver,
            DespawnOnExitState(*scene.get()),
        ));

        // The normal map. Note that to generate it in the GIMP image editor, you should
        // open the depth map, and do Filters → Generic → Normal Map
        // You should enable the "flip X" checkbox.
        let normal_handle = asset_server.load_with_settings(
            "textures/parallax_example/cube_normal.png",
            // The normal map texture is in linear color space. Lighting won't look correct
            // if `is_srgb` is `true`, which is the default.
            |settings: &mut ImageLoaderSettings| settings.is_srgb = false,
        );

        let mut cube = Mesh::from(Cuboid::new(0.15, 0.15, 0.15));

        // NOTE: for normal maps and depth maps to work, the mesh
        // needs tangents generated.
        cube.generate_tangents().unwrap();

        let parallax_material = materials.add(StandardMaterial {
            perceptual_roughness: 0.4,
            base_color_texture: Some(asset_server.load("textures/parallax_example/cube_color.png")),
            normal_map_texture: Some(normal_handle),
            // The depth map is a grayscale texture where black is the highest level and
            // white the lowest.
            depth_map: Some(asset_server.load("textures/parallax_example/cube_depth.png")),
            parallax_depth_scale: 0.09,
            parallax_mapping_method: ParallaxMappingMethod::Relief { max_steps: 4 },
            max_parallax_layer_count: bevy::math::ops::exp2(5.0f32),
            ..default()
        });
        commands.spawn((
            Mesh3d(meshes.add(cube)),
            MeshMaterial3d(parallax_material),
            Transform::from_xyz(0.4, 0.2, -0.8),
            ParallaxCube,
            DespawnOnExitState(*scene.get()),
        ));
    }

    pub fn forward_prepass_camera_setup(
        mut commands: Commands,
        camera: Single<Entity, With<Camera>>,
    ) {
        commands
            .entity(*camera)
            .insert((NormalPrepass, DepthPrepass, MotionVectorPrepass));
    }

    pub fn deferred_camera_setup(mut commands: Commands, camera: Single<Entity, With<Camera>>) {
        commands
            .entity(*camera)
            .insert((DepthPrepass, MotionVectorPrepass, DeferredPrepass));
    }

    pub fn remove_prepass_timer_init(mut commands: Commands) {
        commands.insert_resource(RemovePrepassTimer(Timer::from_seconds(
            0.5,
            bevy::time::TimerMode::Once,
        )));
    }

    pub fn remove_prepass_timer_tick(
        mut commands: Commands,
        time: Res<Time>,
        mut timer: ResMut<RemovePrepassTimer>,
    ) {
        timer.tick(time.delta());
        if timer.just_finished() {
            commands.remove_resource::<RemovePrepassTimer>();
        }
    }

    /// Remove prepass components from camera
    pub fn remove_prepass(
        mut commands: Commands,
        camera: Single<Entity, With<Camera>>,
        mut parallax_cube: Single<&mut Transform, With<ParallaxCube>>,
        scene: Res<State<super::Scene>>,
    ) {
        match scene.get() {
            super::Scene::RemoveForwardPrepass => {
                commands
                    .entity(*camera)
                    .remove::<NormalPrepass>()
                    .remove::<DepthPrepass>()
                    .remove::<MotionVectorPrepass>();
            }
            super::Scene::RemoveDeferredPrepass => {
                commands
                    .entity(*camera)
                    .remove::<DepthPrepass>()
                    .remove::<MotionVectorPrepass>()
                    .remove::<DeferredPrepass>();
            }
            _ => unreachable!("This system should only run on Scene::RemoveForwardPrepass or Scene::RemoveDeferredPrepass"),
        }
        parallax_cube.rotate_z(std::f32::consts::FRAC_PI_3);
    }
}
