use crate::{
    light::{Light, LightRaw},
    render_graph::uniform,
};
use bevy_core::Byteable;
use bevy_ecs::{Commands, IntoQuerySystem, Local, Query, Res, ResMut, Resources, System, World};
use bevy_render::{
    render_graph::{CommandQueue, Node, ResourceSlots, SystemNode},
    renderer::{
        BufferId, BufferInfo, BufferUsage, RenderContext, RenderResourceBinding,
        RenderResourceBindings, RenderResourceContext,
    },
};
use bevy_transform::prelude::*;
use std::mem;

/// A Render Graph [Node] that write light data from the ECS to GPU buffers
#[derive(Default)]
pub struct LightsNode {
    command_queue: CommandQueue,
    max_lights: usize,
}

impl LightsNode {
    pub fn new(max_lights: usize) -> Self {
        LightsNode {
            max_lights,
            command_queue: CommandQueue::default(),
        }
    }
}

impl Node for LightsNode {
    fn update(
        &mut self,
        _world: &World,
        _resources: &Resources,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        self.command_queue.execute(render_context);
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct LightCount {
    count: u32,
    _padding: [u32; 3],
}

impl LightCount {
    fn new(count: u32) -> Self {
        Self {
            count,
            _padding: [0; 3],
        }
    }
}

unsafe impl Byteable for LightCount {}

#[repr(C)]
struct LightNodeRaw<L: ?Sized = [LightRaw]> {
    count: LightCount,
    lights: L,
}

impl LightNodeRaw {
    fn create(max_lights: usize) -> Box<Self> {
        use std::alloc::{alloc_zeroed, Layout};

        // SAFETY: This should work, I asked on the rust discord and they didn't
        // immediately kill me.
        // Could depend on https://docs.rs/slice-dst/ instead.
        unsafe {
            #[repr(C)]
            struct Repr {
                ptr: *const u8,
                size: usize,
            }

            let layout = Layout::from_size_align(
                std::mem::size_of::<LightCount>() + std::mem::size_of::<LightRaw>() * max_lights,
                mem::align_of::<LightNodeRaw<[LightRaw; 0]>>(),
            )
            .unwrap();

            let ptr = alloc_zeroed(layout);
            assert!(!ptr.is_null());

            Box::from_raw(mem::transmute(Repr {
                ptr,
                size: max_lights,
            }))
        }
    }

    fn as_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                &self.count as *const LightCount as *const u8,
                std::mem::size_of::<LightCount>()
                    + std::mem::size_of::<LightRaw>() * self.lights.len(),
            )
        }
    }
}

#[test]
fn test_create_light_node_raw() {
    let raw = LightNodeRaw::create(2);

    drop(raw);
}

impl Default for Box<LightNodeRaw> {
    fn default() -> Self {
        LightNodeRaw::create(0)
    }
}

impl SystemNode for LightsNode {
    fn get_system(&self, commands: &mut Commands) -> Box<dyn System> {
        let system = lights_node_system.system();
        commands.insert_local_resource(
            system.id(),
            LightsNodeSystemState {
                max_lights: self.max_lights,
                light_buffer: None,
                raw: LightNodeRaw::create(self.max_lights),
            },
        );
        system
    }
}

/// Local "lights node system" state
#[derive(Default)]
pub struct LightsNodeSystemState {
    light_buffer: Option<BufferId>,
    max_lights: usize,
    raw: Box<LightNodeRaw>,
}

pub fn lights_node_system(
    mut state: Local<LightsNodeSystemState>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    // TODO: this write on RenderResourceBindings will prevent this system from running in parallel with other systems that do the same
    //
    // If the `light_buffer` could be created when creating this system, this
    // wouldn't need to be here.
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    // PERF: Use `Either` + `Changed` (+ `Removed`?) queries.
    mut query: Query<(&Light, &Transform, &Translation)>,
) {
    let max_lights = state.max_lights;

    let light_buffer = *state.light_buffer.get_or_insert_with(|| {
        let size = std::mem::size_of::<LightCount>() + std::mem::size_of::<LightRaw>() * max_lights;
        let buffer = render_resource_context.create_buffer(BufferInfo {
            size,
            buffer_usage: BufferUsage::UNIFORM | BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });
        render_resource_bindings.set(
            uniform::LIGHTS,
            RenderResourceBinding::Buffer {
                buffer,
                range: 0..size as u64,
                dynamic_index: None,
            },
        );
        buffer
    });

    let mut count = 0;
    for ((light, transform, translation), light_raw) in
        query.iter().iter().zip(state.raw.lights.iter_mut())
    {
        count += 1;
        *light_raw = LightRaw::from(&light, &transform.value, &translation);
    }

    state.raw.count = LightCount::new(count);

    render_resource_context.write_buffer(light_buffer, 0, state.raw.as_bytes());
}
