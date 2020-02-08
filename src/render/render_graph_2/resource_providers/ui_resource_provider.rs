use crate::asset::MeshType;
use crate::prelude::Node;
use crate::render::render_graph_2::resource_name;
use crate::render::render_graph_2::ResourceProvider;
use crate::{
    asset::{Asset, AssetStorage, Handle, Mesh},
    ecs, math,
    render::render_graph_2::Renderer,
};
use bevy_transform::prelude::Parent;
use legion::prelude::*;
use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(Clone, Copy, Debug, AsBytes, FromBytes)]
pub struct RectData {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub z_index: f32,
}

pub struct UiResourceProvider {
    pub quad: Option<Handle<Mesh>>,
}

impl UiResourceProvider {
    pub fn new() -> Self {
        UiResourceProvider { quad: None }
    }

    pub fn update(&mut self, renderer: &mut dyn Renderer, world: &World) {
        let node_query = <Read<Node>>::query().filter(!component::<Parent>());

        let mut data = Vec::new();
        if node_query.iter(world).count() > 0 {
            // TODO: this probably isn't the best way to handle z-ordering
            let mut z = 0.9999;
            {
                let mut add_data: Box<dyn FnMut(&World, Entity, ()) -> Option<()>> =
                    Box::new(|world, entity, _| {
                        let node = world.get_component::<Node>(entity).unwrap();
                        data.push(RectData {
                            position: node.global_position.into(),
                            size: node.size.into(),
                            color: node.color.into(),
                            z_index: z,
                        });

                        z -= 0.0001;
                        Some(())
                    });

                for entity in node_query
                    .iter_entities(world)
                    .map(|(entity, _)| entity)
                    .collect::<Vec<Entity>>()
                {
                    ecs::run_on_hierarchy(world, entity, (), &mut add_data);
                }
            }
        }

        if data.len() == 0 {
            return;
        }

        let size = std::mem::size_of::<RectData>();

        let mesh_id = self.quad.as_ref().unwrap().id;

        renderer.create_instance_buffer_with_data(
            resource_name::buffer::UI_INSTANCES,
            mesh_id,
            data.as_bytes(),
            size,
            data.len(),
            wgpu::BufferUsage::COPY_SRC | wgpu::BufferUsage::VERTEX,
        );
    }
}

impl ResourceProvider for UiResourceProvider {
    fn initialize(&mut self, _renderer: &mut dyn Renderer, world: &mut World) {
        // self.update(renderer, world);
        let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh>>().unwrap();
        let quad = Mesh::load(MeshType::Quad {
            north_west: math::vec2(-0.5, 0.5),
            north_east: math::vec2(0.5, 0.5),
            south_west: math::vec2(-0.5, -0.5),
            south_east: math::vec2(0.5, -0.5),
        });
        self.quad = Some(mesh_storage.add(quad));
    }

    fn update(&mut self, renderer: &mut dyn Renderer, world: &mut World) {
        self.update(renderer, world);
    }

    fn resize(
        &mut self,
        _renderer: &mut dyn Renderer,
        _world: &mut World,
        _width: u32,
        _height: u32,
    ) {
    }
}
