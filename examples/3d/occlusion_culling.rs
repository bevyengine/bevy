//! Demonstrates occlusion culling.
//!
//! This demo rotates many small cubes around a rotating large cube at the
//! origin. At all times, the large cube will be occluding several of the small
//! cubes. The demo displays the number of cubes that were actually rendered, so
//! the effects of occlusion culling can be seen.

use std::{
    any::TypeId,
    f32::consts::PI,
    fmt::Write as _,
    result::Result,
    sync::{Arc, Mutex},
};

use bevy::{
    color::palettes::css::{SILVER, WHITE},
    core_pipeline::{
        core_3d::{
            graph::{Core3d, Node3d},
            Opaque3d,
        },
        prepass::DepthPrepass,
    },
    pbr::PbrPlugin,
    prelude::*,
    render::{
        batching::gpu_preprocessing::{
            GpuPreprocessingSupport, IndirectParametersBuffers, IndirectParametersIndexed,
        },
        experimental::occlusion_culling::OcclusionCulling,
        render_graph::{self, NodeRunError, RenderGraphContext, RenderGraphExt, RenderLabel},
        render_resource::{Buffer, BufferDescriptor, BufferUsages, MapMode},
        renderer::{RenderContext, RenderDevice},
        settings::WgpuFeatures,
        Render, RenderApp, RenderDebugFlags, RenderPlugin, RenderStartup, RenderSystems,
    },
};
use bytemuck::Pod;

/// The radius of the spinning sphere of cubes.
const OUTER_RADIUS: f32 = 3.0;

/// The density of cubes in the other sphere.
const OUTER_SUBDIVISION_COUNT: u32 = 5;

/// The speed at which the outer sphere and large cube rotate in radians per
/// frame.
const ROTATION_SPEED: f32 = 0.01;

/// The length of each side of the small cubes, in meters.
const SMALL_CUBE_SIZE: f32 = 0.1;

/// The length of each side of the large cube, in meters.
const LARGE_CUBE_SIZE: f32 = 2.0;

/// A marker component for the immediate parent of the large sphere of cubes.
#[derive(Default, Component)]
struct SphereParent;

/// A marker component for the large spinning cube at the origin.
#[derive(Default, Component)]
struct LargeCube;

/// A plugin for the render app that reads the number of culled meshes from the
/// GPU back to the CPU.
struct ReadbackIndirectParametersPlugin;

/// The node that we insert into the render graph in order to read the number of
/// culled meshes from the GPU back to the CPU.
#[derive(Default)]
struct ReadbackIndirectParametersNode;

/// The [`RenderLabel`] that we use to identify the
/// [`ReadbackIndirectParametersNode`].
#[derive(Clone, PartialEq, Eq, Hash, Debug, RenderLabel)]
struct ReadbackIndirectParameters;

/// The intermediate staging buffers that we use to read back the indirect
/// parameters from the GPU to the CPU.
///
/// We read back the GPU indirect parameters so that we can determine the number
/// of meshes that were culled.
///
/// `wgpu` doesn't allow us to read indirect buffers back from the GPU to the
/// CPU directly. Instead, we have to copy them to a temporary staging buffer
/// first, and then read *those* buffers back from the GPU to the CPU. This
/// resource holds those temporary buffers.
#[derive(Resource, Default)]
struct IndirectParametersStagingBuffers {
    /// The buffer that stores the indirect draw commands.
    ///
    /// See [`IndirectParametersIndexed`] for more information about the memory
    /// layout of this buffer.
    data: Option<Buffer>,
    /// The buffer that stores the *number* of indirect draw commands.
    ///
    /// We only care about the first `u32` in this buffer.
    batch_sets: Option<Buffer>,
}

/// A resource, shared between the main world and the render world, that saves a
/// CPU-side copy of the GPU buffer that stores the indirect draw parameters.
///
/// This is needed so that we can display the number of meshes that were culled.
/// It's reference counted, and protected by a lock, because we don't precisely
/// know when the GPU will be ready to present the CPU with the buffer copy.
/// Even though the rendering runs at least a frame ahead of the main app logic,
/// we don't require more precise synchronization than the lock because we don't
/// really care how up-to-date the counter of culled meshes is. If it's off by a
/// few frames, that's no big deal.
#[derive(Clone, Resource, Deref, DerefMut)]
struct SavedIndirectParameters(Arc<Mutex<Option<SavedIndirectParametersData>>>);

