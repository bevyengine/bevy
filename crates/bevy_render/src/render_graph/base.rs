use super::{
    fullscreen_pass_node, CameraNode, FullscreenPassNode, PassNode, RenderGraph, SharedBuffersNode,
    TextureCopyNode, WindowSwapChainNode, WindowTextureNode,
};
use crate::{
    pass::{
        LoadOp, Operations, PassDescriptor, RenderPassColorAttachmentDescriptor,
        RenderPassDepthStencilAttachmentDescriptor, TextureAttachment,
    },
    pipeline::{
        BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrite, PipelineDescriptor,
    },
    shader::{Shader, ShaderStage, ShaderStages},
    texture::{
        Extent3d, SamplerDescriptor, TextureDescriptor, TextureDimension, TextureFormat,
        TextureUsage,
    },
    Color,
};
use bevy_asset::Assets;
use bevy_ecs::{reflect::ReflectComponent, world::World};
use bevy_reflect::Reflect;
use bevy_window::WindowId;

/// A component that indicates that an entity should be drawn in the "main pass"
#[derive(Clone, Debug, Default, Reflect)]
#[reflect(Component)]
pub struct MainPass;

#[derive(Debug)]
pub struct Msaa {
    pub samples: u32,
}

impl Default for Msaa {
    fn default() -> Self {
        Self { samples: 1 }
    }
}

impl Msaa {
    pub fn color_attachment_descriptor(
        &self,
        attachment: TextureAttachment,
        resolve_target: TextureAttachment,
        ops: Operations<Color>,
    ) -> RenderPassColorAttachmentDescriptor {
        if self.samples > 1 {
            RenderPassColorAttachmentDescriptor {
                attachment,
                resolve_target: Some(resolve_target),
                ops,
            }
        } else {
            RenderPassColorAttachmentDescriptor {
                attachment,
                resolve_target: None,
                ops,
            }
        }
    }
}

#[derive(Debug)]
pub struct BaseRenderGraphConfig {
    pub add_2d_camera: bool,
    pub add_3d_camera: bool,
    pub add_main_depth_texture: bool,
    pub add_main_pass: bool,
    pub add_resolve_pass: bool,
    pub add_post_pass: bool,
    pub connect_main_pass_to_swapchain: bool,
    pub connect_main_pass_to_main_depth_texture: bool,
}

pub mod node {
    pub const PRIMARY_SWAP_CHAIN: &str = "swapchain";
    pub const CAMERA_3D: &str = "camera_3d";
    pub const CAMERA_2D: &str = "camera_2d";
    pub const TEXTURE_COPY: &str = "texture_copy";
    pub const SHARED_BUFFERS: &str = "shared_buffers";
    pub const MAIN_DEPTH_TEXTURE: &str = "main_pass_depth_texture";
    pub const MAIN_RENDER_TEXTURE: &str = "main_pass_render_texture";
    pub const MAIN_SAMPLED_COLOR_ATTACHMENT: &str = "main_pass_sampled_color_attachment";
    pub const MAIN_SAMPLED_DEPTH_STENCIL_ATTACHMENT: &str =
        "main_pass_sampled_depth_stencil_attachment";
    pub const MAIN_PASS: &str = "main_pass";
    pub const MAIN_RESOLVE_PASS: &str = "main_resolve_pass";
    pub const POST_PASS: &str = "post_pass";
}
pub mod camera {
    pub const CAMERA_3D: &str = "Camera3d";
    pub const CAMERA_2D: &str = "Camera2d";
}

impl Default for BaseRenderGraphConfig {
    fn default() -> Self {
        BaseRenderGraphConfig {
            add_2d_camera: true,
            add_3d_camera: true,
            add_main_pass: true,
            add_resolve_pass: true,
            add_post_pass: true,
            add_main_depth_texture: true,
            connect_main_pass_to_swapchain: true,
            connect_main_pass_to_main_depth_texture: true,
        }
    }
}

fn setup_utility_nodes(config: &BaseRenderGraphConfig, graph: &mut RenderGraph) {
    graph.add_node(node::TEXTURE_COPY, TextureCopyNode::default());
    if config.add_3d_camera {
        graph.add_system_node(node::CAMERA_3D, CameraNode::new(camera::CAMERA_3D));
    }

    if config.add_2d_camera {
        graph.add_system_node(node::CAMERA_2D, CameraNode::new(camera::CAMERA_2D));
    }

    graph.add_node(node::SHARED_BUFFERS, SharedBuffersNode::default());
}

