use bevy_macro_utils::{get_lit_bool, get_lit_str, BevyManifest, Symbol};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Comma,
    Data, DataStruct, Error, Fields, Lit, LitInt, LitStr, Meta, MetaList, Result,
};

const UNIFORM_ATTRIBUTE_NAME: Symbol = Symbol("uniform");
const TEXTURE_ATTRIBUTE_NAME: Symbol = Symbol("texture");
const STORAGE_TEXTURE_ATTRIBUTE_NAME: Symbol = Symbol("storage_texture");
const SAMPLER_ATTRIBUTE_NAME: Symbol = Symbol("sampler");
const STORAGE_ATTRIBUTE_NAME: Symbol = Symbol("storage");
const BIND_GROUP_DATA_ATTRIBUTE_NAME: Symbol = Symbol("bind_group_data");
const BINDLESS_ATTRIBUTE_NAME: Symbol = Symbol("bindless");

#[derive(Copy, Clone, Debug)]
enum BindingType {
    Uniform,
    Texture,
    StorageTexture,
    Sampler,
    Storage,
}

#[derive(Clone)]
enum BindingState<'a> {
    Free,
    Occupied {
        binding_type: BindingType,
        ident: &'a Ident,
    },
    OccupiedConvertedUniform,
    OccupiedMergeableUniform {
        uniform_fields: Vec<&'a syn::Field>,
    },
}

