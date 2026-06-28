use bevy_macro_utils::{fq_std::FQDefault, BevyManifest};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::ParseStream, parse_macro_input, parse_quote, punctuated::Punctuated, spanned::Spanned,
    Data, DeriveInput, Fields, FieldsUnnamed, Ident, Index, Path, Result, Token, WhereClause,
};

const TEMPLATE_DEFAULT_ATTRIBUTE: &str = "default";
const TEMPLATE_ATTRIBUTE: &str = "template";
const BUILT_IN_ATTRIBUTE: &str = "built_in";

pub(crate) fn derive_from_template(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs = BevyManifest::shared(|manifest| manifest.get_path("bevy_ecs"));

    let type_ident = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    let template_ident = format_ident!("{type_ident}Template");

    let is_pub = matches!(ast.vis, syn::Visibility::Public(_));
    let maybe_pub = if is_pub { quote!(pub) } else { quote!() };

    let template = match &ast.data {
        Data::Struct(data_struct) => {
            let result = match struct_impl(&data_struct.fields, &bevy_ecs, false) {
                Ok(result) => result,
                Err(err) => return err.into_compile_error().into(),
            };
            let StructImpl {
                template_fields,
                template_field_builds,
                template_field_defaults,
                template_field_clones,
                ..
            } = result;
            match &data_struct.fields {
                Fields::Named(_) => {
                    quote! {
                        #[allow(missing_docs)]
                        #maybe_pub struct #template_ident #impl_generics #where_clause {
                            #(#template_fields,)*
                        }

                        impl #impl_generics #bevy_ecs::template::Template for #template_ident #type_generics #where_clause {
                            type Output = #type_ident #type_generics;
                            fn build_template(&self, context: &mut #bevy_ecs::template::TemplateContext) -> #bevy_ecs::error::Result<Self::Output> {
                                #bevy_ecs::error::Result::Ok(#type_ident {
                                    #(#template_field_builds,)*
                                })
                            }

                            fn clone_template(&self) -> Self {
                                Self {
                                    #(#template_field_clones,)*
                                }
                            }
                        }

                        impl #impl_generics #FQDefault for #template_ident #type_generics #where_clause {
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
                            fn build_template(&self, context: &mut #bevy_ecs::template::TemplateContext) -> #bevy_ecs::error::Result<Self::Output> {
                                #bevy_ecs::error::Result::Ok(#type_ident (
                                    #(#template_field_builds,)*
                                ))
                            }

                            fn clone_template(&self) -> Self {
                                Self(
                                    #(#template_field_clones,)*
                                )
                            }
                        }

                        impl #impl_generics #FQDefault for #template_ident #type_generics #where_clause {
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
                            fn build_template(&self, context: &mut #bevy_ecs::template::TemplateContext) -> #bevy_ecs::error::Result<Self::Output> {
                                #bevy_ecs::error::Result::Ok(#type_ident)
                            }

                            fn clone_template(&self) -> Self {
                                Self
                            }
                        }

                        impl #impl_generics #FQDefault for #template_ident #type_generics #where_clause {
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
            let mut variant_clones = Vec::new();
            let mut variant_default_ident = None;
            let mut variant_defaults = Vec::new();
            for variant in &data_enum.variants {
                let result = match struct_impl(&variant.fields, &bevy_ecs, true) {
                    Ok(result) => result,
                    Err(err) => return err.into_compile_error().into(),
                };
                let StructImpl {
                    template_fields,
                    template_field_builds,
                    template_field_defaults,
                    template_field_clones,
                    ..
                } = result;

                let is_default = variant
                    .attrs
                    .iter()
                    .any(|a| a.path().is_ident(TEMPLATE_DEFAULT_ATTRIBUTE));
                if is_default && variant_default_ident.is_some() {
                    panic!("Cannot have multiple default variants");
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

                        let field_idents = fields.named.iter().map(|f| &f.ident);
                        variant_clones.push(quote! {
                            // TODO: proper assignments here
                            #template_ident::#variant_ident {
                                #(#field_idents,)*
                            } => {
                                #template_ident::#variant_ident {
                                    #(#template_field_clones,)*
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
                            /// Default value for this variant, generated by a `FromTemplate` derive.
                            #maybe_pub fn #variant_default_name() -> Self {
                                Self::#variant_ident {
                                    #(#template_field_defaults,)*
                                }
                            }
                        });
                    }
                    Fields::Unnamed(FieldsUnnamed { unnamed: f, .. }) => {
                        let field_idents = f
                            .iter()
                            .enumerate()
                            .map(|(i, _)| format_ident!("t{}", i))
                            .collect::<Vec<_>>();
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
                        variant_clones.push(quote! {
                            #template_ident::#variant_ident(
                                #(#field_idents,)*
                             ) => {
                                #template_ident::#variant_ident(
                                    #(#template_field_clones,)*
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
                            /// Default value for this variant, generated by a `FromTemplate` derive.
                            #maybe_pub fn #variant_default_name() -> Self {
                                Self::#variant_ident(
                                    #(#template_field_defaults,)*
                                )
                            }
                        });
                    }
                    Fields::Unit => {
                        variant_definitions.push(quote! {#variant_ident});
                        variant_builds.push(
                            quote! {#template_ident::#variant_ident => #type_ident::#variant_ident},
                        );
                        variant_clones.push(
                            quote! {#template_ident::#variant_ident => #template_ident::#variant_ident},
                        );
                        if is_default {
                            variant_default_ident = Some(quote! {
                                Self::#variant_ident
                            });
                        }
                        variant_defaults.push(quote! {
                            /// Default value for this variant, generated by a `FromTemplate` derive.
                            #maybe_pub fn #variant_default_name() -> Self {
                                Self::#variant_ident
                            }
                        });
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
                    fn build_template(&self, context: &mut #bevy_ecs::template::TemplateContext) -> #bevy_ecs::error::Result<Self::Output> {
                        #bevy_ecs::error::Result::Ok(match self {
                            #(#variant_builds,)*
                        })
                    }

                    fn clone_template(&self) -> Self {
                        match self {
                            #(#variant_clones,)*
                        }
                    }
                }

                impl #impl_generics #FQDefault for #template_ident #type_generics #where_clause {
                    fn default() -> Self {
                        #variant_default_ident
                    }
                }
            }
        }
        Data::Union(_) => panic!("Union types are not supported yet."),
    };

    let mut unpin_where_clause = where_clause.cloned().unwrap_or_else(|| WhereClause {
        where_token: <Token![where]>::default(),
        predicates: Punctuated::new(),
    });

    unpin_where_clause
        .predicates
        .push(parse_quote! { for<'a> [()]: #bevy_ecs::template::SpecializeFromTemplate });

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs::template::FromTemplate for #type_ident #type_generics #where_clause {
            type Template = #template_ident #type_generics;
        }

        impl #impl_generics ::core::marker::Unpin for #type_ident #type_generics #unpin_where_clause {}

        #template
    })
}