fn setup_textures(config: &BaseRenderGraphConfig, msaa: &Msaa, graph: &mut RenderGraph) {
    // Always create main swap chain
    graph.add_node(
        node::PRIMARY_SWAP_CHAIN,
        WindowSwapChainNode::new(WindowId::primary()),
    );

    if config.add_post_pass {
        // Setup render textures
        let main_render_texture_node = WindowTextureNode::new(
            WindowId::primary(),
            TextureDescriptor {
                size: Extent3d::new(1, 1, 1),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Bgra8Unorm,
                usage: TextureUsage::OUTPUT_ATTACHMENT | TextureUsage::SAMPLED,
            },
            Some(SamplerDescriptor::default()),
            None,
        );

        graph.add_node(node::MAIN_RENDER_TEXTURE, main_render_texture_node);
    } else {
        // No post pass
        if config.add_main_depth_texture {
            graph.add_node(
                node::MAIN_DEPTH_TEXTURE,
                WindowTextureNode::new(
                    WindowId::primary(),
                    TextureDescriptor {
                        size: Extent3d {
                            depth: 1,
                            width: 1,
                            height: 1,
                        },
                        mip_level_count: 1,
                        sample_count: msaa.samples,
                        dimension: TextureDimension::D2,
                        format: TextureFormat::Depth32Float, /* PERF: vulkan docs recommend using 24
                                                              * bit depth for better performance */
                        usage: TextureUsage::OUTPUT_ATTACHMENT,
                    },
                    None,
                    None,
                ),
            );
        }
    }

    if msaa.samples > 1 {
        graph.add_node(
            node::MAIN_SAMPLED_COLOR_ATTACHMENT,
            WindowTextureNode::new(
                WindowId::primary(),
                TextureDescriptor {
                    size: Extent3d {
                        depth: 1,
                        width: 1,
                        height: 1,
                    },
                    mip_level_count: 1,
                    sample_count: msaa.samples,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Bgra8Unorm,
                    usage: TextureUsage::OUTPUT_ATTACHMENT | TextureUsage::SAMPLED,
                },
                None,
                None,
            ),
        );

        graph.add_node(
            node::MAIN_SAMPLED_DEPTH_STENCIL_ATTACHMENT,
            WindowTextureNode::new(
                WindowId::primary(),
                TextureDescriptor {
                    size: Extent3d {
                        depth: 1,
                        width: 1,
                        height: 1,
                    },
                    mip_level_count: 1,
                    sample_count: msaa.samples,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Depth32Float, /* PERF: vulkan docs recommend using 24
                                                          * bit depth for better performance */
                    usage: TextureUsage::OUTPUT_ATTACHMENT,
                },
                None,
                None,
            ),
        );
    }
}

