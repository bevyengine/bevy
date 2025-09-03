//! Demonstrates how to use depth-only cameras.
//!
//! A *depth-only camera* is a camera that renders only to a depth buffer, not
//! to a color buffer. That depth buffer can then be used in shaders for various
//! special effects.
//!
//! To create a depth-only camera, we create a [`Camera3d`] and set its
//! [`RenderTarget`] to [`RenderTarget::None`] to disable creation of a color
//! buffer. Then we add a new node to the render graph that copies the
//! [`bevy::render::view::ViewDepthTexture`] that Bevy creates for that camera
//! to a texture. This texture can then be attached to a material and sampled in
//! the shader.
//!
//! This demo consists of a rotating cube with a depth-only camera pointed at
//! it. The depth texture from the depth-only camera appears on a plane. You can
//! use the WASD keys to make the depth-only camera orbit around the cube.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{
    asset::RenderAssetUsages,
    camera::RenderTarget,
    color::palettes::css::LIME,
    core_pipeline::{
        core_3d::graph::{Core3d, Node3d},
        prepass::DepthPrepass,
    },
    ecs::{query::QueryItem, system::lifetimeless::Read},
    image::{ImageCompareFunction, ImageSampler, ImageSamplerDescriptor},
    math::ops::{acos, atan2, sin_cos},
    prelude::*,
    render::{
        camera::ExtractedCamera,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_graph::{
            NodeRunError, RenderGraphContext, RenderGraphExt as _, RenderLabel, ViewNode,
            ViewNodeRunner,
        },
        render_resource::{
            AsBindGroup, CommandEncoderDescriptor, Extent3d, Origin3d, TexelCopyTextureInfo,
            TextureAspect, TextureDimension, TextureFormat,
        },
        renderer::RenderContext,
        texture::GpuImage,
        view::ViewDepthTexture,
        RenderApp,
    },
    shader::ShaderRef,
};

/// A marker component for a rotating cube.
#[derive(Component)]
struct RotatingCube;

/// The material that displays the contents of the depth buffer.
///
/// This material is placed on the plane.
#[derive(Clone, Debug, Asset, TypePath, AsBindGroup)]
struct ShowDepthTextureMaterial {
    /// A copy of the depth texture that the depth-only camera produced.
    #[texture(0, sample_type = "depth")]
    #[sampler(1, sampler_type = "comparison")]
    depth_texture: Option<Handle<Image>>,
}

/// A label for the render node that copies the depth buffer from that of the
/// camera to the [`DemoDepthTexture`].
#[derive(Clone, PartialEq, Eq, Hash, Debug, RenderLabel)]
struct CopyDepthTexturePass;

/// The render node that copies the depth buffer from that of the camera to the
/// [`DemoDepthTexture`].
#[derive(Default)]
struct CopyDepthTextureNode;

/// Holds a copy of the depth buffer that the depth-only camera produces.
///
/// We need to make a copy for two reasons:
///
/// 1. The Bevy renderer automatically creates and maintains depth buffers on
///    its own. There's no mechanism to fetch the depth buffer for a camera outside
///    the render app. Thus it can't easily be attached to a material.
///
/// 2. `wgpu` doesn't allow applications to simultaneously render to and sample
///    from a standard depth texture, so a copy must be made regardless.
#[derive(Clone, Resource)]
struct DemoDepthTexture(Handle<Image>);

/// [Spherical coordinates], used to implement the camera orbiting
/// functionality.
///
/// Note that these are in the mathematics convention, not the physics
/// convention. In a real application, one would probably use the physics
/// convention, but for familiarity's sake we stick to the most common
/// convention here.
///
/// [Spherical coordinates]: https://en.wikipedia.org/wiki/Spherical_coordinate_system
#[derive(Clone, Copy, Debug)]
struct SphericalCoordinates {
    /// The radius, in world units.
    radius: f32,
    /// The elevation angle (latitude).
    inclination: f32,
    /// The azimuth angle (longitude).
    azimuth: f32,
}

