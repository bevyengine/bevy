use crate::{Resources, World};
use std::borrow::Cow;

#[derive(Copy, Clone)]
pub enum ThreadLocalExecution {
    Immediate,
    NextFlush,
} 

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SystemId(pub u32);

impl SystemId {
    pub fn new() -> Self {
        SystemId(rand::random::<u32>())
    }
}

pub trait System: Send + Sync {
    fn name(&self) -> Cow<'static, str>;
    fn id(&self) -> SystemId;
    fn thread_local_execution(&self) -> ThreadLocalExecution;
    fn run(&mut self, world: &World, resources: &Resources);
    fn run_thread_local(&mut self, world: &mut World, resources: &mut Resources);
    fn initialize(&mut self, _resources: &mut Resources) {}
}
