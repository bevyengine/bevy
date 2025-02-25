//! Create invalid skinned meshes to test renderer behaviour.

use bevy::{
    core_pipeline::{
        motion_blur::MotionBlur,
        prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
    },
    math::ops,
    pbr::DefaultOpaqueRendererMethod,
    prelude::*,
    render::{
        camera::ScalingMode,
        mesh::{
            skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
            Indices, PrimitiveTopology, VertexAttributeValues,
        },
        render_asset::RenderAssetUsages,
    },
};
use std::f32::consts::TAU;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight {
            brightness: 20_000.0,
            ..default()
        })
        .insert_resource(Globals::default())
        .add_systems(Startup, (setup_environment, setup_meshes))
        .add_systems(
            Update,
            (
                update_animated_joints,
                update_render_mode,
                update_motion_blur,
                update_text,
            ),
        )
        .run();
}

fn setup_environment(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 1.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::AutoMin {
                min_width: 18.0,
                min_height: 6.0,
            },
            ..OrthographicProjection::default_3d()
        }),
        default_motion_blur(),
        // MSAA is incompatible with deferred rendering.
        Msaa::Off,
    ));

    // Add a directional light to make sure we exercise the renderer's lighting path.
    commands.spawn((
        Transform::from_xyz(1.0, 1.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
    ));

    // Add a plane behind the skinned meshes so that we can see their shadows.
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, -1.0),
        Mesh3d(mesh_assets.add(Plane3d::default().mesh().size(100.0, 100.0).normal(Dir3::Z))),
        MeshMaterial3d(material_assets.add(StandardMaterial {
            base_color: Color::srgb(0.1 * 0.5, 0.3 * 0.5, 0.1 * 0.5),
            reflectance: 0.2,
            ..default()
        })),
    ));
}

fn setup_meshes(
    mut commands: Commands,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    mut inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
) {
    let unskinned_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        vec![
            [-0.35, -0.35, 0.0],
            [0.35, -0.35, 0.0],
            [-0.35, 0.35, 0.0],
            [0.35, 0.35, 0.0],
            [-0.5, 1.0, 0.0],
            [0.5, 1.0, 0.0],
            [-0.5, 2.0, 0.0],
            [0.5, 2.0, 0.0],
        ],
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, 1.0]; 8]);

    let skinned_mesh = unskinned_mesh
        .clone()
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            VertexAttributeValues::Uint16x4(vec![
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [0, 0, 0, 0],
                [1, 0, 0, 0],
                [1, 0, 0, 0],
                [1, 0, 0, 0],
                [1, 0, 0, 0],
            ]),
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_JOINT_WEIGHT,
            vec![[1.00, 0.00, 0.0, 0.0]; 8],
        )
        .with_inserted_indices(Indices::U16(vec![0, 1, 3, 0, 3, 2, 4, 5, 7, 4, 7, 6]));

    let unskinned_mesh_handle = mesh_assets.add(unskinned_mesh);
    let skinned_mesh_handle = mesh_assets.add(skinned_mesh);

    let inverse_bindposes_handle = inverse_bindposes_assets.add(vec![
        Mat4::IDENTITY,
        Mat4::from_translation(Vec3::new(0.0, -1.5, 0.0)),
    ]);

    let material_handle = material_assets.add(StandardMaterial {
        cull_mode: None,
        ..default()
    });

    // Mesh 0: Normal.
    // Mesh 1: Asset is missing joint index and joint weight attributes.
    // Mesh 2: Entity is missing SkinnedMesh component.
    // Mesh 3: One joint entity deleted.

    for mesh_index in 0..4 {
        let transform = Transform::from_xyz(((mesh_index as f32) - 1.5) * 4.0, 0.0, 0.0);

        let joint_0 = commands.spawn(transform).id();

        let joint_1 = commands
            .spawn((ChildOf(joint_0), AnimatedJoint, Transform::IDENTITY))
            .id();

        let mesh_handle = match mesh_index {
            1 => &unskinned_mesh_handle,
            _ => &skinned_mesh_handle,
        };

        let mut entity_commands = commands.spawn((
            Mesh3d(mesh_handle.clone()),
            MeshMaterial3d(material_handle.clone()),
            transform,
        ));

        if mesh_index != 2 {
            entity_commands.insert(SkinnedMesh {
                inverse_bindposes: inverse_bindposes_handle.clone(),
                joints: vec![joint_0, joint_1],
            });
        }

        if mesh_index == 3 {
            commands.entity(joint_1).despawn();
        }
    }
}

fn default_motion_blur() -> MotionBlur {
    MotionBlur {
        // Use an unrealistically large shutter angle so that motion blur is clearly visible.
        shutter_angle: 4.0,
        samples: 2,
        #[cfg(all(feature = "webgl2", target_arch = "wasm32", not(feature = "webgpu")))]
        _webgl2_padding: Default::default(),
    }
}

