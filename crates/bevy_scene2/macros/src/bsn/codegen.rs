use crate::bsn::types::{
    Bsn, BsnConstructor, BsnEntry, BsnFields, BsnInheritedScene, BsnRelatedSceneList, BsnRoot,
    BsnSceneListItem, BsnSceneListItems, BsnType, BsnValue,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{Ident, Index, Lit, Member, Path};

impl BsnRoot {
    pub fn to_tokens(&self, bevy_scene: &Path, bevy_ecs: &Path, bevy_asset: &Path) -> TokenStream {
        self.0.to_tokens(bevy_scene, bevy_ecs, bevy_asset)
    }
}

impl<const ALLOW_FLAT: bool> Bsn<ALLOW_FLAT> {
    pub fn to_tokens(&self, bevy_scene: &Path, bevy_ecs: &Path, bevy_asset: &Path) -> TokenStream {
        let mut entries = Vec::with_capacity(self.entries.len());
        for bsn_entry in &self.entries {
            entries.push(match bsn_entry {
                BsnEntry::TemplatePatch(bsn_type) => {
                    let mut assignments = Vec::new();
                    bsn_type.to_patch_tokens(
                        bevy_ecs,
                        bevy_scene,
                        &mut assignments,
                        true,
                        &[Member::Named(Ident::new(
                            "value",
                            proc_macro2::Span::call_site(),
                        ))],
                        true,
                    );
                    let path = &bsn_type.path;
                    quote! {
                        <#path as #bevy_scene::PatchTemplate>::patch_template(move |value| {
                            #(#assignments)*
                        })
                    }
                }
                BsnEntry::GetTemplatePatch(bsn_type) => {
                    let mut assignments = Vec::new();
                    bsn_type.to_patch_tokens(
                        bevy_ecs,
                        bevy_scene,
                        &mut assignments,
                        true,
                        &[Member::Named(Ident::new(
                            "value",
                            proc_macro2::Span::call_site(),
                        ))],
                        true,
                    );
                    let path = &bsn_type.path;
                    quote! {
                        <#path as #bevy_scene::PatchGetTemplate>::patch(move |value| {
                            #(#assignments)*
                        })
                    }
                }
                BsnEntry::TemplateConst {
                    type_path,
                    const_ident,
                } => {
                    quote! {
                        <#type_path as #bevy_scene::PatchTemplate>::patch_template(
                            move |value| {
                                *value = #type_path::#const_ident;
                            },
                        )
                    }
                }
                BsnEntry::SceneExpression(block) => {
                    quote! {#block}
                }
                BsnEntry::TemplateConstructor(BsnConstructor {
                    type_path,
                    function,
                    args,
                }) => {
                    quote! {
                        <#type_path as #bevy_scene::PatchTemplate>::patch_template(
                            move |value| {
                                *value = #type_path::#function(#args);
                            },
                        )
                    }
                }
                BsnEntry::GetTemplateConstructor(BsnConstructor {
                    type_path,
                    function,
                    args,
                }) => {
                    // NOTE: The odd turbofish line break below avoids breaking rustfmt
                    quote! {
                        <#type_path as #bevy_scene::PatchGetTemplate>::patch(
                            move |value| {
                                *value = <#type_path as #bevy_ecs::template::GetTemplate>
                                    ::Template::#function(#args);
                            }
                        )
                    }
                }
                BsnEntry::ChildrenSceneList(scene_list) => {
                    let scenes = scene_list.0.to_tokens(bevy_scene, bevy_ecs, bevy_asset);
                    quote! {
                        #bevy_scene::RelatedScenes::<#bevy_ecs::hierarchy::ChildOf, _>::new(#scenes)
                    }
                }
                BsnEntry::RelatedSceneList(BsnRelatedSceneList {
                    scene_list,
                    relationship_path,
                }) => {
                    let scenes = scene_list.0.to_tokens(bevy_scene, bevy_ecs, bevy_asset);
                    // NOTE: The odd turbofish line breaks below avoid breaking rustfmt
                    quote! {
                        #bevy_scene::RelatedScenes::<
                            <#relationship_path as #bevy_ecs::relationship::RelationshipTarget>
                                ::Relationship,
                            _
                        >::new(
                            #scenes
                        )
                    }
                }
                BsnEntry::InheritedScene(inherited_scene) => match inherited_scene {
                    BsnInheritedScene::Asset(lit_str) => {
                        quote! {#bevy_scene::InheritSceneAsset::from(#lit_str)}
                    }
                    BsnInheritedScene::Fn { function, args } => {
                        quote! {#bevy_scene::InheritScene(#function(#args))}
                    }
                },
                BsnEntry::Name(ident) => {
                    let name = ident.to_string();
                    quote! {
                        <#bevy_ecs::name::Name as PatchGetTemplate>::patch(
                            move |value| {
                                *value = Name(#name.into());
                            }
                        )
                    }
                }
                BsnEntry::NameExpression(expr_tokens) => {
                    quote! {
                        <#bevy_ecs::name::Name as PatchGetTemplate>::patch(
                            move |value| {
                                *value = Name({#expr_tokens}.into());
                            }
                        )
                    }
                }
            });
        }

        quote! {(#(#entries,)*)}
    }
}