/// A CPU-side copy of the GPU buffer that stores the indirect draw parameters.
///
/// This is needed so that we can display the number of meshes that were culled.
struct SavedIndirectParametersData {
    /// The CPU-side copy of the GPU buffer that stores the indirect draw
    /// parameters.
    data: Vec<IndirectParametersIndexed>,
    /// The CPU-side copy of the GPU buffer that stores the *number* of indirect
    /// draw parameters that we have.
    ///
    /// All we care about is the number of indirect draw parameters for a single
    /// view, so this is only one word in size.
    count: u32,
    /// True if occlusion culling is supported at all; false if it's not.
    occlusion_culling_supported: bool,
    /// True if we support inspecting the number of meshes that were culled on
    /// this platform; false if we don't.
    ///
    /// If `multi_draw_indirect_count` isn't supported, then we would have to
    /// employ a more complicated approach in order to determine the number of
    /// meshes that are occluded, and that would be out of scope for this
    /// example.
    occlusion_culling_introspection_supported: bool,
}

impl SavedIndirectParameters {
    fn new() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }
}

fn init_saved_indirect_parameters(
    render_device: Res<RenderDevice>,
    gpu_preprocessing_support: Res<GpuPreprocessingSupport>,
    saved_indirect_parameters: Res<SavedIndirectParameters>,
) {
    let mut saved_indirect_parameters = saved_indirect_parameters.0.lock().unwrap();
    *saved_indirect_parameters = Some(SavedIndirectParametersData {
        data: vec![],
        count: 0,
        occlusion_culling_supported: gpu_preprocessing_support.is_culling_supported(),
        // In order to determine how many meshes were culled, we look at the indirect count buffer
        // that Bevy only populates if the platform supports `multi_draw_indirect_count`. So, if we
        // don't have that feature, then we don't bother to display how many meshes were culled.
        occlusion_culling_introspection_supported: render_device
            .features()
            .contains(WgpuFeatures::MULTI_DRAW_INDIRECT_COUNT),
    });
}

/// The demo's current settings.
#[derive(Resource)]
struct AppStatus {
    /// Whether occlusion culling is presently enabled.
    ///
    /// By default, this is set to true.
    occlusion_culling: bool,
}

impl Default for AppStatus {
    fn default() -> Self {
        AppStatus {
            occlusion_culling: true,
        }
    }
}

fn main() {
    let render_debug_flags = RenderDebugFlags::ALLOW_COPIES_FROM_INDIRECT_PARAMETERS;

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Bevy Occlusion Culling Example".into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    debug_flags: render_debug_flags,
                    ..default()
                })
                .set(PbrPlugin {
                    debug_flags: render_debug_flags,
                    ..default()
                }),
        )
        .add_plugins(ReadbackIndirectParametersPlugin)
        .init_resource::<AppStatus>()
        .add_systems(Startup, setup)
        .add_systems(Update, spin_small_cubes)
        .add_systems(Update, spin_large_cube)
        .add_systems(Update, update_status_text)
        .add_systems(Update, toggle_occlusion_culling_on_request)
        .run();
}

impl Plugin for ReadbackIndirectParametersPlugin {
    fn build(&self, app: &mut App) {
        // Create the `SavedIndirectParameters` resource that we're going to use
        // to communicate between the thread that the GPU-to-CPU readback
        // callback runs on and the main application threads. This resource is
        // atomically reference counted. We store one reference to the
        // `SavedIndirectParameters` in the main app and another reference in
        // the render app.
        let saved_indirect_parameters = SavedIndirectParameters::new();
        app.insert_resource(saved_indirect_parameters.clone());

        // Fetch the render app.
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            // Insert another reference to the `SavedIndirectParameters`.
            .insert_resource(saved_indirect_parameters)
            // Setup the parameters in RenderStartup.
            .add_systems(RenderStartup, init_saved_indirect_parameters)
            .init_resource::<IndirectParametersStagingBuffers>()
            .add_systems(ExtractSchedule, readback_indirect_parameters)
            .add_systems(
                Render,
                create_indirect_parameters_staging_buffers
                    .in_set(RenderSystems::PrepareResourcesFlush),
            )
            // Add the node that allows us to read the indirect parameters back
            // from the GPU to the CPU, which allows us to determine how many
            // meshes were culled.
            .add_render_graph_node::<ReadbackIndirectParametersNode>(
                Core3d,
                ReadbackIndirectParameters,
            )
            // We read back the indirect parameters any time after
            // `EndMainPass`. Readback doesn't particularly need to execute
            // before `EndMainPassPostProcessing`, but we specify that anyway
            // because we want to make the indirect parameters run before
            // *something* in the graph, and `EndMainPassPostProcessing` is a
            // good a node as any other.
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EndMainPass,
                    ReadbackIndirectParameters,
                    Node3d::EndMainPassPostProcessing,
                ),
            );
    }
}

