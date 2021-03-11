use bevy_app::{AppBuilder, CoreStage, Plugin};
use bevy_asset::{AddAsset, Assets, Handle};
use bevy_ecs::{
    prelude::{Commands, Query, ResMut},
    schedule::*,
    system::*,
};
use bevy_log::info;
use bevy_math::*;
use bevy_reflect::TypeUuid;
use bevy_render::{
    mesh::*,
    pipeline::{CullMode, PipelineDescriptor, PrimitiveTopology},
    prelude::{Color, MeshBundle},
    render_graph::{base, AssetRenderResourcesNode, RenderGraph},
    renderer::RenderResources,
    shader::{Shader, ShaderStage, ShaderStages},
};
use bevy_transform::{components::GlobalTransform, TransformSystem};
/// Bevy immediate mode debug drawing:
/// This crate introduces a DebugDraw3D resource which provides functions such as `draw_line(start, end, color)`
/// Whenever such a draw_line function is called, a set of vertices is added to the DebugDraw3D objects data.
/// At the end of the frame the internal data is copied into a mesh entity for drawing and then cleared from the DebugDraw3D object.
/// With this, no persistent line drawing is possible and lines have to be added every frame (hence immediate mode).
/// For convenience a system called `debug_draw_all_gizmos` is provided that draws a coordinate gizmo for any `GlobalTransform`.
///
/// ToDo:
/// * Add more convenience functions such as `draw_arrow(start, end, head_size, color)`, `draw_circle(origin, radius, axis, color)`, `draw_aabb(min,max,color)`.
/// * Modify the shader and access the depth buffer and perform hidden-line rendering rather than a binary depth test for better line visualization.
/// * Add the `debug_draw_all_gizmos` system to the plugin, using a parameter to turn it on or off.
/// * Add transparent triangle drawing (useful to visually project a line down on a plane) and matching utility functions.
/// * Add timed or persistent drawing: This requires storing `Line` structs containing a lifetime rather than directly pushing to an array.
/// * Even though this is a debug feature, there current approach may likely not be the most performant solution and optimizations/refactoring should be applied.

pub struct DebugDrawPlugin;
impl Plugin for DebugDrawPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_asset::<DebugDraw3DMaterial>()
            .init_resource::<DebugDraw3D>()
            .add_startup_system(setup_debug_draw_3d.system())
            .add_system_to_stage(
                CoreStage::PostUpdate,
                update_debug_draw_3d
                    .system()
                    .after(TransformSystem::TransformPropagate),
            );
    }
}

/// DebugDraw3D Resource providing functions for immediate mode drawing
pub struct DebugDraw3D {
    // The mesh data is held as plain arrays here
    // If we wish to extend to more than just lines we may need multiple pairs that will later be ass, e.g. vertices_line and vertices_triangle
    vertices: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    dirty: bool,
    clear: bool,
}

impl Default for DebugDraw3D {
    fn default() -> Self {
        DebugDraw3D {
            vertices: Default::default(),
            colors: Default::default(),
            dirty: true,
            clear: true,
        }
    }
}

impl DebugDraw3D {
    pub fn draw_line(&mut self, start: Vec3, end: Vec3, color: Color) {
        self.vertices.push(start.into());
        self.vertices.push(end.into());
        self.colors.push(color.into());
        self.colors.push(color.into());
        self.set_dirty();
    }

    pub fn set_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn reset(&mut self) {
        if self.clear {
            self.vertices.clear();
            self.colors.clear();
        }
        self.dirty = false;
    }

    pub fn set_clear(&mut self, clear: bool) {
        self.clear = clear;
    }
}
/// This component marks the internal entity that does the mesh drawing.
#[derive(Default)]
struct DebugDraw3DComponent;

/// The Material holding the shader for debug drawing
#[derive(RenderResources, Default, TypeUuid)]
#[uuid = "188f0f97-60b2-476a-a749-7a0103adeeba"]
pub struct DebugDraw3DMaterial;

///This system sets up the entity holding the actual mesh for drawing as well as the render pipeline step for the shader.
fn setup_debug_draw_3d(
    mut commands: Commands,
    mut shaders: ResMut<Assets<Shader>>,
    mut materials: ResMut<Assets<DebugDraw3DMaterial>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    // Crate a shader Pipeline
    let mut p = PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(
            ShaderStage::Vertex,
            include_str!("shaders/debugDrawLine.vert"),
        )),
        fragment: Some(shaders.add(Shader::from_glsl(
            ShaderStage::Fragment,
            include_str!("shaders/debugDrawLine.frag"),
        ))),
    });
    p.primitive.topology = PrimitiveTopology::LineList;
    p.primitive.cull_mode = CullMode::None;

    // add the material to the pipeline
    render_graph.add_system_node(
        "debug_draw_3d",
        AssetRenderResourcesNode::<DebugDraw3DMaterial>::new(false),
    );
    // connect that node stage the MAIN_PASS node
    render_graph
        .add_node_edge("debug_draw_3d", base::node::MAIN_PASS)
        .unwrap();

    let material_instance = materials.add(DebugDraw3DMaterial {});

    // Spawn a entity that will do the debug drawing with its mesh
    commands
        .spawn(MeshBundle::default())
        .with(material_instance)
        .with(DebugDraw3DComponent::default())
        .current_entity()
        .unwrap();

    info!("Loaded debug lines plugin.");
}

/// This system updates the debug draw Entity with the data from
fn update_debug_draw_3d(
    mut debug_draw: ResMut<DebugDraw3D>,
    mut meshes: ResMut<Assets<Mesh>>,
    query: Query<(&DebugDraw3DComponent, &Handle<Mesh>)>,
) {
    if !debug_draw.dirty {
        return;
    } else {
        for (_, mesh) in query.iter() {
            if let Some(mesh) = meshes.get_mut(mesh) {
                mesh.set_attribute(
                    Mesh::ATTRIBUTE_POSITION,
                    VertexAttributeValues::Float3(debug_draw.vertices.clone()),
                );
                mesh.set_attribute(
                    "Vertex_Color",
                    VertexAttributeValues::Float4(debug_draw.colors.clone()),
                );
            }
        }
    }
    debug_draw.reset();
}

pub fn debug_draw_all_gizmos(mut debug_draw: ResMut<DebugDraw3D>, query: Query<&GlobalTransform>) {
    for transform in query.iter() {
        debug_draw.draw_line(
            transform.translation,
            transform.translation + transform.local_x(),
            Color::RED,
        );
        debug_draw.draw_line(
            transform.translation,
            transform.translation + transform.local_y(),
            Color::GREEN,
        );
        debug_draw.draw_line(
            transform.translation,
            transform.translation + transform.local_z(),
            Color::BLUE,
        );
    }
}