macro_rules! field_value_type {
    () => {
        BsnValue::Expr(_)
            | BsnValue::Closure(_)
            | BsnValue::Ident(_)
            | BsnValue::Lit(_)
            | BsnValue::Tuple(_)
    };
}

impl BsnType {
    fn to_patch_tokens(
        &self,
        bevy_ecs: &Path,
        bevy_scene: &Path,
        assignments: &mut Vec<TokenStream>,
        is_root_template: bool,
        field_path: &[Member],
        is_path_ref: bool,
    ) {
        let path = &self.path;
        if !is_root_template {
            assignments.push(quote! {#bevy_scene::touch_type::<#path>();});
        }
        let maybe_deref = is_path_ref.then(|| quote! {*});
        let maybe_borrow_mut = (!is_path_ref).then(|| quote! {&mut});
        if let Some(variant) = &self.enum_variant {
            let variant_name_lower = variant.to_string().to_lowercase();
            let variant_default_name = format_ident!("default_{}", variant_name_lower);
            match &self.fields {
                BsnFields::Named(fields) => {
                    let field_assignments = fields.iter().map(|f| {
                        let name = &f.name;
                        let value = &f.value;
                        if let Some(BsnValue::Type(bsn_type)) = &value {
                            if bsn_type.enum_variant.is_some() {
                                quote! {*#name = #bsn_type;}
                            } else {
                                let mut type_assignments = Vec::new();
                                bsn_type.to_patch_tokens(
                                    bevy_ecs,
                                    bevy_scene,
                                    &mut type_assignments,
                                    false,
                                    &[Member::Named(name.clone())],
                                    true,
                                );
                                quote! {#(#type_assignments)*}
                            }
                        } else {
                            quote! {*#name = #value;}
                        }
                    });
                    let field_names = fields.iter().map(|f| &f.name);
                    assignments.push(quote! {
                        if !matches!(#(#field_path).*, #bevy_ecs::template::Wrapper::<<#path as #bevy_ecs::template::GetTemplate>::Template>::#variant { .. }) {
                            #maybe_deref #(#field_path).* = #bevy_ecs::template::Wrapper::<<#path as #bevy_ecs::template::GetTemplate>::Template>::#variant_default_name();
                        }
                        if let #bevy_ecs::template::Wrapper::<<#path as #bevy_ecs::template::GetTemplate>::Template>::#variant { #(#field_names, )*.. } = #maybe_borrow_mut #(#field_path).* {
                            #(#field_assignments)*
                        }
                    })
                }
                BsnFields::Tuple(fields) => {
                    // root template enums produce per-field "patches", at the cost of requiring the EnumDefaults pattern
                    let field_assignments = fields.iter().enumerate().map(|(index, f)| {
                        let name = format_ident!("t{}", index);
                        let value = &f.value;
                        if let BsnValue::Type(bsn_type) = &value {
                            if bsn_type.enum_variant.is_some() {
                                quote! {*#name = #bsn_type;}
                            } else {
                                let mut type_assignments = Vec::new();
                                bsn_type.to_patch_tokens(
                                    bevy_ecs,
                                    bevy_scene,
                                    &mut type_assignments,
                                    false,
                                    &[Member::Named(name.clone())],
                                    true,
                                );
                                quote! {#(#type_assignments)*}
                            }
                        } else {
                            quote! {*#name = #value;}
                        }
                    });
                    let field_names = fields
                        .iter()
                        .enumerate()
                        .map(|(index, _)| format_ident!("t{}", index));
                    assignments.push(quote! {
                        if !matches!(#(#field_path).*, #bevy_ecs::template::Wrapper::<<#path as #bevy_ecs::template::GetTemplate>::Template>::#variant(..)) {
                            #maybe_deref #(#field_path).* = #bevy_ecs::template::Wrapper::<<#path as #bevy_ecs::template::GetTemplate>::Template>::#variant_default_name();
                        }
                        if let #bevy_ecs::template::Wrapper::<<#path as #bevy_ecs::template::GetTemplate>::Template>::#variant (#(#field_names, )*.. ) = #maybe_borrow_mut #(#field_path).* {
                            #(#field_assignments)*
                        }
                    })
                }
            }
        } else {
            match &self.fields {
                BsnFields::Named(fields) => {
                    for field in fields {
                        let field_name = &field.name;
                        let field_value = &field.value;
                        match field_value {
                            // NOTE: It is very important to still produce outputs for None field values. This is what
                            // enables field autocomplete in Rust Analyzer
                            Some(field_value_type!()) | None => {
                                if field.is_template {
                                    assignments
                                        .push(quote! {#(#field_path.)*#field_name = #bevy_ecs::template::TemplateField::Template(#field_value);});
                                } else {
                                    assignments
                                        .push(quote! {#(#field_path.)*#field_name = #field_value;});
                                }
                            }
                            Some(BsnValue::Type(field_type)) => {
                                if field_type.enum_variant.is_some() {
                                    assignments
                                        .push(quote! {#(#field_path.)*#field_name = #field_type;});
                                } else {
                                    let mut new_field_path = field_path.to_vec();
                                    new_field_path.push(Member::Named(field_name.clone()));
                                    field_type.to_patch_tokens(
                                        bevy_ecs,
                                        bevy_scene,
                                        assignments,
                                        false,
                                        &new_field_path,
                                        false,
                                    );
                                }
                            }
                        }
                    }
                }
                BsnFields::Tuple(fields) => {
                    for (index, field) in fields.iter().enumerate() {
                        let field_index = Index::from(index);
                        let field_value = &field.value;
                        match field_value {
                            field_value_type!() => {
                                if field.is_template {
                                    assignments.push(
                                        quote! {#(#field_path.)*#field_index = #bevy_ecs::template::TemplateField::Template(#field_value);},
                                    );
                                } else {
                                    assignments.push(
                                        quote! {#(#field_path.)*#field_index = #field_value;},
                                    );
                                }
                            }
                            BsnValue::Type(field_type) => {
                                if field_type.enum_variant.is_some() {
                                    assignments
                                        .push(quote! {#(#field_path.)*#field_index = #field_type;});
                                } else {
                                    let mut new_field_path = field_path.to_vec();
                                    new_field_path.push(Member::Unnamed(field_index));
                                    field_type.to_patch_tokens(
                                        bevy_ecs,
                                        bevy_scene,
                                        assignments,
                                        false,
                                        &new_field_path,
                                        false,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl BsnSceneListItems {
    pub fn to_tokens(&self, bevy_scene: &Path, bevy_ecs: &Path, bevy_asset: &Path) -> TokenStream {
        let scenes = self.0.iter().map(|scene| match scene {
            BsnSceneListItem::Scene(bsn) => {
                let tokens = bsn.to_tokens(bevy_scene, bevy_ecs, bevy_asset);
                quote! {#bevy_scene::EntityScene(#tokens)}
            }
            BsnSceneListItem::Expression(block) => quote! {#block},
        });
        quote! {
            (#(#scenes,)*)
        }
    }
}

impl ToTokens for BsnType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let path = &self.path;
        let maybe_variant = if let Some(variant) = &self.enum_variant {
            Some(quote! {::#variant})
        } else {
            None
        };
        let result = match &self.fields {
            BsnFields::Named(fields) => {
                let assignments = fields.iter().map(|f| {
                    let name = &f.name;
                    let value = &f.value;
                    quote! {#name: #value}
                });
                quote! {
                    #path #maybe_variant {
                        #(#assignments,)*
                    }
                }
            }
            BsnFields::Tuple(fields) => {
                let assignments = fields.iter().map(|f| &f.value);
                quote! {
                    #path #maybe_variant (
                        #(#assignments,)*
                    )
                }
            }
        };
        result.to_tokens(tokens);
    }
}

impl ToTokens for BsnValue {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            BsnValue::Expr(expr_tokens) => {
                quote! {{#expr_tokens}.into()}.to_tokens(tokens);
            }
            BsnValue::Closure(closure_tokens) => {
                quote! {(#closure_tokens).into()}.to_tokens(tokens);
            }
            BsnValue::Ident(ident) => {
                quote! {(#ident).into()}.to_tokens(tokens);
            }
            BsnValue::Lit(lit) => match lit {
                Lit::Str(str) => quote! {#str.into()}.to_tokens(tokens),
                _ => lit.to_tokens(tokens),
            },
            BsnValue::Tuple(tuple) => {
                let tuple_tokens = tuple.0.iter();
                quote! {(#(#tuple_tokens),*)}.to_tokens(tokens);
            }
            BsnValue::Type(ty) => {
                ty.to_tokens(tokens);
            }
        };
    }
}