/// The path to the shader that renders the depth texture.
static SHADER_ASSET_PATH: &str = "shaders/show_depth_texture_material.wgsl";

/// The size in texels of a depth texture.
const DEPTH_TEXTURE_SIZE: u32 = 256;

/// The rate at which the user can move the camera, in radians per second.
const CAMERA_MOVEMENT_SPEED: f32 = 2.0;

/// The entry point.
fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugins(MaterialPlugin::<ShowDepthTextureMaterial>::default())
        .add_plugins(ExtractResourcePlugin::<DemoDepthTexture>::default())
        .init_resource::<DemoDepthTexture>()
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_cube)
        .add_systems(Update, draw_camera_gizmo)
        .add_systems(Update, move_camera);

    // Add the `CopyDepthTextureNode` to the render app.
    let render_app = app
        .get_sub_app_mut(RenderApp)
        .expect("Render app should be present");
    render_app.add_render_graph_node::<ViewNodeRunner<CopyDepthTextureNode>>(
        Core3d,
        CopyDepthTexturePass,
    );
    // We have the texture copy operation run in between the prepasses and
    // the opaque pass. Since the depth rendering is part of the prepass, this
    // is a reasonable time to perform the operation.
    render_app.add_render_graph_edges(
        Core3d,
        (
            Node3d::EndPrepasses,
            CopyDepthTexturePass,
            Node3d::MainOpaquePass,
        ),
    );

    app.run();
}

/// Creates the scene.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut show_depth_texture_materials: ResMut<Assets<ShowDepthTextureMaterial>>,
    demo_depth_texture: Res<DemoDepthTexture>,
) {
    spawn_rotating_cube(&mut commands, &mut meshes, &mut standard_materials);
    spawn_plane(
        &mut commands,
        &mut meshes,
        &mut show_depth_texture_materials,
        &demo_depth_texture,
    );
    spawn_light(&mut commands);
    spawn_depth_only_camera(&mut commands);
    spawn_main_camera(&mut commands);
    spawn_instructions(&mut commands);
}

/// Spawns the main rotating cube.
fn spawn_rotating_cube(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    standard_materials: &mut Assets<StandardMaterial>,
) {
    let cube_handle = meshes.add(Cuboid::new(3.0, 3.0, 3.0));
    let rotating_cube_material_handle = standard_materials.add(StandardMaterial {
        base_color: Color::WHITE,
        unlit: false,
        ..default()
    });
    commands.spawn((
        Mesh3d(cube_handle.clone()),
        MeshMaterial3d(rotating_cube_material_handle),
        Transform::IDENTITY,
        RotatingCube,
    ));
}

// Spawns the plane that shows the depth texture.
fn spawn_plane(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    show_depth_texture_materials: &mut Assets<ShowDepthTextureMaterial>,
    demo_depth_texture: &DemoDepthTexture,
) {
    let plane_handle = meshes.add(Plane3d::new(Vec3::Z, Vec2::splat(2.0)));
    let show_depth_texture_material = show_depth_texture_materials.add(ShowDepthTextureMaterial {
        depth_texture: Some(demo_depth_texture.0.clone()),
    });
    commands.spawn((
        Mesh3d(plane_handle),
        MeshMaterial3d(show_depth_texture_material),
        Transform::from_xyz(10.0, 4.0, 0.0).with_scale(Vec3::splat(2.5)),
    ));
}

/// Spawns a light.
fn spawn_light(commands: &mut Commands) {
    commands.spawn((PointLight::default(), Transform::from_xyz(5.0, 6.0, 7.0)));
}

