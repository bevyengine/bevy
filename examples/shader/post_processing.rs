//! Shows how to render a custom post processing effect, here a chromatic aberration.
//! This example is useful to implement your own post-processing effect such as
//! edge detection, blur, pixelization, vignette... and countless others.

use bevy::{
    core_pipeline::{
        draw_3d_graph, node, AlphaMask3d, Opaque3d, RenderTargetClearColors, Transparent3d,
    },
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    reflect::TypeUuid,
    render::{
        camera::{ActiveCamera, Camera, CameraTypePlugin, RenderTarget},
        render_asset::{PrepareAssetError, RenderAsset, RenderAssets},
        render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotValue},
        render_phase::RenderPhase,
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            Extent3d, SamplerBindingType, ShaderStages, TextureDescriptor, TextureDimension,
            TextureFormat, TextureSampleType, TextureUsages, TextureViewDimension,
        },
        renderer::{RenderContext, RenderDevice},
        view::RenderLayers,
        RenderApp, RenderStage,
    },
    sprite::{Material2d, Material2dPipeline, Material2dPlugin, MaterialMesh2dBundle},
};

#[derive(Component, Default)]
pub struct FirstPassCamera;

/// The name of the final node of the first pass.
pub const FIRST_PASS_DRIVER: &str = "first_pass_driver";

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugin(Material2dPlugin::<PostProcessingMaterial>::default())
        .add_plugin(CameraTypePlugin::<FirstPassCamera>::default())
        .add_startup_system(setup)
        .add_system(first_pass_cube_rotator_system);

    let render_app = app.sub_app_mut(RenderApp);
    let driver = FirstPassCameraDriver::new(&mut render_app.world);
    // This will add 3D render phases for the new camera.
    render_app.add_system_to_stage(RenderStage::Extract, extract_first_pass_camera_phases);

    let mut graph = render_app.world.resource_mut::<RenderGraph>();

    // Add a node for the first pass.
    graph.add_node(FIRST_PASS_DRIVER, driver);

    // The first pass's dependencies include those of the main pass.
    graph
        .add_node_edge(node::MAIN_PASS_DEPENDENCIES, FIRST_PASS_DRIVER)
        .unwrap();

    // Insert the first pass node: CLEAR_PASS_DRIVER -> FIRST_PASS_DRIVER -> MAIN_PASS_DRIVER
    graph
        .add_node_edge(node::CLEAR_PASS_DRIVER, FIRST_PASS_DRIVER)
        .unwrap();
    graph
        .add_node_edge(FIRST_PASS_DRIVER, node::MAIN_PASS_DRIVER)
        .unwrap();
    app.run();
}

/// Add 3D render phases for `FirstPassCamera`.
fn extract_first_pass_camera_phases(
    mut commands: Commands,
    active: Res<ActiveCamera<FirstPassCamera>>,
) {
    if let Some(entity) = active.get() {
        commands.get_or_spawn(entity).insert_bundle((
            RenderPhase::<Opaque3d>::default(),
            RenderPhase::<AlphaMask3d>::default(),
            RenderPhase::<Transparent3d>::default(),
        ));
    }
}

/// A node for the `FirstPassCamera` that runs `draw_3d_graph` with this camera.
struct FirstPassCameraDriver {
    query: QueryState<Entity, With<FirstPassCamera>>,
}

impl FirstPassCameraDriver {
    pub fn new(render_world: &mut World) -> Self {
        Self {
            query: QueryState::new(render_world),
        }
    }
}
impl Node for FirstPassCameraDriver {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        for camera in self.query.iter_manual(world) {
            graph.run_sub_graph(draw_3d_graph::NAME, vec![SlotValue::Entity(camera)])?;
        }
        Ok(())
    }
}

/// Marks the first pass cube (rendered to a texture.)
#[derive(Component)]
struct FirstPassCube;

