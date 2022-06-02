//! A custom post processing effect, using two render passes and a custom shader.
//! Here a chromatic aberration is applied to a 3d scene containting a rotating cube.
//! This example is useful to implement your own post-processing effect such as
//! edge detection, blur, pixelization, vignette... and countless others.

use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    reflect::TypeUuid,
    render::{
        camera::{Camera, RenderTarget},
        render_asset::{PrepareAssetError, RenderAsset, RenderAssets},
        render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotValue},
        render_resource::{
            BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
            BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
            Extent3d, SamplerBindingType, ShaderStages, TextureDescriptor, TextureDimension,
            TextureFormat, TextureSampleType, TextureUsages, TextureViewDimension,
        },
        renderer::{RenderContext, RenderDevice},
        view::RenderLayers,
        RenderApp,
    },
    sprite::{Material2d, Material2dPipeline, Material2dPlugin, MaterialMesh2dBundle},
};

#[derive(Component, Default)]
pub struct PostProcessingPassCamera;

/// The name of the final node of the post process pass.
pub const POST_PROCESS_PASS_DRIVER: &str = "post_process_pass_driver";

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_plugin(Material2dPlugin::<PostProcessingMaterial>::default())
        .add_startup_system(setup)
        .add_system(main_pass_cube_rotator_system);

    app.run();
}

/// Marks the Main pass cube (rendered to a texture.)
#[derive(Component)]
struct MainPassCube;

fn setup(
    mut commands: Commands,
    mut windows: ResMut<Windows>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut post_processing_materials: ResMut<Assets<PostProcessingMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
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

    // The cube that will be rendered to the texture.
    commands
        .spawn_bundle(PbrBundle {
            mesh: cube_handle,
            material: cube_material_handle,
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
            ..default()
        })
        .insert(MainPassCube);

    // Light
    // NOTE: Currently lights are shared between passes - see https://github.com/bevyengine/bevy/issues/3462
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        ..default()
    });

    // Main pass camera
    commands.spawn_bundle(Camera3dBundle {
        camera_3d: Camera3d {
            clear_color: ClearColorConfig::Custom(Color::WHITE),
        },
        camera: Camera {
            target: RenderTarget::Image(image_handle.clone()),
            ..default()
        },
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 15.0))
            .looking_at(Vec3::default(), Vec3::Y),
        ..default()
    });

    // This specifies the layer used for the post processing pass, which will be attached to the post processing pass camera and 2d quad.
    let post_processing_pass_layer = RenderLayers::layer((RenderLayers::TOTAL_LAYERS - 1) as u8);

    let quad_handle = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
        size.width as f32,
        size.height as f32,
    ))));

    // This material has the texture that has been rendered.
    let material_handle = post_processing_materials.add(PostProcessingMaterial {
        source_image: image_handle,
    });

    // Post processing pass 2d quad, with material containing the rendered main pass texture, with a custom shader.
    commands
        .spawn_bundle(MaterialMesh2dBundle {
            mesh: quad_handle.into(),
            material: material_handle,
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 1.5),
                ..default()
            },
            ..default()
        })
        .insert(post_processing_pass_layer);

    // The post-processing pass camera.
    commands
        .spawn_bundle(Camera2dBundle {
            camera: Camera {
                // render after the "main pass" camera
                priority: 1,
                ..default()
            },
            ..Camera2dBundle::default()
        })
        .insert(PostProcessingPassCamera)
        .insert(post_processing_pass_layer);
}

/// Rotates the inner cube (main pass)
fn main_pass_cube_rotator_system(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<MainPassCube>>,
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
    /// In this example, this image will be the result of the main pass.
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