fn setup_main_pass(config: &BaseRenderGraphConfig, msaa: &Msaa, graph: &mut RenderGraph) {
    let color_attachments = if config.add_resolve_pass && msaa.samples > 1 {
        // Resolve happens during separate pass
        vec![RenderPassColorAttachmentDescriptor {
            attachment: TextureAttachment::Input("color_attachment".to_string()),
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::rgb(0.1, 0.2, 0.3)),
                store: true,
            },
        }]
    } else {
        vec![msaa.color_attachment_descriptor(
            TextureAttachment::Input("color_attachment".to_string()),
            TextureAttachment::Input("color_resolve_target".to_string()),
            Operations {
                load: LoadOp::Clear(Color::rgb(0.1, 0.1, 0.1)),
                store: true,
            },
        )]
    };

    let depth_stencil_attachment = Some(RenderPassDepthStencilAttachmentDescriptor {
        attachment: TextureAttachment::Input("depth".to_string()),
        depth_ops: Some(Operations {
            load: LoadOp::Clear(1.0),
            store: true,
        }),
        stencil_ops: None,
    });

    let mut main_pass_node = PassNode::<&MainPass>::new(PassDescriptor {
        color_attachments,
        depth_stencil_attachment,
        sample_count: msaa.samples,
    });

    main_pass_node.use_default_clear_color(0);

    if config.add_3d_camera {
        main_pass_node.add_camera(camera::CAMERA_3D);
    }

    if config.add_2d_camera {
        main_pass_node.add_camera(camera::CAMERA_2D);
    }

    graph.add_node(node::MAIN_PASS, main_pass_node);

    graph
        .add_node_edge(node::TEXTURE_COPY, node::MAIN_PASS)
        .unwrap();
    graph
        .add_node_edge(node::SHARED_BUFFERS, node::MAIN_PASS)
        .unwrap();

    if config.add_3d_camera {
        graph
            .add_node_edge(node::CAMERA_3D, node::MAIN_PASS)
            .unwrap();
    }

    if config.add_2d_camera {
        graph
            .add_node_edge(node::CAMERA_2D, node::MAIN_PASS)
            .unwrap();
    }

    if config.add_post_pass || config.add_resolve_pass {
        if msaa.samples > 1 {
            graph
                .add_slot_edge(
                    node::MAIN_SAMPLED_COLOR_ATTACHMENT,
                    WindowTextureNode::OUT_TEXTURE,
                    node::MAIN_PASS,
                    "color_attachment",
                )
                .unwrap();
            graph
                .add_slot_edge(
                    node::MAIN_SAMPLED_DEPTH_STENCIL_ATTACHMENT,
                    WindowTextureNode::OUT_TEXTURE,
                    node::MAIN_PASS,
                    "depth",
                )
                .unwrap();
        } else {
            graph
                .add_slot_edge(
                    node::MAIN_RENDER_TEXTURE,
                    WindowTextureNode::OUT_TEXTURE,
                    node::MAIN_PASS,
                    "color_attachment",
                )
                .unwrap();
            graph
                .add_slot_edge(
                    node::MAIN_DEPTH_TEXTURE,
                    WindowTextureNode::OUT_TEXTURE,
                    node::MAIN_PASS,
                    "depth",
                )
                .unwrap();
        }
    } else if config.add_main_pass {
        if msaa.samples > 1 {
            graph
                .add_slot_edge(
                    node::MAIN_SAMPLED_COLOR_ATTACHMENT,
                    WindowTextureNode::OUT_TEXTURE,
                    node::MAIN_PASS,
                    "color_attachment",
                )
                .unwrap();
        }

        if config.connect_main_pass_to_swapchain {
            graph
                .add_slot_edge(
                    node::PRIMARY_SWAP_CHAIN,
                    WindowSwapChainNode::OUT_TEXTURE,
                    node::MAIN_PASS,
                    if msaa.samples > 1 {
                        "color_resolve_target"
                    } else {
                        "color_attachment"
                    },
                )
                .unwrap();
        }
        if config.add_main_depth_texture && config.connect_main_pass_to_main_depth_texture {
            graph
                .add_slot_edge(
                    node::MAIN_DEPTH_TEXTURE,
                    WindowTextureNode::OUT_TEXTURE,
                    node::MAIN_PASS,
                    "depth",
                )
                .unwrap();
        }
    }
}

fn setup_resolve_pass(config: &BaseRenderGraphConfig, msaa: &Msaa, graph: &mut RenderGraph) {
    let resolve_pass_node = PassNode::<()>::new(PassDescriptor {
        color_attachments: vec![RenderPassColorAttachmentDescriptor {
            attachment: TextureAttachment::Input("color_attachment".to_string()),
            resolve_target: Some(TextureAttachment::Input("color_resolve_target".to_string())),
            ops: Operations {
                load: LoadOp::Load,
                store: true,
            },
        }],
        depth_stencil_attachment: None,
        sample_count: msaa.samples,
    });

    graph.add_node(node::MAIN_RESOLVE_PASS, resolve_pass_node);

    graph
        .add_node_edge(node::MAIN_PASS, node::MAIN_RESOLVE_PASS)
        .unwrap();
    graph
        .add_slot_edge(
            node::MAIN_SAMPLED_COLOR_ATTACHMENT,
            WindowTextureNode::OUT_TEXTURE,
            node::MAIN_RESOLVE_PASS,
            "color_attachment",
        )
        .unwrap();

    if config.add_post_pass {
        // output to render texture
        graph
            .add_slot_edge(
                node::MAIN_RENDER_TEXTURE,
                WindowTextureNode::OUT_TEXTURE,
                node::MAIN_RESOLVE_PASS,
                "color_resolve_target",
            )
            .unwrap();
    } else {
        // output directly to swap chain
        graph
            .add_slot_edge(
                node::PRIMARY_SWAP_CHAIN,
                WindowSwapChainNode::OUT_TEXTURE,
                node::MAIN_RESOLVE_PASS,
                "color_resolve_target",
            )
            .unwrap();
    }
}

