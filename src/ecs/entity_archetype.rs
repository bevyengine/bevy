use legion::prelude::*;

// builder macro that makes defaults easy? Object3dBuilder { Option<Material> } impl Builder for Object3dBuilder { }
pub trait EntityArchetype {
    fn insert(self, world: &mut World) -> Entity;

    // this would make composing entities from multiple archetypes possible
    // add_components appears to be missing from World. it will be less efficient without that
    // fn add_components(self, world: &mut World);

    // generate by macro. maybe a separate macro?
    // fn query() -> Query
}