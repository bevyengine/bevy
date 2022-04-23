use bevy_app::prelude::*;
use bevy_asset::{Assets, Handle};
use bevy_core_pipeline::{draw_3d_graph, node, AlphaMask3d, Opaque3d, Transparent3d};
use bevy_ecs::entity::Entity;
use bevy_ecs::prelude::{Component, World};
use bevy_ecs::system::{Commands, Query};

use bevy_render::prelude::Image;
use bevy_render::render_asset::RenderAssets;
use bevy_render::render_graph::{self, NodeRunError, RenderGraph, RenderGraphContext, SlotValue};
use bevy_render::render_phase::RenderPhase;
use bevy_render::render_resource::{
    Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, ImageCopyBuffer,
    ImageDataLayout, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};
use bevy_render::renderer::{RenderContext, RenderDevice, RenderQueue};
use bevy_render::{RenderApp, RenderStage};

// The name of the final node of the first pass.
pub const FRAME_CAPTURE_DRIVER: &str = "frame_capture_driver";

#[derive(Component, Clone)]
pub struct FrameCapture {
    pub cpu_buffer: Buffer,
    pub gpu_image: Handle<Image>,
    pub width: u32,
    pub height: u32,
    pub camera: Option<Entity>,
    pub active: bool,
}

impl FrameCapture {
    pub fn new(
        width: u32,
        height: u32,
        active: bool,
        format: TextureFormat,
        images: &mut Assets<Image>,
        render_device: &RenderDevice,
    ) -> Self {
        let size = Extent3d {
            width,
            height,
            ..Default::default()
        };

        // This is the texture that will be rendered to.
        let mut image = Image {
            texture_descriptor: TextureDescriptor {
                label: None,
                size,
                dimension: TextureDimension::D2,
                format,
                mip_level_count: 1,
                sample_count: 1,
                usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::COPY_SRC
                    | TextureUsages::RENDER_ATTACHMENT,
            },
            ..Default::default()
        };
        image.resize(size);

        let gpu_image = images.add(image);

        let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(width as usize) * 4;

        let size = padded_bytes_per_row as u64 * height as u64;

        let cpu_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("Output Buffer"),
            size,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        FrameCapture {
            cpu_buffer,
            gpu_image,
            width,
            height,
            active,
            camera: None,
        }
    }
}

// Add 3D render phases for CAPTURE_CAMERA.
pub fn extract_camera_phases(mut commands: Commands, captures: Query<(Entity, &FrameCapture)>) {
    for (entity, capture) in captures.iter() {
        if capture.active {
            let mut new_capture = capture.clone();
            new_capture.camera = Some(entity);
            commands
                .get_or_spawn(entity)
                .insert_bundle((
                    RenderPhase::<Opaque3d>::default(),
                    RenderPhase::<AlphaMask3d>::default(),
                    RenderPhase::<Transparent3d>::default(),
                ))
                .insert(new_capture);
        }
    }
}

// A node for the first pass camera that runs draw_3d_graph with this camera.
#[derive(Default)]
pub struct CaptureCameraDriver {
    pub captures: Vec<FrameCapture>,
}

impl render_graph::Node for CaptureCameraDriver {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let gpu_images = world.get_resource::<RenderAssets<Image>>().unwrap();

        for capture in self.captures.iter() {
            graph.run_sub_graph(
                draw_3d_graph::NAME,
                vec![SlotValue::Entity(capture.camera.unwrap())],
            )?;

            let gpu_image = gpu_images.get(&capture.gpu_image).unwrap();
            let mut encoder = render_context
                .render_device
                .create_command_encoder(&CommandEncoderDescriptor::default());
            let padded_bytes_per_row =
                RenderDevice::align_copy_bytes_per_row((gpu_image.size.width) as usize) * 4;

            let texture_extent = Extent3d {
                width: gpu_image.size.width as u32,
                height: gpu_image.size.height as u32,
                depth_or_array_layers: 1,
            };

            encoder.copy_texture_to_buffer(
                gpu_image.texture.as_image_copy(),
                ImageCopyBuffer {
                    buffer: &capture.cpu_buffer,
                    layout: ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(
                            std::num::NonZeroU32::new(padded_bytes_per_row as u32).unwrap(),
                        ),
                        rows_per_image: None,
                    },
                },
                texture_extent,
            );
            let render_queue = world.get_resource::<RenderQueue>().unwrap();
            render_queue.submit(std::iter::once(encoder.finish()));
        }

        Ok(())
    }
    fn update(&mut self, world: &mut World) {
        for cap in world.query::<&mut FrameCapture>().iter_mut(world) {
            self.captures.push(cap.clone());
        }
    }
}

pub struct CapturePlugin;
impl Plugin for CapturePlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        // This will add 3D render phases for the capture camera.
        render_app.add_system_to_stage(RenderStage::Extract, extract_camera_phases);

        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();

        // Add a node for the capture.
        graph.add_node(FRAME_CAPTURE_DRIVER, CaptureCameraDriver::default());

        // The capture's dependencies include those of the main pass.
        graph
            .add_node_edge(node::MAIN_PASS_DEPENDENCIES, FRAME_CAPTURE_DRIVER)
            .unwrap();

        // Insert the capture node: CLEAR_PASS_DRIVER -> FRAME_CAPTURE_DRIVER -> MAIN_PASS_DRIVER
        graph
            .add_node_edge(node::CLEAR_PASS_DRIVER, FRAME_CAPTURE_DRIVER)
            .unwrap();
        graph
            .add_node_edge(FRAME_CAPTURE_DRIVER, node::MAIN_PASS_DRIVER)
            .unwrap();
    }
}
