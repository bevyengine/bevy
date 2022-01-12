use std::num::NonZeroU64;

use bevy::{
    core_pipeline::draw_2d_graph,
    prelude::*,
    render::{
        render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotInfo, SlotType},
        render_resource::{
            std430::{AsStd430, Std430},
            *,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        texture::BevyDefault,
        view::{ExtractedView, ViewTarget},
        RenderApp, RenderStage,
    },
    utils::{HashMap, HashSet},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(BackgroundRendererPlugin)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands
        .spawn_bundle(OrthographicCameraBundle::new_2d())
        .insert(CustomRenderingSettings {
            color: Vec4::new(0.8, 0.1, 0.8, 1.),
        });
}

// Attaching this to a camera will run the pipeline below on the camera's
// view before the main pass
#[derive(Component, Clone)]
pub struct CustomRenderingSettings {
    pub color: Vec4,
}

// Pipeline

#[derive(AsStd430)]
pub struct BackgroundUniform {
    color: Vec4,
    time: f32,
    resolution: Vec2,
}

pub struct BackgroundPipeline {
    shader: Handle<Shader>,
    bind_group_layout: BindGroupLayout,
    bind_groups: HashMap<Entity, (BindGroup, Buffer)>,
    msaa: Msaa,
}

impl FromWorld for BackgroundPipeline {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let shader = asset_server.load("shaders/background.wgsl");

        let render_device = world.get_resource::<RenderDevice>().unwrap();

        // The default msaa samples of 1 is incompatable with what draw_2d's main pass expects,
        // so we default to 4 if not specified
        let msaa = world.get_resource::<Msaa>().unwrap_or(&Msaa { samples: 4 });

        // uniform bindings
        let bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("background_bind_group_layout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            NonZeroU64::new(BackgroundUniform::std430_size_static() as u64)
                                .unwrap(),
                        ),
                    },
                    count: None,
                }],
            });

        Self {
            shader,
            bind_group_layout,
            bind_groups: HashMap::default(),
            msaa: msaa.clone(),
        }
    }
}

impl SpecializedPipeline for BackgroundPipeline {
    type Key = ();

    fn specialize(&self, _key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("background_pipeline_desc".into()),
            layout: Some(vec![self.bind_group_layout.clone()]),
            vertex: VertexState {
                shader: self.shader.clone(),
                entry_point: "vert".into(),
                shader_defs: vec![],
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone_weak(),
                entry_point: "frag".into(),
                shader_defs: vec![],
                targets: vec![TextureFormat::bevy_default().into()],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState {
                count: self.msaa.samples,
                ..Default::default()
            },
        }
    }
}

// Extraction - Copy components and resources from World to the RenderWorld
pub struct ExtractedTime(f32);

pub fn extract_time(mut commands: Commands, time: Res<Time>) {
    commands.insert_resource(ExtractedTime(time.time_since_startup().as_secs_f32()));
}

pub fn extract_custom_rendering_settings(
    mut commands: Commands,
    query: Query<(Entity, &CustomRenderingSettings)>,
) {
    // Our query is executed on the normal world whereas commands are executed on the
    // render world. We get_or_spawn on the same entity id as the normal world here
    // so that our component is attached to the same entity both the camera and view
    // will be
    query.for_each(|(e, s)| {
        commands.get_or_spawn(e).insert(s.clone());
    })
}

// Prepare our uniform buffer for each available camera
pub fn write_background_uniforms(
    mut pipeline: ResMut<BackgroundPipeline>,
    render_queue: Res<RenderQueue>,
    render_device: Res<RenderDevice>,
    time: Res<ExtractedTime>,

    query: Query<(Entity, &CustomRenderingSettings, &ExtractedView)>,
) {
    // get or create our buffer and bind group for this camera
    let pipeline = &mut *pipeline;
    query.for_each(|(e, bg_settings, camera_view)| {
        // Make our buffer and bind groups if they aren't cached
        pipeline.bind_groups.entry(e).or_insert_with(|| {
            let buffer = render_device.create_buffer(&BufferDescriptor {
                label: Some(format!("background_uniform_buffer_{}", e.id()).as_str()),
                size: BackgroundUniform::std430_size_static() as u64,
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                label: Some(format!("background_uniform_bind_group_{}", e.id()).as_str()),
                layout: &pipeline.bind_group_layout.clone(),
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });

            (bind_group, buffer)
        });

        // Push our uniforms
        let (_bind_group, buffer) = pipeline.bind_groups.get(&e).unwrap();

        let uniforms = BackgroundUniform {
            color: bg_settings.color,
            time: time.0,
            resolution: Vec2::new(camera_view.width as f32, camera_view.height as f32),
        };
        render_queue.write_buffer(buffer, 0, uniforms.as_std430().as_bytes());
    });
}

