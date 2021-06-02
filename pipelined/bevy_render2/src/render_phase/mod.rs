mod draw;
mod draw_state;

pub use draw::*;
pub use draw_state::*;

use std::marker::PhantomData;
use bevy_ecs::prelude::Query;

// TODO: make this configurable per phase?
pub struct Drawable {
    pub draw_function: DrawFunctionId,
    pub draw_key: usize,
    pub sort_key: usize,
}

pub struct RenderPhase<T> {
    pub drawn_things: Vec<Drawable>,
    marker: PhantomData<fn() -> T>,
}

impl<T> Default for RenderPhase<T> {
    fn default() -> Self {
        Self {
            drawn_things: Vec::new(),
            marker: PhantomData,
        }
    }
}

impl<T> RenderPhase<T> {
    #[inline]
    pub fn add(&mut self, drawable: Drawable) {
        self.drawn_things.push(drawable);
    }

    pub fn sort(&mut self) {
        self.drawn_things.sort_by_key(|d| d.sort_key);
    }
}


pub fn sort_phase_system<T: 'static>(mut render_phases: Query<&mut RenderPhase<T>>) {
   for mut phase in render_phases.iter_mut() {
       phase.sort();
   } 
}