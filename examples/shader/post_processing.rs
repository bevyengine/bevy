//! A custom post processing effect, using two cameras, with one reusing the render texture of the first one.
//! Here a chromatic aberration is applied to a 3d scene containing a rotating cube.
//! This example is useful to implement your own post-processing effect such as
//! edge detection, blur, pixelization, vignette... and countless others.

use bevy::{core_pipeline::clear_color::ClearColorConfig, prelude::*};
use post_processing::PostProcessingCamera;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugin(post_processing::PostProcessingPlugin)
        .add_startup_system(setup)
        .add_system(main_camera_cube_rotator_system);

    app.run();
}

/// Marks the first camera cube (rendered to a texture.)
#[derive(Component)]
struct MainCube;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    asset_server.watch_for_changes().unwrap();

    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 4.0 }));
    let cube_material_handle = materials.add(StandardMaterial {
        base_color: Color::rgb(0.8, 0.7, 0.6),
        reflectance: 0.02,
        unlit: false,
        ..default()
    });

    // The cube that will be rendered to the texture.
    commands
        .spawn_bundle(PbrBundle {
            mesh: cube_handle,
            material: cube_material_handle,
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
            ..default()
        })
        .insert(MainCube);

    // Light
    // NOTE: Currently lights are ignoring render layers - see https://github.com/bevyengine/bevy/issues/3462
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        ..default()
    });

    // Main camera, first to render
    commands
        .spawn_bundle(Camera3dBundle {
            camera_3d: Camera3d {
                clear_color: ClearColorConfig::Custom(Color::WHITE),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 15.0))
                .looking_at(Vec3::default(), Vec3::Y),
            ..default()
        })
        .insert(PostProcessingCamera);
}

/// Rotates the cube rendered by the main camera
fn main_camera_cube_rotator_system(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<MainCube>>,
) {
    for mut transform in &mut query {
        transform.rotate_x(0.55 * time.delta_seconds());
        transform.rotate_z(0.15 * time.delta_seconds());
    }
}