/// Spawns the depth-only camera.
fn spawn_depth_only_camera(commands: &mut Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-4.0, -5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        Camera {
            // We specify no color render target, for maximum efficiency.
            target: RenderTarget::None {
                // When specifying no render target, we must manually specify
                // the viewport size. Otherwise, Bevy won't know how big to make
                // the depth buffer.
                size: UVec2::splat(DEPTH_TEXTURE_SIZE),
            },
            // Make sure that we render from this depth-only camera *before*
            // rendering from the main camera.
            order: -1,
            ..Camera::default()
        },
        // We need to disable multisampling or the depth texture will be
        // multisampled, which adds complexity we don't care about for this
        // demo.
        Msaa::Off,
        // Cameras with no render target render *nothing* by default. To get
        // them to render something, we must add a prepass that specifies what
        // we want to render: in this case, depth.
        DepthPrepass,
    ));
}

/// Spawns the main camera that renders to the window.
fn spawn_main_camera(commands: &mut Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5.0, 2.0, 30.0).looking_at(vec3(5.0, 2.0, 0.0), Vec3::Y),
        // Disable antialiasing just for simplicity's sake.
        Msaa::Off,
    ));
}

/// Spawns the instructional text at the top of the screen.
fn spawn_instructions(commands: &mut Commands) {
    commands.spawn((
        Text::new("Use WASD to move the secondary camera"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..Node::default()
        },
    ));
}

/// Spins the cube a bit every frame.
fn rotate_cube(mut cubes: Query<&mut Transform, With<RotatingCube>>, time: Res<Time>) {
    for mut transform in &mut cubes {
        transform.rotate_x(1.5 * time.delta_secs());
        transform.rotate_y(1.1 * time.delta_secs());
        transform.rotate_z(-1.3 * time.delta_secs());
    }
}

impl Material for ShowDepthTextureMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}

impl ViewNode for CopyDepthTextureNode {
    type ViewQuery = (Read<ExtractedCamera>, Read<ViewDepthTexture>);

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (camera, depth_texture): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        // Make sure we only run on the depth-only camera.
        // We could make a marker component for that camera and extract it to
        // the render world, but using `order` as a tag to tell the main camera
        // and the depth-only camera apart works in a pinch.
        if camera.order >= 0 {
            return Ok(());
        }

        // Grab the texture we're going to copy to.
        let demo_depth_texture = world.resource::<DemoDepthTexture>();
        let image_assets = world.resource::<RenderAssets<GpuImage>>();
        let Some(demo_depth_image) = image_assets.get(demo_depth_texture.0.id()) else {
            return Ok(());
        };

        // Perform the copy.
        render_context.add_command_buffer_generation_task(move |render_device| {
            let mut command_encoder =
                render_device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("copy depth to demo texture command encoder"),
                });
            command_encoder.push_debug_group("copy depth to demo texture");

            // Copy from the view's depth texture to the destination depth
            // texture.
            command_encoder.copy_texture_to_texture(
                TexelCopyTextureInfo {
                    texture: &depth_texture.texture,
                    mip_level: 0,
                    origin: Origin3d::default(),
                    aspect: TextureAspect::DepthOnly,
                },
                TexelCopyTextureInfo {
                    texture: &demo_depth_image.texture,
                    mip_level: 0,
                    origin: Origin3d::default(),
                    aspect: TextureAspect::DepthOnly,
                },
                Extent3d {
                    width: DEPTH_TEXTURE_SIZE,
                    height: DEPTH_TEXTURE_SIZE,
                    depth_or_array_layers: 1,
                },
            );

            command_encoder.pop_debug_group();
            command_encoder.finish()
        });

        Ok(())
    }
}

impl FromWorld for DemoDepthTexture {
    fn from_world(world: &mut World) -> Self {
        let mut images = world.resource_mut::<Assets<Image>>();

        // Create a new 32-bit floating point depth texture.
        let mut depth_image = Image::new_uninit(
            Extent3d {
                width: DEPTH_TEXTURE_SIZE,
                height: DEPTH_TEXTURE_SIZE,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            TextureFormat::Depth32Float,
            RenderAssetUsages::default(),
        );

        // Create a sampler. Note that this needs to specify a `compare`
        // function in order to be compatible with depth textures.
        depth_image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            label: Some("custom depth image sampler".to_owned()),
            compare: Some(ImageCompareFunction::Always),
            ..ImageSamplerDescriptor::default()
        });

