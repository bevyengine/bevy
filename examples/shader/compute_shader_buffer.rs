//! A compute shader that simulates Conway's Game of Life.
//!
//! Compute shaders use the GPU for computing arbitrary information, that may be independent of what
//! is rendered to the screen.

use bevy::{
    prelude::*,
    render::{
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph},
        render_resource::*,
        renderer::{RenderContext, RenderDevice, RenderQueue},
        RenderApp, RenderStage,
    },
    ui,
};

use std::borrow::{BorrowMut, Cow};
use std::sync::{Arc, Mutex};

const SIZE: (u32, u32) = (1280, 720);
const WORKGROUP_SIZE: u32 = 8;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        .add_plugin(GradientComputePlugin)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut image = Image::new_fill(
        Extent3d {
            width: SIZE.0,
            height: SIZE.1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8Unorm,
    );
    image.texture_descriptor.usage =
        TextureUsages::COPY_DST | TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING;
    let image = images.add(image);

    commands.spawn(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(SIZE.0 as f32, SIZE.1 as f32)),
            ..default()
        },
        texture: image.clone(),
        ..default()
    });
    commands
        .spawn(ImageBundle {
            background_color: Color::RED.into(),
            style: ui::Style {
                size: Size {
                    width: Val::Px(100.0),
                    height: Val::Px(100.0),
                },
                ..default()
            },
            ..default()
        })
        .insert(ColoredSquare);
    commands.spawn(Camera2dBundle::default());

    commands.insert_resource(GradientImage(image));
}

#[derive(Resource)]
struct OutBuffer {
    picked_color: Buffer,
    position: Buffer,
}

pub struct GradientComputePlugin;

impl Plugin for GradientComputePlugin {
    fn build(&self, app: &mut App) {
        let color = PickedColor::new();
        // Extract the game of life image resource from the main world into the render world
        // for operation on by the compute shader and display on the sprite.
        app.add_plugin(ExtractResourcePlugin::<GradientImage>::default())
            .add_plugin(ExtractResourcePlugin::<ExtractedPosition>::default())
            .insert_resource(color.clone())
            .add_system(set_squares_color);

        let render_device = app.world.resource::<RenderDevice>();
        let buffer = OutBuffer {
            picked_color: render_device.create_buffer(&BufferDescriptor {
                label: Some("GPU-to-CPU buffer"),
                size: std::mem::size_of::<f32>() as u64 * 4,
                usage: BufferUsages::STORAGE | BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            position: render_device.create_buffer(&BufferDescriptor {
                label: Some("CPU-to-GPU buffer"),
                size: std::mem::size_of::<Vec2>() as u64,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        };

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<GradientPipeline>()
            .insert_resource(buffer)
            .insert_resource(color)
            .add_system_to_stage(RenderStage::Prepare, prepare_position)
            .add_system_to_stage(RenderStage::Queue, queue_bind_group);

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node("gradient", GradientNode::default());
        /*render_graph
        .add_node_edge(
            "gradient",
            bevy::render::main_graph::node::CAMERA_DRIVER,
        )
        .unwrap();
        */
    }
}

fn set_squares_color(
    color: Res<PickedColor>,
    mut colored_squares: Query<&mut BackgroundColor, With<ColoredSquare>>,
) {
    let mut color = color.0.lock().unwrap();
    let color = color.borrow_mut();
    for mut square in colored_squares.iter_mut() {
        let [red, green, blue, alpha] = **color;
        square.0 = Color::Rgba {
            red,
            green,
            blue,
            alpha,
        };
    }
}

#[derive(Component)]
struct ColoredSquare;

#[derive(Clone, Resource)]
struct PickedColor(Arc<Mutex<[f32; 4]>>);

impl PickedColor {
    fn new() -> Self {
        Self(Arc::new(Mutex::new([0.0; 4])))
    }
}

#[derive(Default, Resource)]
struct ExtractedPosition(Vec2);

impl ExtractResource for ExtractedPosition {
    type Source = Windows;

    fn extract_resource(windows: &Self::Source) -> Self {
        Self(
            windows
                .get_primary()
                .and_then(|w| dbg!(w.cursor_position()))
                .unwrap_or(Vec2::new(0.0, 0.0)),
        )
    }
}

// write the extracted position into the corresponding uniform buffer
fn prepare_position(
    position: Res<ExtractedPosition>,
    outputs: Res<OutBuffer>,
    render_queue: Res<RenderQueue>,
) {
    render_queue.write_buffer(&outputs.position, 0, bevy::core::cast_slice(&[position.0]));
}

#[derive(Clone, Deref, Resource, ExtractResource)]
struct GradientImage(Handle<Image>);

#[derive(Resource)]
struct GradientImageBindGroup(BindGroup);

fn queue_bind_group(
    mut commands: Commands,
    pipeline: Res<GradientPipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    gradient_image: Res<GradientImage>,
    render_device: Res<RenderDevice>,
    outputs: Res<OutBuffer>,
) {
    let view = &gpu_images[&gradient_image.0];
    let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &pipeline.texture_bind_group_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&view.texture_view),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Buffer(outputs.picked_color.as_entire_buffer_binding()),
            },
            BindGroupEntry {
                binding: 2,
                resource: BindingResource::Buffer(outputs.position.as_entire_buffer_binding()),
            },
        ],
    });
    commands.insert_resource(GradientImageBindGroup(bind_group));
}