fn setup_post_pass(
    config: &BaseRenderGraphConfig,
    msaa: &Msaa,
    shaders: &mut Assets<Shader>,
    pipelines: &mut Assets<PipelineDescriptor>,
    graph: &mut RenderGraph,
) {
    let pipeline_descriptor = PipelineDescriptor {
        depth_stencil: None,
        color_target_states: vec![ColorTargetState {
            format: TextureFormat::Bgra8UnormSrgb,
            color_blend: BlendState {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha_blend: BlendState {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            write_mask: ColorWrite::ALL,
        }],
        ..PipelineDescriptor::new(ShaderStages {
            vertex: shaders.add(Shader::from_glsl(
                ShaderStage::Vertex,
                fullscreen_pass_node::shaders::VERTEX_SHADER,
            )),
            fragment: Some(shaders.add(Shader::from_glsl(
                ShaderStage::Fragment,
                fullscreen_pass_node::shaders::REINHARD_FRAGMENT_SHADER,
            ))),
        })
    };

    let pipeline_handle = pipelines.add(pipeline_descriptor);

    let pass_descriptor = PassDescriptor {
        color_attachments: vec![RenderPassColorAttachmentDescriptor {
            attachment: TextureAttachment::Input("color_attachment".to_string()),
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(Color::rgb(0.1, 0.2, 0.3)),
                store: true,
            },
        }],
        depth_stencil_attachment: None,
        sample_count: 1,
    };

    let post_pass_node = FullscreenPassNode::new(
        pass_descriptor,
        pipeline_handle,
        vec!["color_texture".into()],
    );

    graph.add_node(node::POST_PASS, post_pass_node);

    graph
        .add_node_edge(node::MAIN_PASS, node::POST_PASS)
        .unwrap();

    if config.add_resolve_pass && msaa.samples > 1 {
        graph
            .add_node_edge(node::MAIN_RESOLVE_PASS, node::POST_PASS)
            .unwrap();
    }

    graph
        .add_slot_edge(
            node::PRIMARY_SWAP_CHAIN,
            WindowSwapChainNode::OUT_TEXTURE,
            node::POST_PASS,
            "color_attachment",
        )
        .unwrap();

    graph
        .add_slot_edge(
            node::MAIN_RENDER_TEXTURE,
            WindowTextureNode::OUT_TEXTURE,
            node::POST_PASS,
            "color_texture",
        )
        .unwrap();

    graph
        .add_slot_edge(
            node::MAIN_RENDER_TEXTURE,
            WindowTextureNode::OUT_SAMPLER,
            node::POST_PASS,
            "color_texture_sampler",
        )
        .unwrap();
}

/// The "base render graph" provides a core set of render graph nodes which can be used to build any
/// graph. By itself this graph doesn't do much, but it allows Render plugins to interop with each
/// other by having a common set of nodes. It can be customized using `BaseRenderGraphConfig`.
pub(crate) fn add_base_graph(config: &BaseRenderGraphConfig, world: &mut World) {
    let world = world.cell();
    let mut graph = world.get_resource_mut::<RenderGraph>().unwrap();
    let msaa = world.get_resource::<Msaa>().unwrap();

    // post pass requires main pass
    debug_assert!(!config.add_post_pass || config.add_main_pass);
    // debug_assert!(!config.add_resolve_pass || msaa.samples > 1);

    // Set up various nodes
    setup_utility_nodes(config, &mut *graph);

    // Set up textures
    setup_textures(config, &*msaa, &mut graph);

    // Set up passes

    if config.add_main_pass {
        setup_main_pass(config, &*msaa, &mut graph);
    }

    if config.add_resolve_pass && msaa.samples > 1 {
        setup_resolve_pass(config, &*msaa, &mut graph);
    }

    if config.add_post_pass {
        let mut shaders = world.get_resource_mut::<Assets<Shader>>().unwrap();
        let mut pipelines = world
            .get_resource_mut::<Assets<PipelineDescriptor>>()
            .unwrap();

        setup_post_pass(config, &*msaa, &mut *shaders, &mut *pipelines, &mut graph);
    }
}