pub fn derive_as_bind_group(ast: syn::DeriveInput) -> Result<TokenStream> {
    let manifest = BevyManifest::shared();
    let render_path = manifest.get_path("bevy_render");
    let image_path = manifest.get_path("bevy_image");
    let asset_path = manifest.get_path("bevy_asset");
    let ecs_path = manifest.get_path("bevy_ecs");

    let mut binding_states: Vec<BindingState> = Vec::new();
    let mut binding_impls = Vec::new();
    let mut binding_layouts = Vec::new();
    let mut attr_prepared_data_ident = None;
    let mut attr_bindless_count = None;

    // `actual_bindless_slot_count` holds the actual number of bindless slots
    // per bind group, taking into account whether the current platform supports
    // bindless resources.
    let actual_bindless_slot_count = Ident::new("actual_bindless_slot_count", Span::call_site());

    // The `BufferBindingType` and corresponding `BufferUsages` used for
    // uniforms. We need this because bindless uniforms don't exist, so in
    // bindless mode we must promote uniforms to storage buffers.
    let uniform_binding_type = Ident::new("uniform_binding_type", Span::call_site());
    let uniform_buffer_usages = Ident::new("uniform_buffer_usages", Span::call_site());

    // Read struct-level attributes
    for attr in &ast.attrs {
        if let Some(attr_ident) = attr.path().get_ident() {
            if attr_ident == BIND_GROUP_DATA_ATTRIBUTE_NAME {
                if let Ok(prepared_data_ident) =
                    attr.parse_args_with(|input: ParseStream| input.parse::<Ident>())
                {
                    attr_prepared_data_ident = Some(prepared_data_ident);
                }
            } else if attr_ident == UNIFORM_ATTRIBUTE_NAME {
                let (binding_index, converted_shader_type) = get_uniform_binding_attr(attr)?;
                binding_impls.push(quote! {{
                    use #render_path::render_resource::AsBindGroupShaderType;
                    let mut buffer = #render_path::render_resource::encase::UniformBuffer::new(Vec::new());
                    let converted: #converted_shader_type = self.as_bind_group_shader_type(&images);
                    buffer.write(&converted).unwrap();
                    (
                        #binding_index,
                        #render_path::render_resource::OwnedBindingResource::Buffer(render_device.create_buffer_with_data(
                            &#render_path::render_resource::BufferInitDescriptor {
                                label: None,
                                usage: #uniform_buffer_usages,
                                contents: buffer.as_ref(),
                            },
                        ))
                    )
                }});

                binding_layouts.push(quote!{
                    #render_path::render_resource::BindGroupLayoutEntry {
                        binding: #binding_index,
                        visibility: #render_path::render_resource::ShaderStages::all(),
                        ty: #render_path::render_resource::BindingType::Buffer {
                            ty: #uniform_binding_type,
                            has_dynamic_offset: false,
                            min_binding_size: Some(<#converted_shader_type as #render_path::render_resource::ShaderType>::min_size()),
                        },
                        count: #actual_bindless_slot_count,
                    }
                });

                let required_len = binding_index as usize + 1;
                if required_len > binding_states.len() {
                    binding_states.resize(required_len, BindingState::Free);
                }
                binding_states[binding_index as usize] = BindingState::OccupiedConvertedUniform;
            } else if attr_ident == BINDLESS_ATTRIBUTE_NAME {
                if let Ok(count_lit) =
                    attr.parse_args_with(|input: ParseStream| input.parse::<Lit>())
                {
                    attr_bindless_count = Some(count_lit);
                }
            }
        }
    }

    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => {
            return Err(Error::new_spanned(
                ast,
                "Expected a struct with named fields",
            ));
        }
    };

    // Count the number of sampler fields needed. We might have to disable
    // bindless if bindless arrays take the GPU over the maximum number of
    // samplers.
    let mut sampler_binding_count = 0;

    // Read field-level attributes
    for field in fields {
        // Search ahead for texture attributes so we can use them with any
        // corresponding sampler attribute.
        let mut tex_attrs = None;
        for attr in &field.attrs {
            let Some(attr_ident) = attr.path().get_ident() else {
                continue;
            };
            if attr_ident == TEXTURE_ATTRIBUTE_NAME {
                let (_binding_index, nested_meta_items) = get_binding_nested_attr(attr)?;
                tex_attrs = Some(get_texture_attrs(nested_meta_items)?);
            }
        }

        for attr in &field.attrs {
            let Some(attr_ident) = attr.path().get_ident() else {
                continue;
            };

            let binding_type = if attr_ident == UNIFORM_ATTRIBUTE_NAME {
                BindingType::Uniform
            } else if attr_ident == TEXTURE_ATTRIBUTE_NAME {
                BindingType::Texture
            } else if attr_ident == STORAGE_TEXTURE_ATTRIBUTE_NAME {
                BindingType::StorageTexture
            } else if attr_ident == SAMPLER_ATTRIBUTE_NAME {
                BindingType::Sampler
            } else if attr_ident == STORAGE_ATTRIBUTE_NAME {
                BindingType::Storage
            } else {
                continue;
            };

            let (binding_index, nested_meta_items) = get_binding_nested_attr(attr)?;

            let field_name = field.ident.as_ref().unwrap();
            let required_len = binding_index as usize + 1;
            if required_len > binding_states.len() {
                binding_states.resize(required_len, BindingState::Free);
            }

            match &mut binding_states[binding_index as usize] {
                value @ BindingState::Free => {
                    *value = match binding_type {
                        BindingType::Uniform => BindingState::OccupiedMergeableUniform {
                            uniform_fields: vec![field],
                        },
                        _ => {
                            // only populate bind group entries for non-uniforms
                            // uniform entries are deferred until the end
                            BindingState::Occupied {
                                binding_type,
                                ident: field_name,
                            }
                        }
                    }
                }
                BindingState::Occupied {
                    binding_type,
                    ident: occupied_ident,
                } => {
                    return Err(Error::new_spanned(
                        attr,
                        format!("The '{field_name}' field cannot be assigned to binding {binding_index} because it is already occupied by the field '{occupied_ident}' of type {binding_type:?}.")
                    ));
                }
                BindingState::OccupiedConvertedUniform => {
                    return Err(Error::new_spanned(
                        attr,
                        format!("The '{field_name}' field cannot be assigned to binding {binding_index} because it is already occupied by a struct-level uniform binding at the same index.")
                    ));
                }
                BindingState::OccupiedMergeableUniform { uniform_fields } => match binding_type {
                    BindingType::Uniform => {
                        uniform_fields.push(field);
                    }
                    _ => {
                        return Err(Error::new_spanned(
                                attr,
                                format!("The '{field_name}' field cannot be assigned to binding {binding_index} because it is already occupied by a {:?}.", BindingType::Uniform)
                            ));
                    }
                },
            }

            match binding_type {
                BindingType::Uniform => {
                    // uniform codegen is deferred to account for combined uniform bindings
                }
                BindingType::Storage => {
                    let StorageAttrs {
                        visibility,
                        read_only,
                        buffer,
                    } = get_storage_binding_attr(nested_meta_items)?;
                    let visibility =
                        visibility.hygienic_quote(&quote! { #render_path::render_resource });

                    let field_name = field.ident.as_ref().unwrap();

                    if buffer {
                        binding_impls.push(quote! {
                            (
                                #binding_index,
                                #render_path::render_resource::OwnedBindingResource::Buffer({
                                    self.#field_name.clone()
                                })
                            )
                        });
                    } else {
                        binding_impls.push(quote! {
                        (
                            #binding_index,
                            #render_path::render_resource::OwnedBindingResource::Buffer({
                                let handle: &#asset_path::Handle<#render_path::storage::ShaderStorageBuffer> = (&self.#field_name);
                                storage_buffers.get(handle).ok_or_else(|| #render_path::render_resource::AsBindGroupError::RetryNextUpdate)?.buffer.clone()
                            })
                        )
                        });
                    }

                    binding_layouts.push(quote! {
                        #render_path::render_resource::BindGroupLayoutEntry {
                            binding: #binding_index,
                            visibility: #visibility,
                            ty: #render_path::render_resource::BindingType::Buffer {
                                ty: #render_path::render_resource::BufferBindingType::Storage { read_only: #read_only },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: #actual_bindless_slot_count,
                        }
                    });
                }
                BindingType::StorageTexture => {
                    let StorageTextureAttrs {
                        dimension,
                        image_format,
                        access,
                        visibility,
                    } = get_storage_texture_binding_attr(nested_meta_items)?;

                    let visibility =
                        visibility.hygienic_quote(&quote! { #render_path::render_resource });

                    let fallback_image = get_fallback_image(&render_path, dimension);

                    // insert fallible texture-based entries at 0 so that if we fail here, we exit before allocating any buffers
                    binding_impls.insert(0, quote! {
                        ( #binding_index,
                          #render_path::render_resource::OwnedBindingResource::TextureView(
                                #dimension,
                                {
                                    let handle: Option<&#asset_path::Handle<#image_path::Image>> = (&self.#field_name).into();
                                    if let Some(handle) = handle {
                                        images.get(handle).ok_or_else(|| #render_path::render_resource::AsBindGroupError::RetryNextUpdate)?.texture_view.clone()
                                    } else {
                                        #fallback_image.texture_view.clone()
                                    }
                                }
                            )
                        )
                    });

                    binding_layouts.push(quote! {
                        #render_path::render_resource::BindGroupLayoutEntry {
                            binding: #binding_index,
                            visibility: #visibility,
                            ty: #render_path::render_resource::BindingType::StorageTexture {
                                access: #render_path::render_resource::StorageTextureAccess::#access,
                                format: #render_path::render_resource::TextureFormat::#image_format,
                                view_dimension: #render_path::render_resource::#dimension,
                            },
                            count: #actual_bindless_slot_count,
                        }
                    });
                }
                BindingType::Texture => {
                    let TextureAttrs {
                        dimension,
                        sample_type,
                        multisampled,
                        visibility,
                    } = tex_attrs.as_ref().unwrap();

                    let visibility =
                        visibility.hygienic_quote(&quote! { #render_path::render_resource });

                    let fallback_image = get_fallback_image(&render_path, *dimension);

                    // insert fallible texture-based entries at 0 so that if we fail here, we exit before allocating any buffers
                    binding_impls.insert(0, quote! {
                        (
                            #binding_index,
                            #render_path::render_resource::OwnedBindingResource::TextureView(
                                #render_path::render_resource::#dimension,
                                {
                                    let handle: Option<&#asset_path::Handle<#image_path::Image>> = (&self.#field_name).into();
                                    if let Some(handle) = handle {
                                        images.get(handle).ok_or_else(|| #render_path::render_resource::AsBindGroupError::RetryNextUpdate)?.texture_view.clone()
                                    } else {
                                        #fallback_image.texture_view.clone()
                                    }
                                }
                            )
                        )
                    });

                    sampler_binding_count += 1;

                    binding_layouts.push(quote! {
                        #render_path::render_resource::BindGroupLayoutEntry {
                            binding: #binding_index,
                            visibility: #visibility,
                            ty: #render_path::render_resource::BindingType::Texture {
                                multisampled: #multisampled,
                                sample_type: #render_path::render_resource::#sample_type,
                                view_dimension: #render_path::render_resource::#dimension,
                            },
                            count: #actual_bindless_slot_count,
                        }
                    });
                }
                BindingType::Sampler => {
                    let SamplerAttrs {
                        sampler_binding_type,
                        visibility,
                        ..
                    } = get_sampler_attrs(nested_meta_items)?;
                    let TextureAttrs { dimension, .. } = tex_attrs
                        .as_ref()
                        .expect("sampler attribute must have matching texture attribute");

                    let visibility =
                        visibility.hygienic_quote(&quote! { #render_path::render_resource });

                    let fallback_image = get_fallback_image(&render_path, *dimension);

                    let expected_samplers = match sampler_binding_type {
                        SamplerBindingType::Filtering => {
                            quote!( [#render_path::render_resource::TextureSampleType::Float { filterable: true }] )
                        }
                        SamplerBindingType::NonFiltering => quote!([
                            #render_path::render_resource::TextureSampleType::Float { filterable: false },
                            #render_path::render_resource::TextureSampleType::Sint,
                            #render_path::render_resource::TextureSampleType::Uint,
                        ]),
                        SamplerBindingType::Comparison => {
                            quote!( [#render_path::render_resource::TextureSampleType::Depth] )
                        }
                    };

                    // insert fallible texture-based entries at 0 so that if we fail here, we exit before allocating any buffers
                    binding_impls.insert(0, quote! {
                        (
                            #binding_index,
                            #render_path::render_resource::OwnedBindingResource::Sampler({
                                let handle: Option<&#asset_path::Handle<#image_path::Image>> = (&self.#field_name).into();
                                if let Some(handle) = handle {
                                    let image = images.get(handle).ok_or_else(|| #render_path::render_resource::AsBindGroupError::RetryNextUpdate)?;

                                    let Some(sample_type) = image.texture_format.sample_type(None, Some(render_device.features())) else {
                                        return Err(#render_path::render_resource::AsBindGroupError::InvalidSamplerType(
                                            #binding_index,
                                            "None".to_string(),
                                            format!("{:?}", #expected_samplers),
                                        ));
                                    };

                                    let valid = #expected_samplers.contains(&sample_type);

                                    if !valid {
                                        return Err(#render_path::render_resource::AsBindGroupError::InvalidSamplerType(
                                            #binding_index,
                                            format!("{:?}", sample_type),
                                            format!("{:?}", #expected_samplers),
                                        ));
                                    }
                                    image.sampler.clone()
                                } else {
                                    #fallback_image.sampler.clone()
                                }
                            })
                        )
                    });

                    sampler_binding_count += 1;

                    binding_layouts.push(quote!{
                        #render_path::render_resource::BindGroupLayoutEntry {
                            binding: #binding_index,
                            visibility: #visibility,
                            ty: #render_path::render_resource::BindingType::Sampler(#render_path::render_resource::#sampler_binding_type),
                            count: #actual_bindless_slot_count,
                        }
                    });
                }
            }
        }
    }

    // Produce impls for fields with uniform bindings
    let struct_name = &ast.ident;
    let struct_name_literal = struct_name.to_string();
    let struct_name_literal = struct_name_literal.as_str();
    let mut field_struct_impls = Vec::new();

    let uniform_binding_type_declarations = match attr_bindless_count {
        Some(_) => {
            quote! {
                let (#uniform_binding_type, #uniform_buffer_usages) =
                    if Self::bindless_supported(render_device) && !force_no_bindless {
                        (
                            #render_path::render_resource::BufferBindingType::Storage { read_only: true },
                            #render_path::render_resource::BufferUsages::STORAGE,
                        )
                    } else {
                        (
                            #render_path::render_resource::BufferBindingType::Uniform,
                            #render_path::render_resource::BufferUsages::UNIFORM,
                        )
                    };
            }
        }
        None => {
            quote! {
                let (#uniform_binding_type, #uniform_buffer_usages) = (
                    #render_path::render_resource::BufferBindingType::Uniform,
                    #render_path::render_resource::BufferUsages::UNIFORM,
                );
            }
        }
    };

    for (binding_index, binding_state) in binding_states.iter().enumerate() {
        let binding_index = binding_index as u32;
        if let BindingState::OccupiedMergeableUniform { uniform_fields } = binding_state {
            // single field uniform bindings for a given index can use a straightforward binding
            if uniform_fields.len() == 1 {
                let field = &uniform_fields[0];
                let field_name = field.ident.as_ref().unwrap();
                let field_ty = &field.ty;
                binding_impls.push(quote! {{
                    let mut buffer = #render_path::render_resource::encase::UniformBuffer::new(Vec::new());
                    buffer.write(&self.#field_name).unwrap();
                    (
                        #binding_index,
                        #render_path::render_resource::OwnedBindingResource::Buffer(render_device.create_buffer_with_data(
                            &#render_path::render_resource::BufferInitDescriptor {
                                label: None,
                                usage: #uniform_buffer_usages,
                                contents: buffer.as_ref(),
                            },
                        ))
                    )
                }});

                binding_layouts.push(quote!{
                    #render_path::render_resource::BindGroupLayoutEntry {
                        binding: #binding_index,
                        visibility: #render_path::render_resource::ShaderStages::all(),
                        ty: #render_path::render_resource::BindingType::Buffer {
                            ty: #uniform_binding_type,
                            has_dynamic_offset: false,
                            min_binding_size: Some(<#field_ty as #render_path::render_resource::ShaderType>::min_size()),
                        },
                        count: actual_bindless_slot_count,
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
                    (
                        #binding_index,
                        #render_path::render_resource::OwnedBindingResource::Buffer(render_device.create_buffer_with_data(
                            &#render_path::render_resource::BufferInitDescriptor {
                                label: None,
                                usage: #uniform_buffer_usages,
                                contents: buffer.as_ref(),
                            },
                        ))
                    )
                }});

                binding_layouts.push(quote!{
                    #render_path::render_resource::BindGroupLayoutEntry {
                        binding: #binding_index,
                        visibility: #render_path::render_resource::ShaderStages::all(),
                        ty: #render_path::render_resource::BindingType::Buffer {
                            ty: #uniform_binding_type,
                            has_dynamic_offset: false,
                            min_binding_size: Some(<#uniform_struct_name as #render_path::render_resource::ShaderType>::min_size()),
                        },
                        count: actual_bindless_slot_count,
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

    // Calculate the number of samplers that we need, so that we don't go over
    // the limit on certain platforms. See
    // https://github.com/bevyengine/bevy/issues/16988.
    let samplers_needed = match attr_bindless_count {
        Some(Lit::Int(ref bindless_count)) => match bindless_count.base10_parse::<u32>() {
            Ok(bindless_count) => sampler_binding_count * bindless_count,
            Err(_) => 0,
        },
        _ => 0,
    };

    // Calculate the actual number of bindless slots, taking hardware
    // limitations into account.
    let (bindless_slot_count, actual_bindless_slot_count_declaration) = match attr_bindless_count {
        Some(bindless_count) => (
            quote! {
                fn bindless_slot_count() -> Option<u32> {
                    Some(#bindless_count)
                }

                fn bindless_supported(render_device: &#render_path::renderer::RenderDevice) -> bool {
                    render_device.features().contains(
                        #render_path::settings::WgpuFeatures::BUFFER_BINDING_ARRAY |
                        #render_path::settings::WgpuFeatures::TEXTURE_BINDING_ARRAY
                    ) &&
                    render_device.limits().max_storage_buffers_per_shader_stage > 0 &&
                        render_device.limits().max_samplers_per_shader_stage >= #samplers_needed
                }
            },
            quote! {
                let #actual_bindless_slot_count = if Self::bindless_supported(render_device) &&
                        !force_no_bindless {
                    ::core::num::NonZeroU32::new(#bindless_count)
                } else {
                    None
                };
            },
        ),
        None => (
            TokenStream::new().into(),
            quote! { let #actual_bindless_slot_count: Option<::core::num::NonZeroU32> = None; },
        ),
    };

    Ok(TokenStream::from(quote! {
        #(#field_struct_impls)*

        impl #impl_generics #render_path::render_resource::AsBindGroup for #struct_name #ty_generics #where_clause {
            type Data = #prepared_data;

            type Param = (
                #ecs_path::system::lifetimeless::SRes<#render_path::render_asset::RenderAssets<#render_path::texture::GpuImage>>,
                #ecs_path::system::lifetimeless::SRes<#render_path::texture::FallbackImage>,
                #ecs_path::system::lifetimeless::SRes<#render_path::render_asset::RenderAssets<#render_path::storage::GpuShaderStorageBuffer>>,
            );

            #bindless_slot_count

            fn label() -> Option<&'static str> {
                Some(#struct_name_literal)
            }

            fn unprepared_bind_group(
                &self,
                layout: &#render_path::render_resource::BindGroupLayout,
                render_device: &#render_path::renderer::RenderDevice,
                (images, fallback_image, storage_buffers): &mut #ecs_path::system::SystemParamItem<'_, '_, Self::Param>,
                force_no_bindless: bool,
            ) -> Result<#render_path::render_resource::UnpreparedBindGroup<Self::Data>, #render_path::render_resource::AsBindGroupError> {
                #uniform_binding_type_declarations

                let bindings = #render_path::render_resource::BindingResources(vec![#(#binding_impls,)*]);

                Ok(#render_path::render_resource::UnpreparedBindGroup {
                    bindings,
                    data: #get_prepared_data,
                })
            }

            fn bind_group_layout_entries(
                render_device: &#render_path::renderer::RenderDevice,
                force_no_bindless: bool
            ) -> Vec<#render_path::render_resource::BindGroupLayoutEntry> {
                #actual_bindless_slot_count_declaration
                #uniform_binding_type_declarations

                vec![#(#binding_layouts,)*]
            }
        }
    }))
}

fn get_fallback_image(
    render_path: &syn::Path,
    dimension: BindingTextureDimension,
) -> proc_macro2::TokenStream {
    quote! {
        match #render_path::render_resource::#dimension {
            #render_path::render_resource::TextureViewDimension::D1 => &fallback_image.d1,
            #render_path::render_resource::TextureViewDimension::D2 => &fallback_image.d2,
            #render_path::render_resource::TextureViewDimension::D2Array => &fallback_image.d2_array,
            #render_path::render_resource::TextureViewDimension::Cube => &fallback_image.cube,
            #render_path::render_resource::TextureViewDimension::CubeArray => &fallback_image.cube_array,
            #render_path::render_resource::TextureViewDimension::D3 => &fallback_image.d3,
        }
    }
}

