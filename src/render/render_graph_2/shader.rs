use crate::prelude::*;
use crate::{asset::Texture, legion::{prelude::{Entity, World}, borrow::{Ref, RefMap}}, render::Albedo};


pub enum ShaderValue<'a> {
    Int(u32),
    Float(f32),
    Vec4(Vec4),
    Texture(&'a Handle<Texture>),
}

pub type ShaderMaterialSelector = fn(Entity, &World) -> Option<RefMap<&dyn ShaderMaterial>>;
pub struct ShaderMaterials {
    // used for distinguishing 
    pub materials: Vec<ShaderMaterialSelector>
}

impl<'a> ShaderMaterials {
  pub fn new() -> Self {
    ShaderMaterials {
      materials: Vec::new(),
    }
  }

  pub fn add(&mut self, selector: ShaderMaterialSelector) {
    self.materials.push(selector);
  }
}

pub trait ShaderMaterial {
  fn iter_properties(&self) -> std::slice::Iter<&'static str> ;
  fn get_property(&self, name: &str) -> Option<ShaderValue>;
  fn get_selector(&self) -> ShaderMaterialSelector;
}

pub struct StandardMaterial {
  pub albedo: Albedo
}

// create this from a derive macro
const STANDARD_MATERIAL_PROPERTIES: &[&str] = &["albedo"];
impl ShaderMaterial for StandardMaterial {
    fn iter_properties(&self) -> std::slice::Iter<&'static str>  {
      STANDARD_MATERIAL_PROPERTIES.iter()
    }
    fn get_property(&self, name: &str) -> Option<ShaderValue> {
      match name {
        "albedo" => Some(match self.albedo {
          Albedo::Color(color) => ShaderValue::Vec4(color),
          Albedo::Texture(ref texture) => ShaderValue::Texture(texture)
        }),
        _ => None,
      }
    }
    fn get_selector(&self) -> ShaderMaterialSelector {
      |entity, world| { 
        world.get_component::<Self>(entity).map(
          |c: Ref<StandardMaterial>| {
            c.map_into(|s| {
              s as &dyn ShaderMaterial
            })
          }
        )
      }
    }
}
