use crate::Node;
use bevy_asset::{Assets, HandleUntyped};
use bevy_ecs::world::World;
use bevy_reflect::TypeUuid;
use bevy_render::{
    camera::ActiveCameras,
    pass::{
        LoadOp, Operations, PassDescriptor, RenderPassColorAttachmentDescriptor,
        RenderPassDepthStencilAttachmentDescriptor, TextureAttachment,
    },
    pipeline::*,
    prelude::Msaa,
    render_graph::{
        base, CameraNode, PassNode, RenderGraph, RenderResourcesNode, WindowSwapChainNode,
        WindowTextureNode,
    },
    shader::{Shader, ShaderStage, ShaderStages},
    texture::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsage},
};
use bevy_window::WindowId;
use node::UI_PASS;

pub const UI_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 3234320022263993878);

pub fn build_ui_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    PipelineDescriptor {
        depth_stencil: Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Less,
            stencil: StencilState {
                front: StencilFaceState::IGNORE,
                back: StencilFaceState::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
            bias: DepthBiasState {
                constant: 0,
                slope_scale: 0.0,
                clamp: 0.0,
            },
            clamp_depth: false,
        }),
        color_target_states: vec![ColorTargetState {
            format: TextureFormat::default(),
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
                include_str!("ui.vert"),
            )),
            fragment: Some(shaders.add(Shader::from_glsl(
                ShaderStage::Fragment,
                include_str!("ui.frag"),
            ))),
        })
    }
}

pub mod node {
    pub const CAMERA_UI: &str = "camera_ui";
    pub const NODE: &str = "node";
    pub const UI_PASS: &str = "ui_pass";
    pub const UI_DEPTH_TEXTURE: &str = "ui_depth";
}

pub mod camera {
    pub const CAMERA_UI: &str = "CameraUi";
}

pub(crate) fn add_ui_graph(world: &mut World) {
    let world = world.cell();
    let mut graph = world.get_resource_mut::<RenderGraph>().unwrap();
    let mut pipelines = world
        .get_resource_mut::<Assets<PipelineDescriptor>>()
        .unwrap();
    let mut shaders = world.get_resource_mut::<Assets<Shader>>().unwrap();
    let mut active_cameras = world.get_resource_mut::<ActiveCameras>().unwrap();
    let msaa = world.get_resource::<Msaa>().unwrap();

    pipelines.set_untracked(UI_PIPELINE_HANDLE, build_ui_pipeline(&mut shaders));

    let color_attachments = if graph.get_node_id(base::node::MAIN_RESOLVE_PASS).is_ok()
        || graph.get_node_id(base::node::POST_PASS).is_ok()
    {
        vec![RenderPassColorAttachmentDescriptor {
            attachment: TextureAttachment::Input("color_attachment".to_string()),
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Load,
                store: true,
            },
        }]
    } else {
        vec![msaa.color_attachment_descriptor(
            TextureAttachment::Input("color_attachment".to_string()),
            TextureAttachment::Input("color_resolve_target".to_string()),
            Operations {
                load: LoadOp::Load,
                store: true,
            },
        )]
    };

    let mut ui_pass_node = PassNode::<&Node>::new(PassDescriptor {
        color_attachments,
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachmentDescriptor {
            attachment: TextureAttachment::Input("depth".to_string()),
            depth_ops: Some(Operations {
                load: LoadOp::Clear(1.0),
                store: true,
            }),
            stencil_ops: None,
        }),
        sample_count: msaa.samples,
    });

    ui_pass_node.add_camera(camera::CAMERA_UI);
    graph.add_node(node::UI_PASS, ui_pass_node);

    if let Ok(previous_node) = graph
        .get_node_id(base::node::POST_PASS)
        .or_else(|_| graph.get_node_id(base::node::MAIN_RESOLVE_PASS))
    {
        graph
            .add_slot_edge(
                base::node::PRIMARY_SWAP_CHAIN,
                WindowSwapChainNode::OUT_TEXTURE,
                node::UI_PASS,
                "color_attachment",
            )
            .unwrap();

        // create own depth texture
        let ui_depth_texture_node = WindowTextureNode::new(
            WindowId::primary(),
            TextureDescriptor {
                size: Extent3d::new(1, 1, 1),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Depth32Float,
                usage: TextureUsage::OUTPUT_ATTACHMENT,
            },
            None,
            None,
        );
        graph.add_node(node::UI_DEPTH_TEXTURE, ui_depth_texture_node);

        graph
            .add_slot_edge(
                node::UI_DEPTH_TEXTURE,
                WindowTextureNode::OUT_TEXTURE,
                UI_PASS,
                "depth",
            )
            .unwrap();

        graph.add_node_edge(previous_node, node::UI_PASS).unwrap();
    } else {
        graph
            .add_slot_edge(
                base::node::PRIMARY_SWAP_CHAIN,
                WindowSwapChainNode::OUT_TEXTURE,
                node::UI_PASS,
                if msaa.samples > 1 {
                    "color_resolve_target"
                } else {
                    "color_attachment"
                },
            )
            .unwrap();

        graph
            .add_slot_edge(
                base::node::MAIN_DEPTH_TEXTURE,
                WindowTextureNode::OUT_TEXTURE,
                node::UI_PASS,
                "depth",
            )
            .unwrap();

        if msaa.samples > 1 {
            graph
                .add_slot_edge(
                    base::node::MAIN_SAMPLED_COLOR_ATTACHMENT,
                    WindowSwapChainNode::OUT_TEXTURE,
                    node::UI_PASS,
                    "color_attachment",
                )
                .unwrap();
        }

        // ensure ui pass runs after main pass
        graph
            .add_node_edge(base::node::MAIN_PASS, node::UI_PASS)
            .unwrap();
    }

    // setup ui camera
    graph.add_system_node(node::CAMERA_UI, CameraNode::new(camera::CAMERA_UI));
    graph.add_node_edge(node::CAMERA_UI, node::UI_PASS).unwrap();
    graph.add_system_node(node::NODE, RenderResourcesNode::<Node>::new(true));
    graph.add_node_edge(node::NODE, node::UI_PASS).unwrap();
    active_cameras.add(camera::CAMERA_UI);
}
