use crate::render::render_graph_2::{BindGroup, UniformPropertyType, Binding, BindType};
use spirv_reflect::{
    types::{ReflectDescriptorSet, ReflectTypeDescription, ReflectDescriptorBinding, ReflectDescriptorType},
    ShaderModule,
};
use zerocopy::AsBytes;
// use rspirv::{binary::Parser, dr::Loader, lift::LiftContext};

// TODO: pick rspirv vs spirv-reflect
// pub fn get_shader_layout(spirv_data: &[u32]) {
//     let mut loader = Loader::new();  // You can use your own consumer here.
//     {
//         let p = Parser::new(spirv_data.as_bytes(), &mut loader);
//         p.parse().unwrap();
//     }
//     let module = loader.module();
//     let structured = LiftContext::convert(&module).unwrap();
//     println!("{:?}", structured.types);
// }

pub fn get_shader_layout(spirv_data: &[u32]) {
    match ShaderModule::load_u8_data(spirv_data.as_bytes()) {
        Ok(ref mut module) => {
            let entry_point_name = module.get_entry_point_name();
            let shader_stage = module.get_shader_stage();
            println!("entry point: {}", entry_point_name);
            println!("shader stage: {:?}", shader_stage);

            let mut bind_groups = Vec::new();
            for descriptor_set in module.enumerate_descriptor_sets(None).unwrap() {
                let bind_group = reflect_bind_group(&descriptor_set);
                bind_groups.push(bind_group);
            }

            println!("  result {:?}", &bind_groups);

            println!();
        }
        _ => {}
    }
}

fn reflect_bind_group(descriptor_set: &ReflectDescriptorSet) -> BindGroup {
    println!("  set {}", descriptor_set.set);
    let mut bindings = Vec::new();
    for descriptor_binding in descriptor_set.bindings.iter() {
        let binding = reflect_binding(descriptor_binding);
        bindings.push(binding);
    }

    BindGroup::new(bindings)
}

fn reflect_binding(binding: &ReflectDescriptorBinding) -> Binding {
    let type_description = binding.type_description.as_ref().unwrap();
    let bind_type = match binding.descriptor_type {
        ReflectDescriptorType::UniformBuffer => reflect_uniform(type_description),
        _ => panic!("unsupported bind type {:?}", binding.descriptor_type),
    };

        // println!("  {:?}", binding);
    Binding{
        bind_type: bind_type,
        name: type_description.type_name.to_string()
    }
}

fn reflect_uniform(binding: &ReflectTypeDescription) -> BindType {
    BindType::Uniform {
        dynamic: false,
        properties: Vec::new()
    }
}
