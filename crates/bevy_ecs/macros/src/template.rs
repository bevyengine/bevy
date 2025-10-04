use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, FieldsUnnamed, Index, Path};

const TEMPLATE_ATTRIBUTE: &str = "template";
const TEMPLATE_DEFAULT_ATTRIBUTE: &str = "default";

pub(crate) fn derive_get_template(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let manifest = BevyManifest::shared();
    let bevy_ecs = manifest.get_path("bevy_ecs");

    let type_ident = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    let template_ident = format_ident!("{type_ident}Template");

    let is_pub = matches!(ast.vis, syn::Visibility::Public(_));
    let maybe_pub = if is_pub { quote!(pub) } else { quote!() };

    let template = match &ast.data {
        Data::Struct(data_struct) => {
            let StructImpl {
                template_fields,
                template_field_builds,
                template_field_defaults,
                ..
            } = struct_impl(&data_struct.fields, &bevy_ecs, false);
            match &data_struct.fields {
                Fields::Named(_) => {
                    quote! {
                        #[allow(missing_docs)]
                        #maybe_pub struct #template_ident #impl_generics #where_clause {
                            #(#template_fields,)*
                        }

                        impl #impl_generics #bevy_ecs::template::Template for #template_ident #type_generics #where_clause {
                            type Output = #type_ident #type_generics;
                            fn build(&mut self, entity: &mut #bevy_ecs::world::EntityWorldMut) -> #bevy_ecs::error::Result<Self::Output> {
                                Ok(#type_ident {
                                    #(#template_field_builds,)*
                                })
                            }
                        }

                        impl #impl_generics Default for #template_ident #type_generics #where_clause {
                            fn default() -> Self {
                                Self {
                                    #(#template_field_defaults,)*
                                }
                            }
                        }
                    }
                }
                Fields::Unnamed(_) => {
                    quote! {
                        #[allow(missing_docs)]
                        #maybe_pub struct #template_ident #impl_generics (
                            #(#template_fields,)*
                        )  #where_clause;

                        impl #impl_generics #bevy_ecs::template::Template for #template_ident #type_generics #where_clause {
                            type Output = #type_ident #type_generics;
                            fn build(&mut self, entity: &mut #bevy_ecs::world::EntityWorldMut) -> #bevy_ecs::error::Result<Self::Output> {
                                Ok(#type_ident (
                                    #(#template_field_builds,)*
                                ))
                            }
                        }

                        impl #impl_generics Default for #template_ident #type_generics #where_clause {
                            fn default() -> Self {
                                Self (
                                    #(#template_field_defaults,)*
                                )
                            }
                        }
                    }
                }
                Fields::Unit => {
                    quote! {
                        #[allow(missing_docs)]
                        #maybe_pub struct #template_ident;

                        impl #impl_generics #bevy_ecs::template::Template for #template_ident #type_generics #where_clause {
                            type Output = #type_ident;
                            fn build(&mut self, entity: &mut #bevy_ecs::world::EntityWorldMut) -> #bevy_ecs::error::Result<Self::Output> {
                                Ok(#type_ident)
                            }
                        }

                        impl #impl_generics Default for #template_ident #type_generics #where_clause {
                            fn default() -> Self {
                                Self
                            }
                        }
                    }
                }
            }
        }
        Data::Enum(data_enum) => {
            let mut variant_definitions = Vec::new();
            let mut variant_builds = Vec::new();
            let mut variant_default_ident = None;
            let mut variant_defaults = Vec::new();
            for variant in &data_enum.variants {
                let StructImpl {
                    template_fields,
                    template_field_builds,
                    template_field_defaults,
                    ..
                } = struct_impl(&variant.fields, &bevy_ecs, true);

                let is_default = variant
                    .attrs
                    .iter()
                    .find(|a| a.path().is_ident(TEMPLATE_DEFAULT_ATTRIBUTE))
                    .is_some();
                if is_default {
                    if variant_default_ident.is_some() {
                        panic!("Cannot have multiple default variants");
                    }
                }
                let variant_ident = &variant.ident;
                let variant_name_lower = variant_ident.to_string().to_lowercase();
                let variant_default_name = format_ident!("default_{}", variant_name_lower);
                match &variant.fields {
                    Fields::Named(fields) => {
                        variant_definitions.push(quote! {
                            #variant_ident {
                                #(#template_fields,)*
                            }
                        });
                        let field_idents = fields.named.iter().map(|f| &f.ident);
                        variant_builds.push(quote! {
                            // TODO: proper assignments here
                            #template_ident::#variant_ident {
                                #(#field_idents,)*
                            } => {
                                #type_ident::#variant_ident {
                                    #(#template_field_builds,)*
                                }
                            }
                        });

                        if is_default {
                            variant_default_ident = Some(quote! {
                                Self::#variant_ident {
                                    #(#template_field_defaults,)*
                                }
                            });
                        }
                        variant_defaults.push(quote! {
                            #maybe_pub fn #variant_default_name() -> Self {
                                Self::#variant_ident {
                                    #(#template_field_defaults,)*
                                }
                            }
                        })
                    }
                    Fields::Unnamed(FieldsUnnamed { unnamed: f, .. }) => {
                        let field_idents =
                            f.iter().enumerate().map(|(i, _)| format_ident!("t{}", i));
                        variant_definitions.push(quote! {
                            #variant_ident(#(#template_fields,)*)
                        });
                        variant_builds.push(quote! {
                            // TODO: proper assignments here
                            #template_ident::#variant_ident(
                                #(#field_idents,)*
                             ) => {
                                #type_ident::#variant_ident(
                                    #(#template_field_builds,)*
                                )
                            }
                        });
                        if is_default {
                            variant_default_ident = Some(quote! {
                                Self::#variant_ident(
                                    #(#template_field_defaults,)*
                                )
                            });
                        }

                        variant_defaults.push(quote! {
                            #maybe_pub fn #variant_default_name() -> Self {
                                Self::#variant_ident(
                                    #(#template_field_defaults,)*
                                )
                            }
                        })
                    }
                    Fields::Unit => {
                        variant_definitions.push(quote! {#variant_ident});
                        variant_builds.push(
                            quote! {#template_ident::#variant_ident => #type_ident::#variant_ident},
                        );
                        if is_default {
                            variant_default_ident = Some(quote! {
                                Self::#variant_ident
                            });
                        }
                        variant_defaults.push(quote! {
                            #maybe_pub fn #variant_default_name() -> Self {
                                Self::#variant_ident
                            }
                        })
                    }
                }
            }

            if variant_default_ident.is_none() {
                panic!("Deriving Template for enums requires picking a default variant using #[default]");
            }

            quote! {
                #[allow(missing_docs)]
                #maybe_pub enum #template_ident #type_generics #where_clause {
                    #(#variant_definitions,)*
                }

                impl #impl_generics #template_ident #type_generics #where_clause {
                    #(#variant_defaults)*
                }

                impl #impl_generics #bevy_ecs::template::Template for #template_ident #type_generics #where_clause {
                    type Output = #type_ident #type_generics;
                    fn build(&mut self, entity: &mut #bevy_ecs::world::EntityWorldMut) -> #bevy_ecs::error::Result<Self::Output> {
                        Ok(match self {
                            #(#variant_builds,)*
                        })
                    }
                }

                impl #impl_generics Default for #template_ident #type_generics #where_clause {
                    fn default() -> Self {
                        #variant_default_ident
                    }
                }
            }
        }
        Data::Union(_) => panic!("Union types are not supported yet."),
    };

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs::template::GetTemplate for #type_ident #type_generics #where_clause {
            type Template = #template_ident #type_generics;
        }

        #template
    })
}

