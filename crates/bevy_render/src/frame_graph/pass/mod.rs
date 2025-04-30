pub mod render_pass;
pub mod render_pass_builder;

pub use render_pass::*;

use super::{FrameGraphError, RenderContext};

pub struct Pass(Box<dyn PassTrait>);

impl Default for Pass {
    fn default() -> Self {
        Pass(Box::new(EmptyPass))
    }
}

impl Pass {
    pub fn new<T: PassTrait>(pass: T) -> Pass {
        Pass(Box::new(pass))
    }

    pub fn execute(&self, render_context: &mut RenderContext) -> Result<(), FrameGraphError> {
        self.0.execute(render_context)
    }
}

pub trait PassTrait: 'static + Send + Sync {
    fn execute(&self, render_context: &mut RenderContext) -> Result<(), FrameGraphError>;
}

pub struct EmptyPass;

impl PassTrait for EmptyPass {
    fn execute(&self, _render_context: &mut RenderContext) -> Result<(), FrameGraphError> {
        Ok(())
    }
}
