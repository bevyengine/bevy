use bevy::{
    core_pipeline::node::MAIN_PASS_DEPENDENCIES,
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{self, RenderGraph},
        render_resource::*,
        renderer::{RenderContext, RenderDevice},
        RenderApp, RenderStage,
    },
    window::WindowDescriptor,
};

const SIZE: (u32, u32) = (1280, 720);
const WORKGROUP_SIZE: u32 = 8;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(WindowDescriptor {
            // uncomment for unthrottled FPS
            // vsync: false,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(GameOfLifeComputePlugin)
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

    commands.spawn_bundle(SpriteBundle {
        sprite: Sprite {
            custom_size: Some(Vec2::new(SIZE.0 as f32, SIZE.1 as f32)),
            ..Default::default()
        },
        texture: image.clone(),
        ..Default::default()
    });
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    commands.insert_resource(GameOfLifeImage(image));
}

pub struct GameOfLifeComputePlugin;

impl Plugin for GameOfLifeComputePlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<GameOfLifePipeline>()
            .add_system_to_stage(RenderStage::Extract, extract_game_of_life_image)
            .add_system_to_stage(RenderStage::Queue, queue_bind_group);

        let mut render_graph = render_app.world.resource_mut::<RenderGraph>();
        render_graph.add_node("game_of_life", DispatchGameOfLife::default());
        render_graph
            .add_node_edge("game_of_life", MAIN_PASS_DEPENDENCIES)
            .unwrap();
    }
}

struct GameOfLifeImage(Handle<Image>);
struct GameOfLifeImageBindGroup(BindGroup);

fn extract_game_of_life_image(mut commands: Commands, image: Res<GameOfLifeImage>) {
    commands.insert_resource(GameOfLifeImage(image.0.clone()));
}
fn queue_bind_group(
    mut commands: Commands,
    pipeline: Res<GameOfLifePipeline>,
    gpu_images: Res<RenderAssets<Image>>,
    game_of_life_image: Res<GameOfLifeImage>,
    render_device: Res<RenderDevice>,
) {
    let view = &gpu_images[&game_of_life_image.0];
    let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
        label: None,
        layout: &pipeline.texture_bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: BindingResource::TextureView(&view.texture_view),
        }],
    });
    commands.insert_resource(GameOfLifeImageBindGroup(bind_group));
}

pub struct GameOfLifePipeline {
    sim_pipeline: ComputePipeline,
    init_pipeline: ComputePipeline,
    texture_bind_group_layout: BindGroupLayout,
}

impl FromWorld for GameOfLifePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let shader_source = include_str!("../../assets/shaders/game_of_life.wgsl");
        let shader = render_device.create_shader_module(&ShaderModuleDescriptor {
            label: None,
            source: ShaderSource::Wgsl(shader_source.into()),
        });

        let texture_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: None,
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::StorageTexture {
                        access: StorageTextureAccess::ReadWrite,
                        format: TextureFormat::Rgba8Unorm,
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                }],
            });

        let pipeline_layout = render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });
        let init_pipeline = render_device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "init",
        });
        let sim_pipeline = render_device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "update",
        });

        GameOfLifePipeline {
            sim_pipeline,
            init_pipeline,
            texture_bind_group_layout,
        }
    }
}

enum Initialized {
    Default,
    No,
    Yes,
}

struct DispatchGameOfLife {
    initialized: Initialized,
}
impl Default for DispatchGameOfLife {
    fn default() -> Self {
        Self {
            initialized: Initialized::Default,
        }
    }
}
impl render_graph::Node for DispatchGameOfLife {
    fn update(&mut self, _world: &mut World) {
        match self.initialized {
            Initialized::Default => self.initialized = Initialized::No,
            Initialized::No => self.initialized = Initialized::Yes,
            Initialized::Yes => {}
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let pipeline = world.resource::<GameOfLifePipeline>();
        let texture_bind_group = &world.resource::<GameOfLifeImageBindGroup>().0;

        let mut pass = render_context
            .command_encoder
            .begin_compute_pass(&ComputePassDescriptor::default());

        if let Initialized::No = self.initialized {
            pass.set_pipeline(&pipeline.init_pipeline);
            pass.set_bind_group(0, texture_bind_group, &[]);
            pass.dispatch(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
        }

        pass.set_pipeline(&pipeline.sim_pipeline);
        pass.set_bind_group(0, texture_bind_group, &[]);
        pass.dispatch(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);

        Ok(())
    }
}