fn setup(
    mut commands: Commands,
    mut windows: ResMut<Windows>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut post_processing_materials: ResMut<Assets<PostProcessingMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut clear_colors: ResMut<RenderTargetClearColors>,
) {
    let window = windows.get_primary_mut().unwrap();
    let size = Extent3d {
        width: window.physical_width(),
        height: window.physical_height(),
        ..default()
    };

    // This is the texture that will be rendered to.
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
        },
        ..default()
    };

    // fill image.data with zeroes
    image.resize(size);

    let image_handle = images.add(image);

    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 4.0 }));
    let cube_material_handle = materials.add(StandardMaterial {
        base_color: Color::rgb(0.8, 0.7, 0.6),
        reflectance: 0.02,
        unlit: false,
        ..default()
    });

    // This specifies the layer used for the first pass, which will be attached to the first pass camera and cube.
    let first_pass_layer = RenderLayers::layer(1);

    // The cube that will be rendered to the texture.
    commands
        .spawn_bundle(PbrBundle {
            mesh: cube_handle,
            material: cube_material_handle,
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
            ..default()
        })
        .insert(FirstPassCube)
        .insert(first_pass_layer);

    // Light
    // NOTE: Currently lights are shared between passes - see https://github.com/bevyengine/bevy/issues/3462
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        ..default()
    });

    // First pass camera
    let render_target = RenderTarget::Image(image_handle.clone());
    clear_colors.insert(render_target.clone(), Color::WHITE);
    commands
        .spawn_bundle(PerspectiveCameraBundle::<FirstPassCamera> {
            camera: Camera {
                target: render_target,
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 15.0))
                .looking_at(Vec3::default(), Vec3::Y),
            ..PerspectiveCameraBundle::new()
        })
        .insert(first_pass_layer);
    // NOTE: omitting the RenderLayers component for this camera may cause a validation error:
    //
    // thread 'main' panicked at 'wgpu error: Validation Error
    //
    //    Caused by:
    //        In a RenderPass
    //          note: encoder = `<CommandBuffer-(0, 1, Metal)>`
    //        In a pass parameter
    //          note: command buffer = `<CommandBuffer-(0, 1, Metal)>`
    //        Attempted to use texture (5, 1, Metal) mips 0..1 layers 0..1 as a combination of COLOR_TARGET within a usage scope.
    //
    // This happens because the texture would be written and read in the same frame, which is not allowed.
    // So either render layers must be used to avoid this, or the texture must be double buffered.

    let quad_handle = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
        size.width as f32,
        size.height as f32,
    ))));

    // This material has the texture that has been rendered.
    let material_handle = post_processing_materials.add(PostProcessingMaterial {
        source_image: image_handle,
    });

    // Main pass 2d quad, with material containing the rendered first pass texture, with a custom shader.
    commands.spawn_bundle(MaterialMesh2dBundle {
        mesh: quad_handle.into(),
        material: material_handle,
        transform: Transform {
            translation: Vec3::new(0.0, 0.0, 1.5),
            ..default()
        },
        ..default()
    });

    // The main pass camera.
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

/// Rotates the inner cube (first pass)
fn first_pass_cube_rotator_system(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<FirstPassCube>>,
) {
    for mut transform in query.iter_mut() {
        transform.rotation *= Quat::from_rotation_x(1.5 * time.delta_seconds());
        transform.rotation *= Quat::from_rotation_z(1.3 * time.delta_seconds());
    }
}

// Region below declares of the custom material handling post processing effect

/// Our custom post processing material
#[derive(TypeUuid, Clone)]
#[uuid = "bc2f08eb-a0fb-43f1-a908-54871ea597d5"]
struct PostProcessingMaterial {
    /// In this example, this image will be the result of the first pass.
    source_image: Handle<Image>,
}

struct PostProcessingMaterialGPU {
    bind_group: BindGroup,
}

impl Material2d for PostProcessingMaterial {
    fn bind_group(material: &PostProcessingMaterialGPU) -> &BindGroup {
        &material.bind_group
    }

    fn bind_group_layout(render_device: &RenderDevice) -> BindGroupLayout {
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        view_dimension: TextureViewDimension::D2,
                        sample_type: TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        })
    }

    fn fragment_shader(asset_server: &AssetServer) -> Option<Handle<Shader>> {
        asset_server.watch_for_changes().unwrap();
        Some(asset_server.load("shaders/custom_material_chromatic_aberration.wgsl"))
    }
}

impl RenderAsset for PostProcessingMaterial {
    type ExtractedAsset = PostProcessingMaterial;
    type PreparedAsset = PostProcessingMaterialGPU;
    type Param = (
        SRes<RenderDevice>,
        SRes<Material2dPipeline<PostProcessingMaterial>>,
        SRes<RenderAssets<Image>>,
    );

    fn prepare_asset(
        extracted_asset: PostProcessingMaterial,
        (render_device, pipeline, images): &mut SystemParamItem<Self::Param>,
    ) -> Result<PostProcessingMaterialGPU, PrepareAssetError<PostProcessingMaterial>> {
        let (view, sampler) = if let Some(result) = pipeline
            .mesh2d_pipeline
            .get_image_texture(images, &Some(extracted_asset.source_image.clone()))
        {
            result
        } else {
            return Err(PrepareAssetError::RetryNextUpdate(extracted_asset));
        };

        let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &pipeline.material2d_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(sampler),
                },
            ],
        });
        Ok(PostProcessingMaterialGPU { bind_group })
    }

    fn extract_asset(&self) -> PostProcessingMaterial {
        self.clone()
    }
}