struct StructImpl {
    template_fields: Vec<proc_macro2::TokenStream>,
    template_field_builds: Vec<proc_macro2::TokenStream>,
    template_field_defaults: Vec<proc_macro2::TokenStream>,
    template_field_clones: Vec<proc_macro2::TokenStream>,
}

enum TemplateType {
    FromTemplate,
    BuiltIn,
    Manual(Path),
}

fn struct_impl(fields: &Fields, bevy_ecs: &Path, is_enum: bool) -> Result<StructImpl> {
    let mut template_fields = Vec::with_capacity(fields.len());
    let mut template_field_builds = Vec::with_capacity(fields.len());
    let mut template_field_defaults = Vec::with_capacity(fields.len());
    let mut template_field_clones = Vec::with_capacity(fields.len());
    let is_named = matches!(fields, Fields::Named(_));
    for (index, field) in fields.iter().enumerate() {
        let is_pub = matches!(field.vis, syn::Visibility::Public(_));
        let field_maybe_pub = if is_pub { quote!(pub) } else { quote!() };
        let ident = &field.ident;
        let ty = &field.ty;
        let index = Index::from(index);
        let mut template_type = TemplateType::FromTemplate;
        for attr in &field.attrs {
            if attr.path().is_ident(TEMPLATE_ATTRIBUTE) {
                attr.parse_args_with(|stream: ParseStream| {
                    let forked = stream.fork();
                    let ident = forked.parse::<Ident>()?;
                    if ident == BUILT_IN_ATTRIBUTE {
                        stream.parse::<Ident>()?;
                        template_type = TemplateType::BuiltIn;
                    } else {
                        if let Ok(path) = stream.parse::<Path>() {
                            template_type = TemplateType::Manual(path);
                        } else {
                            return Err(syn::Error::new(
                                attr.span(),
                                "Expected a Template type path",
                            ));
                        }
                    }
                    Ok(())
                })?;
            }
        }

        let template_type = match template_type {
            TemplateType::FromTemplate => {
                quote!(<#ty as #bevy_ecs::template::FromTemplate>::Template)
            }
            TemplateType::BuiltIn => {
                quote!(<#ty as #bevy_ecs::template::BuiltInTemplate>::Template)
            }
            TemplateType::Manual(path) => quote! {#path},
        };

        if is_named {
            template_fields.push(quote! {
                #field_maybe_pub #ident: #template_type
            });
            if is_enum {
                template_field_builds.push(quote! {
                    #ident: #ident.build_template(context)?
                });
                template_field_clones.push(quote! {
                    #ident: #bevy_ecs::template::Template::clone_template(#ident)
                });
            } else {
                template_field_builds.push(quote! {
                    #ident: self.#ident.build_template(context)?
                });
                template_field_clones.push(quote! {
                    #ident: #bevy_ecs::template::Template::clone_template(&self.#ident)
                });
            }

            template_field_defaults.push(quote! {
                #ident: #FQDefault::default()
            });
        } else {
            template_fields.push(quote! {
                #field_maybe_pub #template_type
            });
            if is_enum {
                let enum_tuple_ident = format_ident!("t{}", index);
                template_field_builds.push(quote! {
                    #enum_tuple_ident.build_template(context)?
                });
                template_field_clones.push(quote! {
                    #bevy_ecs::template::Template::clone_template(#enum_tuple_ident)
                });
            } else {
                template_field_builds.push(quote! {
                    self.#index.build_template(context)?
                });
                template_field_clones.push(quote! {
                    #bevy_ecs::template::Template::clone_template(&self.#index)
                });
            }
            template_field_defaults.push(quote! {
                #FQDefault::default()
            });
        }
    }
    Ok(StructImpl {
        template_fields,
        template_field_builds,
        template_field_defaults,
        template_field_clones,
    })
}
