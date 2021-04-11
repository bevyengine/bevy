use crate::main_pass::TrackedRenderPass;
use bevy_ecs::world::World;
use parking_lot::Mutex;

// TODO: should this be generic on "drawn thing"? would provide more flexibility and  explicitness
// instead of hard coded draw key and sort key
pub trait Draw: Send + Sync + 'static {
    fn draw(
        &mut self,
        world: &World,
        pass: &mut TrackedRenderPass,
        draw_key: usize,
        sort_key: usize,
    );
}

#[derive(Default)]
pub struct DrawFunctions {
    pub draw_function: Mutex<Vec<Box<dyn Draw>>>,
}

impl DrawFunctions {
    pub fn add<D: Draw>(&self, draw_function: D) {
        self.draw_function.lock().push(Box::new(draw_function));
    }
}
