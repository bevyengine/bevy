//! 3d testbed
//!
//! You can switch scene by pressing the spacebar

#[cfg(feature = "bevy_ci_testing")]
use bevy::dev_tools::ci_testing::CiTestingCustomEvent;
use bevy::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins,))
        .init_state::<Scene>()
        .enable_state_scoped_entities::<Scene>()
        .add_systems(OnEnter(Scene::Light), light::setup)
        .add_systems(OnEnter(Scene::Animation), animation::setup)
        .add_systems(Update, switch_scene);

    // Those scenes don't work in CI on Windows runners
    #[cfg(not(all(feature = "bevy_ci_testing", target_os = "windows")))]
    app.add_systems(OnEnter(Scene::Bloom), bloom::setup)
        .add_systems(OnEnter(Scene::Gltf), gltf::setup);

    app.run();
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
enum Scene {
    #[default]
    Light,
    Bloom,
    Gltf,
    Animation,
}

fn switch_scene(
    keyboard: Res<ButtonInput<KeyCode>>,
    #[cfg(feature = "bevy_ci_testing")] mut ci_events: EventReader<CiTestingCustomEvent>,
    scene: Res<State<Scene>>,
    mut next_scene: ResMut<NextState<Scene>>,
) {
    let mut should_switch = false;
    should_switch |= keyboard.just_pressed(KeyCode::Space);
    #[cfg(feature = "bevy_ci_testing")]
    {
        should_switch |= ci_events.read().any(|event| match event {
            CiTestingCustomEvent(event) => event == "switch_scene",
        });
    }
    if should_switch {
        info!("Switching scene");
        next_scene.set(match scene.get() {
            Scene::Light => Scene::Bloom,
            Scene::Bloom => Scene::Gltf,
            Scene::Gltf => Scene::Animation,
            Scene::Animation => Scene::Light,
        });
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
            StateScoped(CURRENT_SCENE),
        ));

        commands.spawn((
            Mesh3d(meshes.add(Cuboid::default())),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: DEEP_PINK.into(),
                ..default()
            })),
            Transform::from_xyz(0.0, 1.0, 0.0),
            StateScoped(CURRENT_SCENE),
        ));

        commands.spawn((
            PointLight {
                intensity: 100_000.0,
                color: RED.into(),
                shadows_enabled: true,
                ..default()
            },
            Transform::from_xyz(1.0, 2.0, 0.0),
            StateScoped(CURRENT_SCENE),
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
            StateScoped(CURRENT_SCENE),
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
            StateScoped(CURRENT_SCENE),
        ));

        commands.spawn((
            Camera3d::default(),
            Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            StateScoped(CURRENT_SCENE),
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
            Camera {
                hdr: true,
                ..default()
            },
            Tonemapping::TonyMcMapface,
            Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            Bloom::NATURAL,
            StateScoped(CURRENT_SCENE),
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
                StateScoped(CURRENT_SCENE),
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
            StateScoped(CURRENT_SCENE),
        ));

        commands.spawn((
            DirectionalLight {
                shadows_enabled: true,
                ..default()
            },
            StateScoped(CURRENT_SCENE),
        ));
        commands.spawn((
            SceneRoot(asset_server.load(
                GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf"),
            )),
            StateScoped(CURRENT_SCENE),
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
            StateScoped(CURRENT_SCENE),
        ));

        commands.spawn((
            Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.)),
            DirectionalLight {
                shadows_enabled: true,
                ..default()
            },
            StateScoped(CURRENT_SCENE),
        ));

        commands
            .spawn((
                SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(FOX_PATH))),
                StateScoped(CURRENT_SCENE),
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
        let entity = children.get(trigger.entity()).unwrap()[0];
        let entity = children.get(entity).unwrap()[0];

        let (entity, mut player) = players.get_mut(entity).unwrap();
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
