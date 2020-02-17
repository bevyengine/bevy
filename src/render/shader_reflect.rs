use crate::render::render_graph_2::{
    BindGroup, BindType, Binding, UniformProperty, UniformPropertyType,
};
use spirv_reflect::{
    types::{
        ReflectDescriptorBinding, ReflectDescriptorSet, ReflectDescriptorType,
        ReflectTypeDescription, ReflectTypeFlags,
    },
    ShaderModule,
};
use zerocopy::AsBytes;
// use rspirv::{binary::Parser, dr::Loader, lift::LiftContext};

// TODO: use rspirv when structured representation is ready. this way we can remove spirv_reflect, which is a non-rust dependency
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

#[derive(Debug, Clone)]
pub struct ShaderLayout {
    pub bind_groups: Vec<BindGroup>,
    pub entry_point: String,
}

pub fn get_shader_layout(spirv_data: &[u32]) -> ShaderLayout {
    match ShaderModule::load_u8_data(spirv_data.as_bytes()) {
        Ok(ref mut module) => {
            let entry_point_name = module.get_entry_point_name();
            let mut bind_groups = Vec::new();
            for descriptor_set in module.enumerate_descriptor_sets(None).unwrap() {
                let bind_group = reflect_bind_group(&descriptor_set);
                bind_groups.push(bind_group);
            }

            ShaderLayout {
                bind_groups,
                entry_point: entry_point_name,
            }
        }
        Err(err) => panic!("Failed to reflect shader layout: {:?}", err)
    }
}

fn reflect_bind_group(descriptor_set: &ReflectDescriptorSet) -> BindGroup {
    let mut bindings = Vec::new();
    for descriptor_binding in descriptor_set.bindings.iter() {
        let binding = reflect_binding(descriptor_binding);
        bindings.push(binding);
    }

    BindGroup::new(descriptor_set.set, bindings)
}

fn reflect_binding(binding: &ReflectDescriptorBinding) -> Binding {
    let type_description = binding.type_description.as_ref().unwrap();
    let bind_type = match binding.descriptor_type {
        ReflectDescriptorType::UniformBuffer => BindType::Uniform {
            dynamic: false,
            properties: vec![reflect_uniform(type_description)],
        },
        _ => panic!("unsupported bind type {:?}", binding.descriptor_type),
    };

    Binding {
        index: binding.binding,
        bind_type,
        name: type_description.type_name.to_string(),
    }
}

#[derive(Debug)]
enum NumberType {
    Int,
    UInt,
    Float,
}

fn reflect_uniform(type_description: &ReflectTypeDescription) -> UniformProperty {
    let uniform_property_type = if type_description
        .type_flags
        .contains(ReflectTypeFlags::STRUCT)
    {
        reflect_uniform_struct(type_description)
    } else {
        reflect_uniform_numeric(type_description)
    };

    UniformProperty {
        name: type_description.type_name.to_string(),
        property_type: uniform_property_type,
    }
}

fn reflect_uniform_struct(type_description: &ReflectTypeDescription) -> UniformPropertyType {
    let mut properties = Vec::new();
    for member in type_description.members.iter() {
        properties.push(reflect_uniform(member));
    }

    UniformPropertyType::Struct(properties)
}

fn reflect_uniform_numeric(type_description: &ReflectTypeDescription) -> UniformPropertyType {
    let traits = &type_description.traits;
    let number_type = if type_description.type_flags.contains(ReflectTypeFlags::INT) {
        match traits.numeric.scalar.signedness {
            0 => NumberType::UInt,
            1 => NumberType::Int,
            signedness => panic!("unexpected signedness {}", signedness),
        }
    } else if type_description
        .type_flags
        .contains(ReflectTypeFlags::FLOAT)
    {
        NumberType::Float
    } else {
        panic!("unexpected type flag {:?}", type_description.type_flags);
    };

    // TODO: handle scalar width here

    if type_description
        .type_flags
        .contains(ReflectTypeFlags::MATRIX)
    {
        match (number_type, traits.numeric.matrix.column_count, traits.numeric.matrix.row_count) {
            (NumberType::Float, 3, 3) => UniformPropertyType::Mat3,
            (NumberType::Float, 4, 4) => UniformPropertyType::Mat4,
            (number_type, column_count, row_count) => panic!(
                "unexpected uniform property matrix format {:?} {}x{}",
                number_type, column_count, row_count
            ),
        }
    } else {
        match (number_type, traits.numeric.vector.component_count) {
            (NumberType::Int, 1) => UniformPropertyType::Int,
            (NumberType::Float, 3) => UniformPropertyType::Vec3,
            (NumberType::Float, 4) => UniformPropertyType::Vec4,
            (NumberType::UInt, 4) => UniformPropertyType::UVec4,
            (number_type, component_count) => panic!(
                "unexpected uniform property format {:?} {}",
                number_type, component_count
            ),
        }
    }
}
