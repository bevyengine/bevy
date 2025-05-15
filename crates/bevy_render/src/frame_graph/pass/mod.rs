pub mod compute_pass;
pub mod encoder_pass;
pub mod pass_builder;
pub mod render_pass;

pub use compute_pass::*;
pub use encoder_pass::*;
pub use pass_builder::*;
pub use render_pass::*;

use wgpu::CommandEncoder;

use super::RenderContext;

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

    pub fn execute(&self, render_context: &mut RenderContext) {
        self.0.render(render_context);
    }
}

pub trait EncoderExecutor: 'static + Send + Sync {
    fn execute(&self, command_encoder: &mut CommandEncoder, render_context: &mut RenderContext);
}

impl<T: EncoderExecutor> PassTrait for T {
    fn render(&self, render_context: &mut RenderContext) {
        render_context.flush_encoder();

        let mut command_encoder = render_context.create_command_encoder();

        self.execute(&mut command_encoder, render_context);

        let command_buffer = command_encoder.finish();

        render_context.add_command_buffer(command_buffer);
    }
}

pub trait PassTrait: 'static + Send + Sync {
    fn render(&self, render_context: &mut RenderContext);
}

pub struct EmptyPass;

impl PassTrait for EmptyPass {
    fn render(&self, render_context: &mut RenderContext) {
        render_context.flush_encoder();
    }
}
