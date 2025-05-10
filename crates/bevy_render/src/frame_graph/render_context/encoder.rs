use super::{PopDebugGroupParameter, PushDebugGroupParameter};

pub trait EncoderCommandBuilder {
    fn add_encoder_command(&mut self, value: EncoderCommand);

    fn push_debug_group(&mut self, label: &str) {
        self.add_encoder_command(EncoderCommand::new(PushDebugGroupParameter {
            label: label.to_string(),
        }));
    }

    fn pop_debug_group(&mut self) {
        self.add_encoder_command(EncoderCommand::new(PopDebugGroupParameter));
    }
}

pub struct EncoderCommand(Box<dyn ErasedEncoderCommand>);

impl EncoderCommand {
    pub fn new<T: ErasedEncoderCommand>(value: T) -> Self {
        Self(Box::new(value))
    }

    pub fn draw(&self, command_encoder: &mut wgpu::CommandEncoder) {
        self.0.draw(command_encoder)
    }
}

pub trait ErasedEncoderCommand: Sync + Send + 'static {
    fn draw(&self, command_encoder: &mut wgpu::CommandEncoder);
}