mod post_processing {
    use bevy::{
        prelude::*,
        reflect::TypeUuid,
        render::{
            camera::RenderTarget,
            mesh::Indices,
            render_resource::{
                AsBindGroup, Extent3d, PrimitiveTopology, ShaderRef, TextureDescriptor,
                TextureDimension, TextureFormat, TextureUsages,
            },
            texture::BevyDefault,
            view::RenderLayers,
        },
        sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle},
        window::{WindowId, WindowResized},
    };

    pub struct PostProcessingPlugin;

    impl Plugin for PostProcessingPlugin {
        fn build(&self, app: &mut App) {
            app.add_plugin(Material2dPlugin::<PostProcessingMaterial>::default())
                .add_system(setup_new_color_blindness_cameras)
                .add_system(update_image_to_window_size)
                .add_system(update_material);
        }
    }

    /// To support window resizing, this fits an image to a windows size.
    #[derive(Component)]
    struct FitToWindowSize {
        image: Handle<Image>,
        material: Handle<PostProcessingMaterial>,
        window_id: WindowId,
    }
    #[derive(Component)]
    pub struct PostProcessingCamera;

    /// Update image size to fit window
    fn update_image_to_window_size(
        windows: Res<Windows>,
        mut image_events: EventWriter<AssetEvent<Image>>,
        mut images: ResMut<Assets<Image>>,
        mut post_processing_materials: ResMut<Assets<PostProcessingMaterial>>,
        mut resize_events: EventReader<WindowResized>,
        fit_to_window_size: Query<&FitToWindowSize>,
    ) {
        for resize_event in resize_events.iter() {
            for fit_to_window in fit_to_window_size.iter() {
                if resize_event.id == fit_to_window.window_id {
                    let size = {
                        let window = windows.get(fit_to_window.window_id).expect("ColorBlindnessCamera is rendering to a window, but this window could not be found");
                        Extent3d {
                            width: window.physical_width(),
                            height: window.physical_height(),
                            ..Default::default()
                        }
                    };
                    let image = images.get_mut(&fit_to_window.image).expect(
                        "FitToScreenSize is referring to an Image, but this Image could not be found",
                    );
                    dbg!(format!("resize to {:?}", size));
                    image.resize(size);
                    // Hack because of https://github.com/bevyengine/bevy/issues/5595
                    image_events.send(AssetEvent::Modified {
                        handle: fit_to_window.image.clone(),
                    });
                    post_processing_materials.get_mut(&fit_to_window.material);
                }
            }
        }
    }

    fn update_material(
        time: Res<Time>,
        cameras: Query<&Handle<PostProcessingMaterial>>,
        mut materials: ResMut<Assets<PostProcessingMaterial>>,
    ) {
        for handle in &cameras {
            let mut mat = materials.get_mut(handle).unwrap();

            mat.offset_r = Vec2::new(-0.01f32 * time.seconds_since_startup().sin() as f32, 0f32);
            mat.offset_g = Vec2::new(
                0.02f32 * time.seconds_since_startup().sin() as f32,
                0.02f32 * time.seconds_since_startup().cos() as f32,
            );
            mat.offset_b = Vec2::new(0f32, -0.01f32 * time.seconds_since_startup().cos() as f32);
        }
    }

    /// sets up post processing for cameras that have had `ColorBlindnessCamera` added
    fn setup_new_color_blindness_cameras(
        mut commands: Commands,
        windows: Res<Windows>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut post_processing_materials: ResMut<Assets<PostProcessingMaterial>>,
        mut images: ResMut<Assets<Image>>,
        mut cameras: Query<(Entity, &mut Camera), Added<PostProcessingCamera>>,
    ) {
        for (entity, mut camera) in &mut cameras {
            let original_target = camera.target.clone();

            let mut option_window_id: Option<WindowId> = None;

            // Get the size the camera is rendering to
            let size = match &camera.target {
                RenderTarget::Window(window_id) => {
                    let window = windows.get(*window_id).expect("ColorBlindnessCamera is rendering to a window, but this window could not be found");
                    option_window_id = Some(*window_id);
                    Extent3d {
                        width: window.physical_width(),
                        height: window.physical_height(),
                        ..Default::default()
                    }
                }
                RenderTarget::Image(handle) => {
                    let image = images.get(handle).expect(
                    "ColorBlindnessCamera is rendering to an Image, but this Image could not be found",
                );
                    image.texture_descriptor.size
                }
            };

            // This is the texture that will be rendered to.
            let mut image = Image {
                texture_descriptor: TextureDescriptor {
                    label: None,
                    size,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::bevy_default(),
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: TextureUsages::TEXTURE_BINDING
                        | TextureUsages::COPY_DST
                        | TextureUsages::RENDER_ATTACHMENT,
                },
                ..Default::default()
            };

            // fill image.data with zeroes
            image.resize(size);

            let image_handle = images.add(image);

            // This specifies the layer used for the post processing camera, which will be attached to the post processing camera and 2d fullscreen triangle.
            let post_processing_pass_layer =
                RenderLayers::layer((RenderLayers::TOTAL_LAYERS - 1) as u8);
            let half_extents = Vec2::new(size.width as f32 / 2f32, size.height as f32 / 2f32);
            let mut triangle_mesh = Mesh::new(PrimitiveTopology::TriangleList);
            // NOTE: positions are actually not used because the vertex shader maps UV and clip space.
            triangle_mesh.insert_attribute(
                Mesh::ATTRIBUTE_POSITION,
                vec![
                    [-half_extents.x, -half_extents.y, 0.0],
                    [half_extents.x * 3f32, -half_extents.y, 0.0],
                    [-half_extents.x, half_extents.y * 3f32, 0.0],
                ],
            );
            triangle_mesh.set_indices(Some(Indices::U32(vec![0, 1, 2])));
            triangle_mesh.insert_attribute(
                Mesh::ATTRIBUTE_NORMAL,
                vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
            );

            triangle_mesh.insert_attribute(
                Mesh::ATTRIBUTE_UV_0,
                vec![[2.0, 0.0], [0.0, 2.0], [0.0, 0.0]],
            );
            let triangle_handle = meshes.add(triangle_mesh);

            // This material has the texture that has been rendered.
            let material_handle = post_processing_materials.add(PostProcessingMaterial {
                source_image: image_handle.clone(),
                offset_r: Vec2::new(0.1f32, 0.1f32),
                offset_g: Vec2::new(0.1f32, -0.1f32),
                offset_b: Vec2::new(-0.1f32, -0.1f32),
            });

            commands
                .entity(entity)
                // add the handle to the camera so we can access it and change the percentages
                .insert(material_handle.clone())
                // also disable show_ui so UI elements don't get rendered twice
                .insert(UiCameraConfig { show_ui: false });
            if let Some(window_id) = option_window_id {
                commands.entity(entity).insert(FitToWindowSize {
                    image: image_handle.clone(),
                    material: material_handle.clone(),
                    window_id,
                });
            }
            camera.target = RenderTarget::Image(image_handle);

            // Post processing 2d fullscreen triangle, with material using the render texture done by the main camera, with a custom shader.
            commands
                .spawn_bundle(MaterialMesh2dBundle {
                    mesh: triangle_handle.into(),
                    material: material_handle,
                    transform: Transform {
                        translation: Vec3::new(0.0, 0.0, 1.5),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .insert(post_processing_pass_layer);

            // The post-processing pass camera.
            commands
                .spawn_bundle(Camera2dBundle {
                    camera: Camera {
                        // renders after the first main camera which has default value: 0.
                        priority: camera.priority + 10,
                        // set this new camera to render to where the other camera was rendering
                        target: original_target,
                        ..Default::default()
                    },
                    ..Camera2dBundle::default()
                })
                .insert(post_processing_pass_layer);
        }
    }

    // Region below declares of the custom material handling post processing effect

    /// Our custom post processing material
    #[derive(AsBindGroup, TypeUuid, Clone)]
    #[uuid = "bc2f08eb-a0fb-43f1-a908-54871ea597d5"]
    struct PostProcessingMaterial {
        /// In this example, this image will be the result of the main camera.
        #[texture(0)]
        #[sampler(1)]
        source_image: Handle<Image>,

        #[uniform(2)]
        offset_r: Vec2,
        #[uniform(3)]
        offset_g: Vec2,
        #[uniform(4)]
        offset_b: Vec2,
    }

    impl Material2d for PostProcessingMaterial {
        fn fragment_shader() -> ShaderRef {
            "shaders/custom_material_chromatic_aberration.wgsl".into()
        }
        fn vertex_shader() -> ShaderRef {
            "shaders/screen_vertex.wgsl".into()
        }
    }
}