// Pipeline caching

pub struct CompiledBackgroundPipeline(pub CachedPipelineId);

pub fn compile_pipeline(
    mut commands: Commands,
    pipeline: Res<BackgroundPipeline>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
) {
    let pipeline = pipeline_cache.queue(pipeline.specialize(()));
    commands.insert_resource(CompiledBackgroundPipeline(pipeline));
}

// Cleanup unused bind groups
fn cleanup_bind_groups(
    mut pipeline: ResMut<BackgroundPipeline>,
    query: Query<Entity, With<CustomRenderingSettings>>,
) {
    let ids: HashSet<Entity> = query.iter().collect();

    pipeline.bind_groups.retain(|e, _| ids.contains(e));
}

// Our custom Node to insert into the rendering graph
pub struct BackgroundNode {
    query: QueryState<(
        Entity,
        &'static ViewTarget, // contains what we're rendering to
    )>,
}

impl BackgroundNode {
    const NAME: &'static str = "background";
    const VIEW_ENTITY: &'static str = "view_entity";
}

impl FromWorld for BackgroundNode {
    fn from_world(world: &mut World) -> Self {
        // we store our query state because we won't be able to create one while drawing
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for BackgroundNode {
    fn input(&self) -> Vec<SlotInfo> {
        // we want bevy's rendering engine to pass us the view we're rendering to
        // so our node can be called for each applicable view
        vec![SlotInfo::new(Self::VIEW_ENTITY, SlotType::Entity)]
    }
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        render_graph_context: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = render_graph_context.get_input_entity(Self::VIEW_ENTITY)?;

        let (e, target) = match self.query.get_manual(world, view_entity) {
            Ok(v) => v,
            Err(_) => return Ok(()), // if our query fails, just don't render
        };

        let background_pipeline = world.get_resource::<BackgroundPipeline>().unwrap();
        let pipeline_cache = world.get_resource::<RenderPipelineCache>().unwrap();
        let compiled_pipeline = world
            .get_resource::<CompiledBackgroundPipeline>()
            .unwrap()
            .0;

        let pipeline = match pipeline_cache.get(compiled_pipeline) {
            Some(p) => p,
            None => return Ok(()),
        };

        let (bind_group, _buffer) = background_pipeline.bind_groups.get(&e).unwrap();

        // draw
        let mut pass = render_context
            .command_encoder
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("background_render_pass"),
                color_attachments: &[target.get_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                })],
                depth_stencil_attachment: None,
            });

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..6, 0..1);

        Ok(())
    }
}

// Plugin
pub struct BackgroundRendererPlugin;

impl Plugin for BackgroundRendererPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        let pipeline = BackgroundPipeline::from_world(&mut render_app.world);

        // insert our pipeline and associated systems
        render_app
            .insert_resource(pipeline)
            .add_system_to_stage(RenderStage::Extract, extract_time)
            .add_system_to_stage(RenderStage::Extract, extract_custom_rendering_settings)
            .add_system_to_stage(RenderStage::Prepare, compile_pipeline)
            .add_system_to_stage(RenderStage::Prepare, write_background_uniforms)
            .add_system_to_stage(RenderStage::Cleanup, cleanup_bind_groups);

        // insert our render node to the graph
        let background_node = BackgroundNode::from_world(&mut render_app.world);
        let mut render_graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();

        // this should work just fine for the 3d graph as well
        let graph_2d = render_graph.get_sub_graph_mut(draw_2d_graph::NAME).unwrap();

        graph_2d.add_node(BackgroundNode::NAME, background_node);
        graph_2d
            .add_node_edge(BackgroundNode::NAME, draw_2d_graph::node::MAIN_PASS)
            .unwrap();
        graph_2d
            .add_slot_edge(
                graph_2d.input_node().unwrap().id,
                draw_2d_graph::input::VIEW_ENTITY,
                BackgroundNode::NAME,
                BackgroundNode::VIEW_ENTITY,
            )
            .unwrap();
    }
}