/// Spawns all the objects in the scene.
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    spawn_small_cubes(&mut commands, &mut meshes, &mut materials);
    spawn_large_cube(&mut commands, &asset_server, &mut meshes, &mut materials);
    spawn_light(&mut commands);
    spawn_camera(&mut commands);
    spawn_help_text(&mut commands);
}

/// Spawns the rotating sphere of small cubes.
fn spawn_small_cubes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // Add the cube mesh.
    let small_cube = meshes.add(Cuboid::new(
        SMALL_CUBE_SIZE,
        SMALL_CUBE_SIZE,
        SMALL_CUBE_SIZE,
    ));

    // Add the cube material.
    let small_cube_material = materials.add(StandardMaterial {
        base_color: SILVER.into(),
        ..default()
    });

    // Create the entity that the small cubes will be parented to. This is the
    // entity that we rotate.
    let sphere_parent = commands
        .spawn(Transform::from_translation(Vec3::ZERO))
        .insert(Visibility::default())
        .insert(SphereParent)
        .id();

    // Now we have to figure out where to place the cubes. To do that, we create
    // a sphere mesh, but we don't add it to the scene. Instead, we inspect the
    // sphere mesh to find the positions of its vertices, and spawn a small cube
    // at each one. That way, we end up with a bunch of cubes arranged in a
    // spherical shape.

    // Create the sphere mesh, and extract the positions of its vertices.
    let sphere = Sphere::new(OUTER_RADIUS)
        .mesh()
        .ico(OUTER_SUBDIVISION_COUNT)
        .unwrap();
    let sphere_positions = sphere.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();

    // At each vertex, create a small cube.
    for sphere_position in sphere_positions.as_float3().unwrap() {
        let sphere_position = Vec3::from_slice(sphere_position);
        let small_cube = commands
            .spawn(Mesh3d(small_cube.clone()))
            .insert(MeshMaterial3d(small_cube_material.clone()))
            .insert(Transform::from_translation(sphere_position))
            .id();
        commands.entity(sphere_parent).add_child(small_cube);
    }
}

/// Spawns the large cube at the center of the screen.
///
/// This cube rotates chaotically and occludes small cubes behind it.
fn spawn_large_cube(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    commands
        .spawn(Mesh3d(meshes.add(Cuboid::new(
            LARGE_CUBE_SIZE,
            LARGE_CUBE_SIZE,
            LARGE_CUBE_SIZE,
        ))))
        .insert(MeshMaterial3d(materials.add(StandardMaterial {
            base_color: WHITE.into(),
            base_color_texture: Some(asset_server.load("branding/icon.png")),
            ..default()
        })))
        .insert(Transform::IDENTITY)
        .insert(LargeCube);
}

// Spins the outer sphere a bit every frame.
//
// This ensures that the set of cubes that are hidden and shown varies over
// time.
fn spin_small_cubes(mut sphere_parents: Query<&mut Transform, With<SphereParent>>) {
    for mut sphere_parent_transform in &mut sphere_parents {
        sphere_parent_transform.rotate_y(ROTATION_SPEED);
    }
}

/// Spins the large cube a bit every frame.
///
/// The chaotic rotation adds a bit of randomness to the scene to better
/// demonstrate the dynamicity of the occlusion culling.
fn spin_large_cube(mut large_cubes: Query<&mut Transform, With<LargeCube>>) {
    for mut transform in &mut large_cubes {
        transform.rotate(Quat::from_euler(
            EulerRot::XYZ,
            0.13 * ROTATION_SPEED,
            0.29 * ROTATION_SPEED,
            0.35 * ROTATION_SPEED,
        ));
    }
}

/// Spawns a directional light to illuminate the scene.
fn spawn_light(commands: &mut Commands) {
    commands
        .spawn(DirectionalLight::default())
        .insert(Transform::from_rotation(Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            PI * -0.15,
            PI * -0.15,
        )));
}

/// Spawns a camera that includes the depth prepass and occlusion culling.
fn spawn_camera(commands: &mut Commands) {
    commands
        .spawn(Camera3d::default())
        .insert(Transform::from_xyz(0.0, 0.0, 9.0).looking_at(Vec3::ZERO, Vec3::Y))
        .insert(DepthPrepass)
        .insert(OcclusionCulling);
}

