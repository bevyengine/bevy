use bevy::{prelude::*, asset, render::{Albedo, render_graph_2::{StandardMaterial, ShaderMaterials, ShaderMaterial, ShaderValue}}};

fn main() {
    AppBuilder::new().add_defaults().setup_world(setup).run();
}

fn setup(world: &mut World) {
    let texture_handle = {
        let mut texture_storage = world.resources.get_mut::<AssetStorage<Texture>>().unwrap();
        let texture = Texture::load(TextureType::Data(asset::create_texels(256)));
        (texture_storage.add(texture))
    };

    let mut color_shader_materials = ShaderMaterials::new();
    let color_material = StandardMaterial {
        albedo: Albedo::Color(math::vec4(1.0, 0.0, 0.0, 0.0))
    };

    color_shader_materials.add(color_material.get_selector());

    world.insert(
        (),
        vec![(
            color_shader_materials,
            color_material,
        )],
    );

    let mut texture_shader_materials = ShaderMaterials::new();
    let texture_material = StandardMaterial {
        albedo: Albedo::Texture(texture_handle)
    };

    texture_shader_materials.add(texture_material.get_selector());

    world.insert(
        (),
        vec![(
            texture_shader_materials,
            texture_material,
        )],
    );

    for (entity, materials) in <Read<ShaderMaterials>>::query().iter_entities(world) {
        println!("entity {}", entity);
        for selector in materials.materials.iter() {
            let shader_material = selector(entity, world).unwrap();
            print!("  ");
            for property in shader_material.iter_properties() {
                println!("property: {}", property);
                print!("    ");
                match shader_material.get_property(property) {
                    Some(a) => match a {
                        ShaderValue::Vec4(color) => println!("color {}", color),
                        ShaderValue::Texture(ref handle) => println!("tex {}", handle.id),
                        _ => println!("other"),
                    },
                    None => println!("none"),
                }
            } 
        }
    }
}
