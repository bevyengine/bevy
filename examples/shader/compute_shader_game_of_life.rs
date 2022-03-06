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
use std::borrow::Cow;
use std::ops::Deref;

const SIZE: (u32, u32) = (1280, 720);
const WORKGROUP_SIZE: u32 = 8;

// the layout descriptor of the bind group of game of life compute shader
const BIND_GROUP_LAYOUT: BindGroupLayoutDescriptor = BindGroupLayoutDescriptor {
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
};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(WindowDescriptor {
            // uncomment for unthrottled FPS
            // vsync: false,
            ..default()
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
            ..default()
        },
        texture: image.clone(),
        ..default()
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
            .init_resource::<SpecializedComputePipelines<GameOfLifePipeline>>()
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

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum GameOfLifePipelineKey {
    Init,
    Update,
}

pub struct GameOfLifePipeline {
    texture_bind_group_layout: BindGroupLayout,
    shader: Handle<Shader>,
}

impl FromWorld for GameOfLifePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let asset_server = world.resource::<AssetServer>();

        let texture_bind_group_layout = render_device.create_bind_group_layout(&BIND_GROUP_LAYOUT);
        let shader = asset_server.load("shaders/game_of_life.wgsl");

        GameOfLifePipeline {
            texture_bind_group_layout,
            shader,
        }
    }
}

impl SpecializedComputePipeline for GameOfLifePipeline {
    type Key = GameOfLifePipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        let entry_point = match key {
            GameOfLifePipelineKey::Init => "init",
            GameOfLifePipelineKey::Update => "update",
        };

        ComputePipelineDescriptor {
            label: None,
            layout: Some(vec![self.texture_bind_group_layout.clone()]),
            shader: self.shader.clone(),
            shader_defs: vec![],
            entry_point: Cow::from(entry_point),
        }
    }
}

enum GameOfLifeState {
    NotReady,
    Init,
    Update,
}

struct DispatchGameOfLife {
    state: GameOfLifeState,
    init_pipeline: CachedPipelineId,
    update_pipeline: CachedPipelineId,
}

impl Default for DispatchGameOfLife {
    fn default() -> Self {
        Self {
            state: GameOfLifeState::NotReady,
            init_pipeline: CachedPipelineId::INVALID,
            update_pipeline: CachedPipelineId::INVALID,
        }
    }
}

impl render_graph::Node for DispatchGameOfLife {
    fn update(&mut self, world: &mut World) {
        let world = world.cell();

        let mut pipelines = world
            .get_resource_mut::<SpecializedComputePipelines<GameOfLifePipeline>>()
            .unwrap();
        let mut pipeline_cache = world.get_resource_mut::<PipelineCache>().unwrap();
        let game_of_life_pipeline = world.get_resource::<GameOfLifePipeline>().unwrap();

        self.init_pipeline = pipelines.specialize(
            &mut pipeline_cache,
            game_of_life_pipeline.deref(),
            GameOfLifePipelineKey::Init,
        );
        self.update_pipeline = pipelines.specialize(
            &mut pipeline_cache,
            game_of_life_pipeline.deref(),
            GameOfLifePipelineKey::Update,
        );

        // if the corresponding pipeline has loaded, transition to the next stage
        match self.state {
            GameOfLifeState::NotReady => {
                if let CachedPipelineState::Ok(_) = pipeline_cache.get_state(self.init_pipeline) {
                    self.state = GameOfLifeState::Init
                }
            }
            GameOfLifeState::Init => {
                if let CachedPipelineState::Ok(_) = pipeline_cache.get_state(self.update_pipeline) {
                    self.state = GameOfLifeState::Update
                }
            }
            GameOfLifeState::Update => {}
        }
    }

    fn run(
        &self,
        _graph: &mut render_graph::RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), render_graph::NodeRunError> {
        let texture_bind_group = &world.resource::<GameOfLifeImageBindGroup>().0;
        let pipeline_cache = world.resource::<PipelineCache>();

        let mut pass = render_context
            .command_encoder
            .begin_compute_pass(&ComputePassDescriptor::default());

        pass.set_bind_group(0, texture_bind_group, &[]);

        // select the pipeline based on the current state
        match self.state {
            GameOfLifeState::NotReady => {}
            GameOfLifeState::Init => {
                let init_pipeline = pipeline_cache
                    .get_compute_pipeline(self.init_pipeline)
                    .unwrap();
                pass.set_pipeline(init_pipeline);
                pass.dispatch(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            }
            GameOfLifeState::Update => {
                let update_pipeline = pipeline_cache
                    .get_compute_pipeline(self.update_pipeline)
                    .unwrap();
                pass.set_pipeline(update_pipeline);
                pass.dispatch(SIZE.0 / WORKGROUP_SIZE, SIZE.1 / WORKGROUP_SIZE, 1);
            }
        }

        Ok(())
    }
}
