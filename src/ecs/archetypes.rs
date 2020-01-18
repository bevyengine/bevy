use crate::prelude::*;
use legion::prelude::*;


// builder macro that makes defaults easy? Object3dBuilder { Option<Material> } impl Builder for Object3dBuilder { }
pub trait EntityArchetype {
    fn insert(self, world: &mut World) -> Entity;
    // add_components appears to be missing from World. it will be less efficient without that
    // fn add_components(self, world: &mut World);
}

pub struct Object3dEntity {
    pub mesh: Handle<Mesh>,
    pub material: Material,
    pub local_to_world: LocalToWorld,
    pub translation: Translation,
}

// TODO: make this a macro
impl EntityArchetype for Object3dEntity {
    fn insert(self, world: &mut World) -> Entity {
        *world.insert((), vec![(
            self.mesh,
            self.material,
            self.local_to_world,
            self.translation,
        )]).first().unwrap()
    }
}

pub struct LightEntity {
    pub light: Light,
    pub local_to_world: LocalToWorld,
    pub translation: Translation,
    pub rotation: Rotation,
}

// TODO: make this a macro
impl EntityArchetype for LightEntity {
    fn insert(self, world: &mut World) -> Entity {
        *world.insert((), vec![(
            self.light,
            self.local_to_world,
            self.translation,
            self.rotation,
        )]).first().unwrap()
    }
}

pub struct CameraEntity {
    pub camera: Camera,
    pub active_camera: ActiveCamera,
    pub local_to_world: LocalToWorld,
}

// TODO: make this a macro
impl EntityArchetype for CameraEntity {
    fn insert(self, world: &mut World) -> Entity {
        *world.insert((), vec![(
            self.camera,
            self.active_camera,
            self.local_to_world,
        )]).first().unwrap()
    }
}