/// Represents the arguments for the `uniform` binding attribute.
///
/// If parsed, represents an attribute
/// like `#[uniform(LitInt, Ident)]`
struct UniformBindingMeta {
    lit_int: LitInt,
    _comma: Comma,
    ident: Ident,
}

/// Represents the arguments for any general binding attribute.
///
/// If parsed, represents an attribute
/// like `#[foo(LitInt, ...)]` where the rest is optional [`Meta`].
enum BindingMeta {
    IndexOnly(LitInt),
    IndexWithOptions(BindingIndexOptions),
}

/// Represents the arguments for an attribute with a list of arguments.
///
/// This represents, for example, `#[texture(0, dimension = "2d_array")]`.
struct BindingIndexOptions {
    lit_int: LitInt,
    _comma: Comma,
    meta_list: Punctuated<Meta, Comma>,
}

impl Parse for BindingMeta {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek2(Comma) {
            input.parse().map(Self::IndexWithOptions)
        } else {
            input.parse().map(Self::IndexOnly)
        }
    }
}

impl Parse for BindingIndexOptions {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            lit_int: input.parse()?,
            _comma: input.parse()?,
            meta_list: input.parse_terminated(Meta::parse, Comma)?,
        })
    }
}

impl Parse for UniformBindingMeta {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            lit_int: input.parse()?,
            _comma: input.parse()?,
            ident: input.parse()?,
        })
    }
}

