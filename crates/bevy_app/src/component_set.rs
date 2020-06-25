use legion::prelude::*;

// builder macro that makes defaults easy? Object3dBuilder { Option<Material> } impl Builder for Object3dBuilder { }
pub trait ComponentSet {
    fn insert(self, world: &mut World) -> Entity;
    fn insert_command_buffer(self, command_buffer: &mut CommandBuffer) -> Entity;

    // this would make composing entities from multiple sets possible
    // add_components appears to be missing from World. it will be less efficient without that
    // fn add_components(self, world: &mut World);

    // generate by macro. maybe a separate macro?
    // fn query() -> Query
}