#[derive(Component)]
struct AnimatedJoint;

fn update_animated_joints(time: Res<Time>, mut query: Query<(&mut Transform, &AnimatedJoint)>) {
    for (mut transform, _) in &mut query {
        let angle = TAU * 4.0 * ops::cos((time.elapsed_secs() / 8.0) * TAU);
        let rotation = Quat::from_rotation_z(angle);

        transform.rotation = rotation;
        transform.translation = rotation.mul_vec3(Vec3::new(0.0, 1.5, 0.0));
    }
}

#[derive(Default, Debug, Copy, Clone, PartialEq)]
enum RenderMode {
    #[default]
    Forward,
    ForwardPrepass,
    Deferred,
}

impl RenderMode {
    fn from_cycle(cycle: u32) -> Self {
        match cycle % 3 {
            0 => RenderMode::Forward,
            1 => RenderMode::ForwardPrepass,
            _ => RenderMode::Deferred,
        }
    }
}

#[derive(Resource)]
struct Globals {
    cycle_render_mode: bool,
    render_mode: RenderMode,
    motion_blur: bool,
}

impl Default for Globals {
    fn default() -> Self {
        Globals {
            cycle_render_mode: true,
            render_mode: RenderMode::default(),
            motion_blur: true,
        }
    }
}

fn update_render_mode(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    cameras: Query<Entity, With<Camera>>,
    mut globals: ResMut<Globals>,
    mut default_opaque_renderer_method: ResMut<DefaultOpaqueRendererMethod>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // By default we cycle through rendering modes over time. If the user chooses
    // a particular mode then we stop cycling.

    let mut desired_render_mode = globals.render_mode;

    if keys.just_pressed(KeyCode::Digit1) {
        desired_render_mode = RenderMode::Forward;
        globals.cycle_render_mode = false;
    }

    if keys.just_pressed(KeyCode::Digit2) {
        desired_render_mode = RenderMode::ForwardPrepass;
        globals.cycle_render_mode = false;
    }

    if keys.just_pressed(KeyCode::Digit3) {
        desired_render_mode = RenderMode::Deferred;
        globals.cycle_render_mode = false;
    }

    if globals.cycle_render_mode {
        let cycle = (time.elapsed_secs() / 4.0) as u32;
        desired_render_mode = RenderMode::from_cycle(cycle);
    }

    if globals.render_mode == desired_render_mode {
        return;
    }

    println!("Switching render mode to {:?}", desired_render_mode);

    for camera in cameras {
        commands
            .entity(camera)
            .remove::<NormalPrepass>()
            .remove::<DepthPrepass>()
            .remove::<DepthPrepass>()
            .remove::<MotionVectorPrepass>()
            .remove::<DeferredPrepass>();
    }

    match desired_render_mode {
        RenderMode::Forward => {
            default_opaque_renderer_method.set_to_forward();
        }

        RenderMode::ForwardPrepass => {
            default_opaque_renderer_method.set_to_forward();

            for camera in cameras {
                commands
                    .entity(camera)
                    .insert(DepthPrepass)
                    .insert(MotionVectorPrepass)
                    .insert(NormalPrepass);
            }
        }

        RenderMode::Deferred => {
            default_opaque_renderer_method.set_to_deferred();

            for camera in cameras {
                commands
                    .entity(camera)
                    .insert(DepthPrepass)
                    .insert(DeferredPrepass)
                    .insert(MotionVectorPrepass);
            }
        }
    }

    globals.render_mode = desired_render_mode;

    // If this is left out then motion blur doesn't work in deferred render mode. TODO?
    for _ in materials.iter_mut() {}
}

fn update_motion_blur(
    keys: Res<ButtonInput<KeyCode>>,
    cameras: Query<(Entity, &mut MotionBlur), With<Camera>>,
    mut globals: ResMut<Globals>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        globals.motion_blur = !globals.motion_blur;

        for (_, mut motion_blur) in cameras {
            motion_blur.samples = if globals.motion_blur {
                default_motion_blur().samples
            } else {
                0
            };
        }
    }
}

fn update_text(mut text: Single<&mut Text>, globals: Res<Globals>) {
    text.clear();

    text.push_str(&format!(
        "{:?}, motion blur {}\n\n",
        globals.render_mode,
        match globals.motion_blur {
            true => "on",
            false => "off",
        }
    ));

    text.push_str("(1) Forward\n");
    text.push_str("(2) ForwardPrepass\n");
    text.push_str("(3) Deferred\n");
    text.push_str("(M) Toggle motion blur");
}