fn get_uniform_binding_attr(attr: &syn::Attribute) -> Result<(u32, Ident)> {
    let uniform_binding_meta = attr.parse_args_with(UniformBindingMeta::parse)?;

    let binding_index = uniform_binding_meta.lit_int.base10_parse()?;
    let ident = uniform_binding_meta.ident;

    Ok((binding_index, ident))
}

fn get_binding_nested_attr(attr: &syn::Attribute) -> Result<(u32, Vec<Meta>)> {
    let binding_meta = attr.parse_args_with(BindingMeta::parse)?;

    match binding_meta {
        BindingMeta::IndexOnly(lit_int) => Ok((lit_int.base10_parse()?, Vec::new())),
        BindingMeta::IndexWithOptions(BindingIndexOptions {
            lit_int,
            _comma: _,
            meta_list,
        }) => Ok((lit_int.base10_parse()?, meta_list.into_iter().collect())),
    }
}

#[derive(Default)]
enum ShaderStageVisibility {
    #[default]
    All,
    None,
    Flags(VisibilityFlags),
}

#[derive(Default)]
struct VisibilityFlags {
    vertex: bool,
    fragment: bool,
    compute: bool,
}

impl ShaderStageVisibility {
    fn vertex_fragment() -> Self {
        Self::Flags(VisibilityFlags::vertex_fragment())
    }