#[derive(Resource)]
pub struct GradientPipeline {
    texture_bind_group_layout: BindGroupLayout,
    init_pipeline: CachedComputePipelineId,
    update_pipeline: CachedComputePipelineId,
}

impl FromWorld for GradientPipeline {
    fn from_world(world: &mut World) -> Self {
        let texture_bind_group_layout =
            world
                .resource::<RenderDevice>()
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        BindGroupLayoutEntry {
                            binding: 0,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::StorageTexture {
                                access: StorageTextureAccess::ReadWrite,
                                format: TextureFormat::Rgba8Unorm,
                                view_dimension: TextureViewDimension::D2,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 1,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        BindGroupLayoutEntry {
                            binding: 2,
                            visibility: ShaderStages::COMPUTE,
                            ty: BindingType::Buffer {
                                ty: BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: BufferSize::new(
                                    std::mem::size_of::<Vec2>() as u64
                                ),
                            },
                            count: None,
                        },
                    ],
                });
        let shader = world
            .resource::<AssetServer>()
            .load("shaders/compute_buffer.wgsl");
        let mut pipeline_cache = world.resource_mut::<PipelineCache>();
        let init_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: Some(vec![texture_bind_group_layout.clone()]),
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from("init"),
        });
        let update_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: None,
            layout: Some(vec![texture_bind_group_layout.clone()]),
            shader,
            shader_defs: vec![],
            entry_point: Cow::from("update"),
        });

        GradientPipeline {
            texture_bind_group_layout,
            init_pipeline,
            update_pipeline,
        }
    }
}

enum GradientState {
    Loading,
    Init,
    Update,
}

struct GradientNode {
    state: GradientState,
}

impl Default for GradientNode {
    fn default() -> Self {
        Self {
            state: GradientState::Loading,
        }
    }
}

impl render_graph::Node for GradientNode {
    fn update(&mut self, world: &mut World) {
        let pipeline = world.resource::<GradientPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();

        // if the corresponding pipeline has loaded, transition to the next stage
        match self.state {
            GradientState::Loading => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.init_pipeline)
                {
                    self.state = GradientState::Init;
                }
            }
            GradientState::Init => {
                if let CachedPipelineState::Ok(_) =
                    pipeline_cache.get_compute_pipeline_state(pipeline.update_pipeline)
                {
                    self.state = GradientState::Update;
                }
            }
            GradientState::Update => {}
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let texture_bind_group = &world.resource::<GradientImageBindGroup>().0;
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<GradientPipeline>();

        let mut pass = render_context
            .command_encoder
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, texture_bind_group, &[]);

        // select the pipeline based on the current state
        match self.state {
            GradientState::Loading => {}
            GradientState::Init => {
                let init_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.init_pipeline)
                    .unwrap();
                pass.set_pipeline(init_pipeline);
                pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            }
            GradientState::Update => {
                let device = world.resource::<RenderDevice>();
                let buf = world.resource::<OutBuffer>();
                let color = world.resource::<PickedColor>();

                let slice = buf.picked_color.slice(..);
                slice.map_async(MapMode::Read, move |result| {
                    let err = result.err();
                    if err.is_some() {
                        panic!("{}", err.unwrap().to_string());
                    }
                });
                let device = device.wgpu_device();
                device.poll(Maintain::Wait);

                let data = slice.get_mapped_range();

                let bufptr = data.as_ptr() as *const f32;
                let bufdata = unsafe { std::slice::from_raw_parts(bufptr, 4) };
                let mut color = color.0.lock().unwrap();
                let color_array = color.borrow_mut();
                color_array.copy_from_slice(bufdata);

                drop(data);
                buf.picked_color.unmap();

                let update_pipeline = pipeline_cache
                    .get_compute_pipeline(pipeline.update_pipeline)
                    .unwrap();
                pass.set_pipeline(update_pipeline);
                pass.dispatch_workgroups(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            }
        }

        Ok(())
    }
}