struct StructImpl {
    template_fields: Vec<proc_macro2::TokenStream>,
    template_field_builds: Vec<proc_macro2::TokenStream>,
    template_field_defaults: Vec<proc_macro2::TokenStream>,
}

fn struct_impl(fields: &Fields, bevy_ecs: &Path, is_enum: bool) -> StructImpl {
    let mut template_fields = Vec::with_capacity(fields.len());
    let mut template_field_builds = Vec::with_capacity(fields.len());
    let mut template_field_defaults = Vec::with_capacity(fields.len());
    let is_named = matches!(fields, Fields::Named(_));
    for (index, field) in fields.iter().enumerate() {
        let is_template = field
            .attrs
            .iter()
            .find(|a| a.path().is_ident(TEMPLATE_ATTRIBUTE))
            .is_some();
        let is_pub = matches!(field.vis, syn::Visibility::Public(_));
        let field_maybe_pub = if is_pub { quote!(pub) } else { quote!() };
        let ident = &field.ident;
        let ty = &field.ty;
        let index = Index::from(index);
        if is_named {
            if is_template {
                template_fields.push(quote! {
                    #field_maybe_pub #ident: #bevy_ecs::template::TemplateField<<#ty as #bevy_ecs::template::GetTemplate>::Template>
                });
                if is_enum {
                    template_field_builds.push(quote! {
                        #ident: match #ident {
                            #bevy_ecs::template::TemplateField::Template(template) => template.build(entity)?,
                            #bevy_ecs::template::TemplateField::Value(value) => Clone::clone(value),
                        }
                    });
                } else {
                    template_field_builds.push(quote! {
                        #ident: match &mut self.#ident {
                            #bevy_ecs::template::TemplateField::Template(template) => template.build(entity)?,
                            #bevy_ecs::template::TemplateField::Value(value) => Clone::clone(value),
                        }
                    });
                }
                template_field_defaults.push(quote! {
                    #ident: Default::default()
                });
            } else {
                template_fields.push(quote! {
                    #field_maybe_pub #ident: <#ty as #bevy_ecs::template::GetTemplate>::Template
                });
                if is_enum {
                    template_field_builds.push(quote! {
                        #ident: #ident.build(entity)?
                    });
                } else {
                    template_field_builds.push(quote! {
                        #ident: self.#ident.build(entity)?
                    });
                }

                template_field_defaults.push(quote! {
                    #ident: Default::default()
                });
            }
        } else {
            if is_template {
                template_fields.push(quote! {
                    #field_maybe_pub #bevy_ecs::template::TemplateField<<#ty as #bevy_ecs::template::GetTemplate>::Template>
                });
                if is_enum {
                    let enum_tuple_ident = format_ident!("t{}", index);
                    template_field_builds.push(quote! {
                        match #enum_tuple_ident {
                            #bevy_ecs::template::TemplateField::Template(template) => template.build(entity)?,
                            #bevy_ecs::template::TemplateField::Value(value) => Clone::clone(value),
                        }
                    });
                } else {
                    template_field_builds.push(quote! {
                        match &mut self.#index {
                            #bevy_ecs::template::TemplateField::Template(template) => template.build(entity)?,
                            #bevy_ecs::template::TemplateField::Value(value) => Clone::clone(value),
                        }
                    });
                }
                template_field_defaults.push(quote! {
                    Default::default()
                });
            } else {
                template_fields.push(quote! {
                    #field_maybe_pub <#ty as #bevy_ecs::template::GetTemplate>::Template
                });
                if is_enum {
                    let enum_tuple_ident = format_ident!("t{}", index);
                    template_field_builds.push(quote! {
                        #enum_tuple_ident.build(entity)?
                    });
                } else {
                    template_field_builds.push(quote! {
                        self.#index.build(entity)?
                    });
                }
                template_field_defaults.push(quote! {
                    Default::default()
                });
            }
        }
    }
    StructImpl {
        template_fields,
        template_field_builds,
        template_field_defaults,
    }
}