    fn compute() -> Self {
        Self::Flags(VisibilityFlags::compute())
    }
}

impl VisibilityFlags {
    fn vertex_fragment() -> Self {
        Self {
            vertex: true,
            fragment: true,
            ..Default::default()
        }
    }

    fn compute() -> Self {
        Self {
            compute: true,
            ..Default::default()
        }
    }
}

impl ShaderStageVisibility {
    fn hygienic_quote(&self, path: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        match self {
            ShaderStageVisibility::All => quote! { #path::ShaderStages::all() },
            ShaderStageVisibility::None => quote! { #path::ShaderStages::NONE },
            ShaderStageVisibility::Flags(flags) => {
                let mut quoted = Vec::new();

                if flags.vertex {
                    quoted.push(quote! { #path::ShaderStages::VERTEX });
                }
                if flags.fragment {
                    quoted.push(quote! { #path::ShaderStages::FRAGMENT });
                }
                if flags.compute {
                    quoted.push(quote! { #path::ShaderStages::COMPUTE });
                }

                quote! { #(#quoted)|* }
            }
        }
    }
}

const VISIBILITY: Symbol = Symbol("visibility");
const VISIBILITY_VERTEX: Symbol = Symbol("vertex");
const VISIBILITY_FRAGMENT: Symbol = Symbol("fragment");
const VISIBILITY_COMPUTE: Symbol = Symbol("compute");
const VISIBILITY_ALL: Symbol = Symbol("all");
const VISIBILITY_NONE: Symbol = Symbol("none");

fn get_visibility_flag_value(meta_list: &MetaList) -> Result<ShaderStageVisibility> {
    let mut flags = Vec::new();

    meta_list.parse_nested_meta(|meta| {
        flags.push(meta.path);
        Ok(())
    })?;

    if flags.is_empty() {
        return Err(Error::new_spanned(
            meta_list,
            "Invalid visibility format. Must be `visibility(flags)`, flags can be `all`, `none`, or a list-combination of `vertex`, `fragment` and/or `compute`."
        ));
    }

    if flags.len() == 1 {
        if let Some(flag) = flags.first() {
            if flag == VISIBILITY_ALL {
                return Ok(ShaderStageVisibility::All);
            } else if flag == VISIBILITY_NONE {
                return Ok(ShaderStageVisibility::None);
            }
        }
    }

    let mut visibility = VisibilityFlags::default();

    for flag in flags {
        if flag == VISIBILITY_VERTEX {
            visibility.vertex = true;
        } else if flag == VISIBILITY_FRAGMENT {
            visibility.fragment = true;
        } else if flag == VISIBILITY_COMPUTE {
            visibility.compute = true;
        } else {
            return Err(Error::new_spanned(
                flag,
                "Not a valid visibility flag. Must be `all`, `none`, or a list-combination of `vertex`, `fragment` and/or `compute`."
            ));
        }
    }

    Ok(ShaderStageVisibility::Flags(visibility))
}

#[derive(Clone, Copy, Default)]
enum BindingTextureDimension {
    D1,
    #[default]
    D2,
    D2Array,
    Cube,
    CubeArray,
    D3,
}

enum BindingTextureSampleType {
    Float { filterable: bool },
    Depth,
    Sint,
    Uint,
}

impl ToTokens for BindingTextureDimension {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(match self {
            BindingTextureDimension::D1 => quote! { TextureViewDimension::D1 },
            BindingTextureDimension::D2 => quote! { TextureViewDimension::D2 },
            BindingTextureDimension::D2Array => quote! { TextureViewDimension::D2Array },
            BindingTextureDimension::Cube => quote! { TextureViewDimension::Cube },
            BindingTextureDimension::CubeArray => quote! { TextureViewDimension::CubeArray },
            BindingTextureDimension::D3 => quote! { TextureViewDimension::D3 },
        });
    }
}

impl ToTokens for BindingTextureSampleType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(match self {
            BindingTextureSampleType::Float { filterable } => {
                quote! { TextureSampleType::Float { filterable: #filterable } }
            }
            BindingTextureSampleType::Depth => quote! { TextureSampleType::Depth },
            BindingTextureSampleType::Sint => quote! { TextureSampleType::Sint },
            BindingTextureSampleType::Uint => quote! { TextureSampleType::Uint },
        });
    }
}

struct TextureAttrs {
    dimension: BindingTextureDimension,
    sample_type: BindingTextureSampleType,
    multisampled: bool,
    visibility: ShaderStageVisibility,
}

impl Default for BindingTextureSampleType {
    fn default() -> Self {
        BindingTextureSampleType::Float { filterable: true }
    }
}

impl Default for TextureAttrs {
    fn default() -> Self {
        Self {
            dimension: Default::default(),
            sample_type: Default::default(),
            multisampled: true,
            visibility: Default::default(),
        }
    }
}

struct StorageTextureAttrs {
    dimension: BindingTextureDimension,
    // Parsing of the image_format parameter is deferred to the type checker,
    // which will error if the format is not member of the TextureFormat enum.
    image_format: proc_macro2::TokenStream,
    // Parsing of the access parameter is deferred to the type checker,
    // which will error if the access is not member of the StorageTextureAccess enum.
    access: proc_macro2::TokenStream,
    visibility: ShaderStageVisibility,
}

impl Default for StorageTextureAttrs {
    fn default() -> Self {
        Self {
            dimension: Default::default(),
            image_format: quote! { Rgba8Unorm },
            access: quote! { ReadWrite },
            visibility: ShaderStageVisibility::compute(),
        }
    }
}

fn get_storage_texture_binding_attr(metas: Vec<Meta>) -> Result<StorageTextureAttrs> {
    let mut storage_texture_attrs = StorageTextureAttrs::default();

    for meta in metas {
        use syn::Meta::{List, NameValue};
        match meta {
            // Parse #[storage_texture(0, dimension = "...")].
            NameValue(m) if m.path == DIMENSION => {
                let value = get_lit_str(DIMENSION, &m.value)?;
                storage_texture_attrs.dimension = get_texture_dimension_value(value)?;
            }
            // Parse #[storage_texture(0, format = ...))].
            NameValue(m) if m.path == IMAGE_FORMAT => {
                storage_texture_attrs.image_format = m.value.into_token_stream();
            }
            // Parse #[storage_texture(0, access = ...))].
            NameValue(m) if m.path == ACCESS => {
                storage_texture_attrs.access = m.value.into_token_stream();
            }
            // Parse #[storage_texture(0, visibility(...))].
            List(m) if m.path == VISIBILITY => {
                storage_texture_attrs.visibility = get_visibility_flag_value(&m)?;
            }
            NameValue(m) => {
                return Err(Error::new_spanned(
                    m.path,
                    "Not a valid name. Available attributes: `dimension`, `image_format`, `access`.",
                ));
            }
            _ => {
                return Err(Error::new_spanned(
                    meta,
                    "Not a name value pair: `foo = \"...\"`",
                ));
            }
        }
    }

    Ok(storage_texture_attrs)
}

const DIMENSION: Symbol = Symbol("dimension");
const IMAGE_FORMAT: Symbol = Symbol("image_format");
const ACCESS: Symbol = Symbol("access");
const SAMPLE_TYPE: Symbol = Symbol("sample_type");
const FILTERABLE: Symbol = Symbol("filterable");
const MULTISAMPLED: Symbol = Symbol("multisampled");

// Values for `dimension` attribute.
const DIM_1D: &str = "1d";
const DIM_2D: &str = "2d";
const DIM_3D: &str = "3d";
const DIM_2D_ARRAY: &str = "2d_array";
const DIM_CUBE: &str = "cube";
const DIM_CUBE_ARRAY: &str = "cube_array";

// Values for sample `type` attribute.
const FLOAT: &str = "float";
const DEPTH: &str = "depth";
const S_INT: &str = "s_int";
const U_INT: &str = "u_int";

fn get_texture_attrs(metas: Vec<Meta>) -> Result<TextureAttrs> {
    let mut dimension = Default::default();
    let mut sample_type = Default::default();
    let mut multisampled = Default::default();
    let mut filterable = None;
    let mut filterable_ident = None;

    let mut visibility = ShaderStageVisibility::vertex_fragment();

    for meta in metas {
        use syn::Meta::{List, NameValue};
        match meta {
            // Parse #[texture(0, dimension = "...")].
            NameValue(m) if m.path == DIMENSION => {
                let value = get_lit_str(DIMENSION, &m.value)?;
                dimension = get_texture_dimension_value(value)?;
            }
            // Parse #[texture(0, sample_type = "...")].
            NameValue(m) if m.path == SAMPLE_TYPE => {
                let value = get_lit_str(SAMPLE_TYPE, &m.value)?;
                sample_type = get_texture_sample_type_value(value)?;
            }
            // Parse #[texture(0, multisampled = "...")].
            NameValue(m) if m.path == MULTISAMPLED => {
                multisampled = get_lit_bool(MULTISAMPLED, &m.value)?;
            }
            // Parse #[texture(0, filterable = "...")].
            NameValue(m) if m.path == FILTERABLE => {
                filterable = get_lit_bool(FILTERABLE, &m.value)?.into();
                filterable_ident = m.path.into();
            }
            // Parse #[texture(0, visibility(...))].
            List(m) if m.path == VISIBILITY => {
                visibility = get_visibility_flag_value(&m)?;
            }
            NameValue(m) => {
                return Err(Error::new_spanned(
                    m.path,
                    "Not a valid name. Available attributes: `dimension`, `sample_type`, `multisampled`, or `filterable`."
                ));
            }
            _ => {
                return Err(Error::new_spanned(
                    meta,
                    "Not a name value pair: `foo = \"...\"`",
                ));
            }
        }
    }

    // Resolve `filterable` since the float
    // sample type is the one that contains the value.
    if let Some(filterable) = filterable {
        let path = filterable_ident.unwrap();
        match sample_type {
            BindingTextureSampleType::Float { filterable: _ } => {
                sample_type = BindingTextureSampleType::Float { filterable }
            }
            _ => {
                return Err(Error::new_spanned(
                    path,
                    "Type must be `float` to use the `filterable` attribute.",
                ));
            }
        };
    }

    Ok(TextureAttrs {
        dimension,
        sample_type,
        multisampled,
        visibility,
    })
}

fn get_texture_dimension_value(lit_str: &LitStr) -> Result<BindingTextureDimension> {
    match lit_str.value().as_str() {
        DIM_1D => Ok(BindingTextureDimension::D1),
        DIM_2D => Ok(BindingTextureDimension::D2),
        DIM_2D_ARRAY => Ok(BindingTextureDimension::D2Array),
        DIM_3D => Ok(BindingTextureDimension::D3),
        DIM_CUBE => Ok(BindingTextureDimension::Cube),
        DIM_CUBE_ARRAY => Ok(BindingTextureDimension::CubeArray),

        _ => Err(Error::new_spanned(
            lit_str,
            "Not a valid dimension. Must be `1d`, `2d`, `2d_array`, `3d`, `cube` or `cube_array`.",
        )),
    }
}

fn get_texture_sample_type_value(lit_str: &LitStr) -> Result<BindingTextureSampleType> {
    match lit_str.value().as_str() {
        FLOAT => Ok(BindingTextureSampleType::Float { filterable: true }),
        DEPTH => Ok(BindingTextureSampleType::Depth),
        S_INT => Ok(BindingTextureSampleType::Sint),
        U_INT => Ok(BindingTextureSampleType::Uint),

        _ => Err(Error::new_spanned(
            lit_str,
            "Not a valid sample type. Must be `float`, `depth`, `s_int` or `u_int`.",
        )),
    }
}

#[derive(Default)]
struct SamplerAttrs {
    sampler_binding_type: SamplerBindingType,
    visibility: ShaderStageVisibility,
}

#[derive(Default)]
enum SamplerBindingType {
    #[default]
    Filtering,
    NonFiltering,
    Comparison,
}

impl ToTokens for SamplerBindingType {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend(match self {
            SamplerBindingType::Filtering => quote! { SamplerBindingType::Filtering },
            SamplerBindingType::NonFiltering => quote! { SamplerBindingType::NonFiltering },
            SamplerBindingType::Comparison => quote! { SamplerBindingType::Comparison },
        });
    }
}

const SAMPLER_TYPE: Symbol = Symbol("sampler_type");

const FILTERING: &str = "filtering";
const NON_FILTERING: &str = "non_filtering";
const COMPARISON: &str = "comparison";

fn get_sampler_attrs(metas: Vec<Meta>) -> Result<SamplerAttrs> {
    let mut sampler_binding_type = Default::default();
    let mut visibility = ShaderStageVisibility::vertex_fragment();

    for meta in metas {
        use syn::Meta::{List, NameValue};
        match meta {
            // Parse #[sampler(0, sampler_type = "..."))].
            NameValue(m) if m.path == SAMPLER_TYPE => {
                let value = get_lit_str(DIMENSION, &m.value)?;
                sampler_binding_type = get_sampler_binding_type_value(value)?;
            }
            // Parse #[sampler(0, visibility(...))].
            List(m) if m.path == VISIBILITY => {
                visibility = get_visibility_flag_value(&m)?;
            }
            NameValue(m) => {
                return Err(Error::new_spanned(
                    m.path,
                    "Not a valid name. Available attributes: `sampler_type`.",
                ));
            }
            _ => {
                return Err(Error::new_spanned(
                    meta,
                    "Not a name value pair: `foo = \"...\"`",
                ));
            }
        }
    }

    Ok(SamplerAttrs {
        sampler_binding_type,
        visibility,
    })
}

fn get_sampler_binding_type_value(lit_str: &LitStr) -> Result<SamplerBindingType> {
    match lit_str.value().as_str() {
        FILTERING => Ok(SamplerBindingType::Filtering),
        NON_FILTERING => Ok(SamplerBindingType::NonFiltering),
        COMPARISON => Ok(SamplerBindingType::Comparison),

        _ => Err(Error::new_spanned(
            lit_str,
            "Not a valid dimension. Must be `filtering`, `non_filtering`, or `comparison`.",
        )),
    }
}

#[derive(Default)]
struct StorageAttrs {
    visibility: ShaderStageVisibility,
    read_only: bool,
    buffer: bool,
}

const READ_ONLY: Symbol = Symbol("read_only");
const BUFFER: Symbol = Symbol("buffer");

fn get_storage_binding_attr(metas: Vec<Meta>) -> Result<StorageAttrs> {
    let mut visibility = ShaderStageVisibility::vertex_fragment();
    let mut read_only = false;
    let mut buffer = false;

    for meta in metas {
        use syn::Meta::{List, Path};
        match meta {
            // Parse #[storage(0, visibility(...))].
            List(m) if m.path == VISIBILITY => {
                visibility = get_visibility_flag_value(&m)?;
            }
            Path(path) if path == READ_ONLY => {
                read_only = true;
            }
            Path(path) if path == BUFFER => {
                buffer = true;
            }
            _ => {
                return Err(Error::new_spanned(
                    meta,
                    "Not a valid attribute. Available attributes: `read_only`, `visibility`",
                ));
            }
        }
    }

    Ok(StorageAttrs {
        visibility,
        read_only,
        buffer,
    })
}