        let depth_image_handle = images.add(depth_image);
        DemoDepthTexture(depth_image_handle)
    }
}

impl ExtractResource for DemoDepthTexture {
    type Source = Self;

    fn extract_resource(source: &Self::Source) -> Self {
        // Share the `DemoDepthTexture` resource over to the render world so
        // that our `CopyDepthTextureNode` can access it.
        (*source).clone()
    }
}

/// Draws an outline of the depth texture on the screen.
fn draw_camera_gizmo(cameras: Query<(&Camera, &GlobalTransform)>, mut gizmos: Gizmos) {
    for (camera, transform) in &cameras {
        // As above, we use the order as a cheap tag to tell the depth texture
        // apart from the main texture.
        if camera.order >= 0 {
            continue;
        }

        // Draw a cone representing the camera.
        gizmos.primitive_3d(
            &Cone {
                radius: 1.0,
                height: 3.0,
            },
            Isometry3d::new(
                transform.translation(),
                // We have to rotate here because `Cone` primitives are oriented
                // along +Y and cameras point along +Z.
                transform.rotation() * Quat::from_rotation_x(FRAC_PI_2),
            ),
            LIME,
        );
    }
}

/// Orbits the cube when WASD is pressed.
fn move_camera(
    mut cameras: Query<(&Camera, &mut Transform)>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    for (camera, mut transform) in &mut cameras {
        // Only affect the depth camera.
        if camera.order >= 0 {
            continue;
        }

        // Convert the camera's position from Cartesian to spherical coordinates.
        let mut spherical_coords = SphericalCoordinates::from_cartesian(transform.translation);

        // Modify those spherical coordinates as appropriate.
        let mut changed = false;
        if keyboard.pressed(KeyCode::KeyW) {
            spherical_coords.inclination -= time.delta_secs() * CAMERA_MOVEMENT_SPEED;
            changed = true;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            spherical_coords.inclination += time.delta_secs() * CAMERA_MOVEMENT_SPEED;
            changed = true;
        }
        if keyboard.pressed(KeyCode::KeyA) {
            spherical_coords.azimuth += time.delta_secs() * CAMERA_MOVEMENT_SPEED;
            changed = true;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            spherical_coords.azimuth -= time.delta_secs() * CAMERA_MOVEMENT_SPEED;
            changed = true;
        }

        // If they were changed, convert from spherical coordinates back to
        // Cartesian ones, and update the camera's transform.
        if changed {
            spherical_coords.inclination = spherical_coords.inclination.clamp(0.01, PI - 0.01);
            transform.translation = spherical_coords.to_cartesian();
            transform.look_at(Vec3::ZERO, Vec3::Y);
        }
    }
}

impl SphericalCoordinates {
    /// [Converts] from Cartesian coordinates to spherical coordinates.
    ///
    /// [Converts]: https://en.wikipedia.org/wiki/Spherical_coordinate_system#Cartesian_coordinates
    fn from_cartesian(p: Vec3) -> SphericalCoordinates {
        let radius = p.length();
        SphericalCoordinates {
            radius,
            inclination: acos(p.y / radius),
            azimuth: atan2(p.z, p.x),
        }
    }

    /// [Converts] from spherical coordinates to Cartesian coordinates.
    ///
    /// [Converts]: https://en.wikipedia.org/wiki/Spherical_coordinate_system#Cartesian_coordinates
    fn to_cartesian(self) -> Vec3 {
        let (sin_inclination, cos_inclination) = sin_cos(self.inclination);
        let (sin_azimuth, cos_azimuth) = sin_cos(self.azimuth);
        self.radius
            * vec3(
                sin_inclination * cos_azimuth,
                cos_inclination,
                sin_inclination * sin_azimuth,
            )
    }
}