/// Spawns the help text at the upper left of the screen.
fn spawn_help_text(commands: &mut Commands) {
    commands.spawn((
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

impl render_graph::Node for ReadbackIndirectParametersNode {
    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // Extract the buffers that hold the GPU indirect draw parameters from
        // the world resources. We're going to read those buffers to determine
        // how many meshes were actually drawn.
        let (Some(indirect_parameters_buffers), Some(indirect_parameters_mapping_buffers)) = (
            world.get_resource::<IndirectParametersBuffers>(),
            world.get_resource::<IndirectParametersStagingBuffers>(),
        ) else {
            return Ok(());
        };

        // Get the indirect parameters buffers corresponding to the opaque 3D
        // phase, since all our meshes are in that phase.
        let Some(phase_indirect_parameters_buffers) =
            indirect_parameters_buffers.get(&TypeId::of::<Opaque3d>())
        else {
            return Ok(());
        };

        // Grab both the buffers we're copying from and the staging buffers
        // we're copying to. Remember that we can't map the indirect parameters
        // buffers directly, so we have to copy their contents to a staging
        // buffer.
        let (
            Some(indexed_data_buffer),
            Some(indexed_batch_sets_buffer),
            Some(indirect_parameters_staging_data_buffer),
            Some(indirect_parameters_staging_batch_sets_buffer),
        ) = (
            phase_indirect_parameters_buffers.indexed.data_buffer(),
            phase_indirect_parameters_buffers
                .indexed
                .batch_sets_buffer(),
            indirect_parameters_mapping_buffers.data.as_ref(),
            indirect_parameters_mapping_buffers.batch_sets.as_ref(),
        )
        else {
            return Ok(());
        };

        // Copy from the indirect parameters buffers to the staging buffers.
        render_context.command_encoder().copy_buffer_to_buffer(
            indexed_data_buffer,
            0,
            indirect_parameters_staging_data_buffer,
            0,
            indexed_data_buffer.size(),
        );
        render_context.command_encoder().copy_buffer_to_buffer(
            indexed_batch_sets_buffer,
            0,
            indirect_parameters_staging_batch_sets_buffer,
            0,
            indexed_batch_sets_buffer.size(),
        );

        Ok(())
    }
}

/// Creates the staging buffers that we use to read back the indirect parameters
/// from the GPU to the CPU.
///
/// We read the indirect parameters from the GPU to the CPU in order to display
/// the number of meshes that were culled each frame.
///
/// We need these staging buffers because `wgpu` doesn't allow us to read the
/// contents of the indirect parameters buffers directly. We must first copy
/// them from the GPU to a staging buffer, and then read the staging buffer.
fn create_indirect_parameters_staging_buffers(
    mut indirect_parameters_staging_buffers: ResMut<IndirectParametersStagingBuffers>,
    indirect_parameters_buffers: Res<IndirectParametersBuffers>,
    render_device: Res<RenderDevice>,
) {
    let Some(phase_indirect_parameters_buffers) =
        indirect_parameters_buffers.get(&TypeId::of::<Opaque3d>())
    else {
        return;
    };

    // Fetch the indirect parameters buffers that we're going to copy from.
    let (Some(indexed_data_buffer), Some(indexed_batch_set_buffer)) = (
        phase_indirect_parameters_buffers.indexed.data_buffer(),
        phase_indirect_parameters_buffers
            .indexed
            .batch_sets_buffer(),
    ) else {
        return;
    };

    // Build the staging buffers. Make sure they have the same sizes as the
    // buffers we're copying from.
    indirect_parameters_staging_buffers.data =
        Some(render_device.create_buffer(&BufferDescriptor {
            label: Some("indexed data staging buffer"),
            size: indexed_data_buffer.size(),
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
    indirect_parameters_staging_buffers.batch_sets =
        Some(render_device.create_buffer(&BufferDescriptor {
            label: Some("indexed batch set staging buffer"),
            size: indexed_batch_set_buffer.size(),
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
}

/// Updates the app status text at the top of the screen.
fn update_status_text(
    saved_indirect_parameters: Res<SavedIndirectParameters>,
    mut texts: Query<&mut Text>,
    meshes: Query<Entity, With<Mesh3d>>,
    app_status: Res<AppStatus>,
) {
    // How many meshes are in the scene?
    let total_mesh_count = meshes.iter().count();

    // Sample the rendered object count. Note that we don't synchronize beyond
    // locking the data and therefore this will value will generally at least
    // one frame behind. This is fine; this app is just a demonstration after
    // all.
    let (
        rendered_object_count,
        occlusion_culling_supported,
        occlusion_culling_introspection_supported,
    ): (u32, bool, bool) = {
        let saved_indirect_parameters = saved_indirect_parameters.lock().unwrap();
        let Some(saved_indirect_parameters) = saved_indirect_parameters.as_ref() else {
            // Bail out early if the resource isn't initialized yet.
            return;
        };
        (
            saved_indirect_parameters
                .data
                .iter()
                .take(saved_indirect_parameters.count as usize)
                .map(|indirect_parameters| indirect_parameters.instance_count)
                .sum(),
            saved_indirect_parameters.occlusion_culling_supported,
            saved_indirect_parameters.occlusion_culling_introspection_supported,
        )
    };

    // Change the text.
    for mut text in &mut texts {
        text.0 = String::new();
        if !occlusion_culling_supported {
            text.0
                .push_str("Occlusion culling not supported on this platform");
            continue;
        }

        let _ = writeln!(
            &mut text.0,
            "Occlusion culling {} (Press Space to toggle)",
            if app_status.occlusion_culling {
                "ON"
            } else {
                "OFF"
            },
        );

        if !occlusion_culling_introspection_supported {
            continue;
        }

        let _ = write!(
            &mut text.0,
            "{rendered_object_count}/{total_mesh_count} meshes rendered"
        );
    }
}

/// A system that reads the indirect parameters back from the GPU so that we can
/// report how many meshes were culled.
fn readback_indirect_parameters(
    mut indirect_parameters_staging_buffers: ResMut<IndirectParametersStagingBuffers>,
    saved_indirect_parameters: Res<SavedIndirectParameters>,
) {
    // If culling isn't supported on this platform, bail.
    if !saved_indirect_parameters
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .occlusion_culling_supported
    {
        return;
    }

    // Grab the staging buffers.
    let (Some(data_buffer), Some(batch_sets_buffer)) = (
        indirect_parameters_staging_buffers.data.take(),
        indirect_parameters_staging_buffers.batch_sets.take(),
    ) else {
        return;
    };

    // Read the GPU buffers back.
    let saved_indirect_parameters_0 = (**saved_indirect_parameters).clone();
    let saved_indirect_parameters_1 = (**saved_indirect_parameters).clone();
    readback_buffer::<IndirectParametersIndexed>(data_buffer, move |indirect_parameters| {
        saved_indirect_parameters_0
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .data = indirect_parameters.to_vec();
    });
    readback_buffer::<u32>(batch_sets_buffer, move |indirect_parameters_count| {
        saved_indirect_parameters_1
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .count = indirect_parameters_count[0];
    });
}

// A helper function to asynchronously read an array of [`Pod`] values back from
// the GPU to the CPU.
//
// The given callback is invoked when the data is ready. The buffer will
// automatically be unmapped after the callback executes.
fn readback_buffer<T>(buffer: Buffer, callback: impl FnOnce(&[T]) + Send + 'static)
where
    T: Pod,
{
    // We need to make another reference to the buffer so that we can move the
    // original reference into the closure below.
    let original_buffer = buffer.clone();
    original_buffer
        .slice(..)
        .map_async(MapMode::Read, move |result| {
            // Make sure we succeeded.
            if result.is_err() {
                return;
            }

            {
                // Cast the raw bytes in the GPU buffer to the appropriate type.
                let buffer_view = buffer.slice(..).get_mapped_range();
                let indirect_parameters: &[T] = bytemuck::cast_slice(
                    &buffer_view[0..(buffer_view.len() / size_of::<T>() * size_of::<T>())],
                );

                // Invoke the callback.
                callback(indirect_parameters);
            }

            // Unmap the buffer. We have to do this before submitting any more
            // GPU command buffers, or `wgpu` will assert.
            buffer.unmap();
        });
}

/// Adds or removes the [`OcclusionCulling`] and [`DepthPrepass`] components
/// when the user presses the spacebar.
fn toggle_occlusion_culling_on_request(
    mut commands: Commands,
    input: Res<ButtonInput<KeyCode>>,
    mut app_status: ResMut<AppStatus>,
    cameras: Query<Entity, With<Camera3d>>,
) {
    // Only run when the user presses the spacebar.
    if !input.just_pressed(KeyCode::Space) {
        return;
    }

    // Toggle the occlusion culling flag in `AppStatus`.
    app_status.occlusion_culling = !app_status.occlusion_culling;

    // Add or remove the `OcclusionCulling` and `DepthPrepass` components as
    // requested.
    for camera in &cameras {
        if app_status.occlusion_culling {
            commands
                .entity(camera)
                .insert(DepthPrepass)
                .insert(OcclusionCulling);
        } else {
            commands
                .entity(camera)
                .remove::<DepthPrepass>()
                .remove::<OcclusionCulling>();
        }
    }
}
