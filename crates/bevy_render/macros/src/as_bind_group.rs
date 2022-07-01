use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{
    parse::ParseStream, parse_macro_input, token::Comma, Data, DataStruct, DeriveInput, Field,
    Fields, LitInt,
};

const BINDING_ATTRIBUTE_NAME: &str = "binding";
const UNIFORM_ATTRIBUTE_NAME: &str = "uniform";
const TEXTURE_ATTRIBUTE_NAME: &str = "texture";
const SAMPLER_ATTRIBUTE_NAME: &str = "sampler";
const BIND_GROUP_DATA_ATTRIBUTE_NAME: &str = "bind_group_data";

#[derive(Copy, Clone, Debug)]
enum BindingType {
    Uniform,
    Texture,
    Sampler,
}

#[derive(Clone)]
enum BindingState<'a> {
    Free,
    Occupied {
        binding_type: BindingType,
        ident: &'a Ident,
    },
    OccupiedConvertedUniform,
    OccupiedMergableUniform {
        uniform_fields: Vec<&'a Field>,
    },
}

pub fn derive_as_bind_group(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let manifest = BevyManifest::default();
    let render_path = manifest.get_path("bevy_render");
    let asset_path = manifest.get_path("bevy_asset");

    let mut binding_states: Vec<BindingState> = Vec::new();
    let mut binding_impls = Vec::new();
    let mut bind_group_entries = Vec::new();
    let mut binding_layouts = Vec::new();
    let mut attr_prepared_data_ident = None;

    // Read struct-level attributes
    for attr in &ast.attrs {
        if let Some(attr_ident) = attr.path.get_ident() {
            if attr_ident == BIND_GROUP_DATA_ATTRIBUTE_NAME {
                if let Ok(prepared_data_ident) =
                    attr.parse_args_with(|input: ParseStream| input.parse::<Ident>())
                {
                    attr_prepared_data_ident = Some(prepared_data_ident);
                }
            } else if attr_ident == UNIFORM_ATTRIBUTE_NAME {
                let (binding_index, converted_shader_type) = attr
                    .parse_args_with(|input: ParseStream| {
                        let binding_index = input
                            .parse::<LitInt>()
                            .and_then(|i| i.base10_parse::<u32>())?;
                        input.parse::<Comma>()?;
                        let converted_shader_type = input.parse::<Ident>()?;
                        Ok((binding_index, converted_shader_type))
                    })
                    .unwrap_or_else(|_| {
                        panic!("struct-level uniform bindings must be in the format: uniform(BINDING_INDEX, ConvertedShaderType)");
                    });

                binding_impls.push(quote! {{
                    use #render_path::render_resource::AsBindGroupShaderType;
                    let mut buffer = #render_path::render_resource::encase::UniformBuffer::new(Vec::new());
                    let converted: #converted_shader_type = self.as_bind_group_shader_type(images);
                    buffer.write(&converted).unwrap();
                    #render_path::render_resource::OwnedBindingResource::Buffer(render_device.create_buffer_with_data(
                        &#render_path::render_resource::BufferInitDescriptor {
                            label: None,
                            usage: #render_path::render_resource::BufferUsages::COPY_DST | #render_path::render_resource::BufferUsages::UNIFORM,
                            contents: buffer.as_ref(),
                        },
                    ))
                }});

                binding_layouts.push(quote!{
                    #render_path::render_resource::BindGroupLayoutEntry {
                        binding: #binding_index,
                        visibility: #render_path::render_resource::ShaderStages::all(),
                        ty: #render_path::render_resource::BindingType::Buffer {
                            ty: #render_path::render_resource::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(<#converted_shader_type as #render_path::render_resource::ShaderType>::min_size()),
                        },
                        count: None,
                    }
                });

                let binding_vec_index = bind_group_entries.len();
                bind_group_entries.push(quote! {
                    #render_path::render_resource::BindGroupEntry {
                        binding: #binding_index,
                        resource: bindings[#binding_vec_index].get_binding(),
                    }
                });

                let required_len = binding_index as usize + 1;
                if required_len > binding_states.len() {
                    binding_states.resize(required_len, BindingState::Free);
                }
                binding_states[binding_index as usize] = BindingState::OccupiedConvertedUniform;
            }
        }
    }

    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("Expected a struct with named fields"),
    };

    // Read field-level attributes
    for field in fields.iter() {
        for attr in &field.attrs {
            let attr_ident = if let Some(ident) = attr.path.get_ident() {
                ident
            } else {
                continue;
            };

            let binding_type = if attr_ident == UNIFORM_ATTRIBUTE_NAME {
                BindingType::Uniform
            } else if attr_ident == TEXTURE_ATTRIBUTE_NAME {
                BindingType::Texture
            } else if attr_ident == SAMPLER_ATTRIBUTE_NAME {
                BindingType::Sampler
            } else {
                continue;
            };

            let binding_index = attr
                .parse_args_with(|input: ParseStream| {
                    let binding_index = input
                        .parse::<LitInt>()
                        .and_then(|i| i.base10_parse::<u32>())
                        .expect("binding index was not a valid u32");
                    Ok(binding_index)
                })
                .unwrap_or_else(|_| {
                    panic!("Invalid `{}` attribute format", BINDING_ATTRIBUTE_NAME)
                });

            let field_name = field.ident.as_ref().unwrap();
            let required_len = binding_index as usize + 1;
            if required_len > binding_states.len() {
                binding_states.resize(required_len, BindingState::Free);
            }

            match &mut binding_states[binding_index as usize] {
                value @ BindingState::Free => {
                    *value = match binding_type {
                        BindingType::Uniform => BindingState::OccupiedMergableUniform {
                            uniform_fields: vec![field],
                        },
                        _ => {
                            // only populate bind group entries for non-uniforms
                            // uniform entries are deferred until the end
                            let binding_vec_index = bind_group_entries.len();
                            bind_group_entries.push(quote! {
                                #render_path::render_resource::BindGroupEntry {
                                    binding: #binding_index,
                                    resource: bindings[#binding_vec_index].get_binding(),
                                }
                            });
                            BindingState::Occupied {
                            binding_type,
                            ident: field_name,
                        }},
                    }
                },
                BindingState::Occupied { binding_type, ident: occupied_ident} => panic!(
                    "The '{field_name}' field cannot be assigned to binding {binding_index} because it is already occupied by the field '{occupied_ident}' of type {binding_type:?}."
                ),
                BindingState::OccupiedConvertedUniform => panic!(
                    "The '{field_name}' field cannot be assigned to binding {binding_index} because it is already occupied by a struct-level uniform binding at the same index."
                ),
                BindingState::OccupiedMergableUniform { uniform_fields } => {
                    match binding_type {
                        BindingType::Uniform => {
                            uniform_fields.push(field);
                        },
                        _ => {panic!("The '{field_name}' field cannot be assigned to binding {binding_index} because it is already occupied by a {:?}.", BindingType::Uniform)},
                    }
                },
            }

            match binding_type {
                BindingType::Uniform => { /* uniform codegen is deferred to account for combined uniform bindings */
                }
                BindingType::Texture => {
                    binding_impls.push(quote! {
                        #render_path::render_resource::OwnedBindingResource::TextureView({
                            let handle: Option<&#asset_path::Handle<#render_path::texture::Image>> = (&self.#field_name).into();
                            if let Some(handle) = handle {
                                images.get(handle).ok_or_else(|| #render_path::render_resource::AsBindGroupError::RetryNextUpdate)?.texture_view.clone()
                            } else {
                                fallback_image.texture_view.clone()
                            }
                        })
                    });

                    binding_layouts.push(quote!{
                        #render_path::render_resource::BindGroupLayoutEntry {
                            binding: #binding_index,
                            visibility: #render_path::render_resource::ShaderStages::all(),
                            ty: #render_path::render_resource::BindingType::Texture {
                                multisampled: false,
                                sample_type: #render_path::render_resource::TextureSampleType::Float { filterable: true },
                                view_dimension: #render_path::render_resource::TextureViewDimension::D2,
                            },
                            count: None,
                        }
                    });
                }
                BindingType::Sampler => {
                    binding_impls.push(quote! {
                        #render_path::render_resource::OwnedBindingResource::Sampler({
                            let handle: Option<&#asset_path::Handle<#render_path::texture::Image>> = (&self.#field_name).into();
                            if let Some(handle) = handle {
                                images.get(handle).ok_or_else(|| #render_path::render_resource::AsBindGroupError::RetryNextUpdate)?.sampler.clone()
                            } else {
                                fallback_image.sampler.clone()
                            }
                        })
                    });

                    binding_layouts.push(quote!{
                        #render_path::render_resource::BindGroupLayoutEntry {
                            binding: #binding_index,
                            visibility: #render_path::render_resource::ShaderStages::all(),
                            ty: #render_path::render_resource::BindingType::Sampler(#render_path::render_resource::SamplerBindingType::Filtering),
                            count: None,
                        }
                    });
                }
            }
        }
    }

    // Produce impls for fields with uniform bindings
    let struct_name = &ast.ident;
    let mut field_struct_impls = Vec::new();
    for (binding_index, binding_state) in binding_states.iter().enumerate() {
        let binding_index = binding_index as u32;
        if let BindingState::OccupiedMergableUniform { uniform_fields } = binding_state {
            let binding_vec_index = bind_group_entries.len();
            bind_group_entries.push(quote! {
                #render_path::render_resource::BindGroupEntry {
                    binding: #binding_index,
                    resource: bindings[#binding_vec_index].get_binding(),
                }
            });
            // single field uniform bindings for a given index can use a straightforward binding
            if uniform_fields.len() == 1 {
                let field = &uniform_fields[0];
                let field_name = field.ident.as_ref().unwrap();
                let field_ty = &field.ty;
                binding_impls.push(quote! {{
                    let mut buffer = #render_path::render_resource::encase::UniformBuffer::new(Vec::new());
                    buffer.write(&self.#field_name).unwrap();
                    #render_path::render_resource::OwnedBindingResource::Buffer(render_device.create_buffer_with_data(
                        &#render_path::render_resource::BufferInitDescriptor {
                            label: None,
                            usage: #render_path::render_resource::BufferUsages::COPY_DST | #render_path::render_resource::BufferUsages::UNIFORM,
                            contents: buffer.as_ref(),
                        },
                    ))
                }});

                binding_layouts.push(quote!{
                    #render_path::render_resource::BindGroupLayoutEntry {
                        binding: #binding_index,
                        visibility: #render_path::render_resource::ShaderStages::all(),
                        ty: #render_path::render_resource::BindingType::Buffer {
                            ty: #render_path::render_resource::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(<#field_ty as #render_path::render_resource::ShaderType>::min_size()),
                        },
                        count: None,
                    }
                });
            // multi-field uniform bindings for a given index require an intermediate struct to derive ShaderType
            } else {
                let uniform_struct_name = Ident::new(
                    &format!("_{struct_name}AsBindGroupUniformStructBindGroup{binding_index}"),
                    Span::call_site(),
                );
                let field_name = uniform_fields.iter().map(|f| f.ident.as_ref().unwrap());
                let field_type = uniform_fields.iter().map(|f| &f.ty);
                field_struct_impls.push(quote! {
                    #[derive(#render_path::render_resource::ShaderType)]
                    struct #uniform_struct_name<'a> {
                        #(#field_name: &'a #field_type,)*
                    }
                });

                let field_name = uniform_fields.iter().map(|f| f.ident.as_ref().unwrap());
                binding_impls.push(quote! {{
                    let mut buffer = #render_path::render_resource::encase::UniformBuffer::new(Vec::new());
                    buffer.write(&#uniform_struct_name {
                        #(#field_name: &self.#field_name,)*
                    }).unwrap();
                    #render_path::render_resource::OwnedBindingResource::Buffer(render_device.create_buffer_with_data(
                        &#render_path::render_resource::BufferInitDescriptor {
                            label: None,
                            usage: #render_path::render_resource::BufferUsages::COPY_DST | #render_path::render_resource::BufferUsages::UNIFORM,
                            contents: buffer.as_ref(),
                        },
                    ))
                }});

                binding_layouts.push(quote!{
                    #render_path::render_resource::BindGroupLayoutEntry {
                        binding: #binding_index,
                        visibility: #render_path::render_resource::ShaderStages::all(),
                        ty: #render_path::render_resource::BindingType::Buffer {
                            ty: #render_path::render_resource::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(<#uniform_struct_name as #render_path::render_resource::ShaderType>::min_size()),
                        },
                        count: None,
                    }
                });
            }
        }
    }

    let generics = ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let (prepared_data, get_prepared_data) = if let Some(prepared) = attr_prepared_data_ident {
        let get_prepared_data = quote! { self.into() };
        (quote! {#prepared}, get_prepared_data)
    } else {
        let prepared_data = quote! { () };
        (prepared_data.clone(), prepared_data)
    };

    TokenStream::from(quote! {
        #(#field_struct_impls)*

        impl #impl_generics #render_path::render_resource::AsBindGroup for #struct_name #ty_generics #where_clause {
            type Data = #prepared_data;
            fn as_bind_group(
                &self,
                layout: &#render_path::render_resource::BindGroupLayout,
                render_device: &#render_path::renderer::RenderDevice,
                images: &#render_path::render_asset::RenderAssets<#render_path::texture::Image>,
                fallback_image: &#render_path::texture::FallbackImage,
            ) -> Result<#render_path::render_resource::PreparedBindGroup<Self>, #render_path::render_resource::AsBindGroupError> {
                let bindings = vec![#(#binding_impls,)*];

                let bind_group = {
                    let descriptor = #render_path::render_resource::BindGroupDescriptor {
                        entries: &[#(#bind_group_entries,)*],
                        label: None,
                        layout: &layout,
                    };
                    render_device.create_bind_group(&descriptor)
                };

                Ok(#render_path::render_resource::PreparedBindGroup {
                    bindings,
                    bind_group,
                    data: #get_prepared_data,
                })
            }

            fn bind_group_layout(render_device: &#render_path::renderer::RenderDevice) -> #render_path::render_resource::BindGroupLayout {
                render_device.create_bind_group_layout(&#render_path::render_resource::BindGroupLayoutDescriptor {
                    entries: &[#(#binding_layouts,)*],
                    label: None,
                })
            }
        }
    })
}
