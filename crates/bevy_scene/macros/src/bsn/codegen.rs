use crate::bsn::types::{
    Bsn, BsnConstructor, BsnEntry, BsnFields, BsnInheritedScene, BsnListRoot, BsnRelatedSceneList,
    BsnRoot, BsnSceneListItem, BsnSceneListItems, BsnType, BsnValue,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use std::collections::{hash_map::Entry, HashMap, HashSet};
use syn::{parse::Parse, Ident, Index, Lit, Member, Path};

/// Tracks named entity references and assigns them unique, sequential indices
/// during the code generation process.
#[derive(Default)]
pub(crate) struct EntityRefs {
    refs: HashMap<String, usize>,
    next: usize,
}

impl EntityRefs {
    /// Retrieves the index for a given entity name.
    /// Creates a new one if it hasn't been seen yet.
    fn get(&mut self, name: String) -> usize {
        match self.refs.entry(name) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let index = self.next;
                entry.insert(index);
                self.next += 1;
                index
            }
        }
    }
}

/// Context used in the [`Bsn`] code generation pipeline.
/// Used to accumulate validation errors without short-circuiting.
pub(crate) struct BsnCodegenCtx<'a> {
    pub bevy_scene: &'a Path,
    pub bevy_ecs: &'a Path,
    pub entity_refs: &'a mut EntityRefs,
    /// Accumulated parsing and validation errors.
    pub errors: Vec<syn::Error>,
}

/// Represents the target path and whether it is a reference, e.g.,
/// when applying a template patch.
struct PatchTarget<'a> {
    /// The path to the field being patched.
    pub path: &'a [Member],
    /// Whether the target is a reference.
    /// - `true`: Requires dereferencing (`*`) to assign a value to the target.
    /// - `false`: Requires a mutable borrow (`&mut`) to create a temporary
    ///   reference.
    pub is_ref: bool,
}

pub trait BsnTokenStream: Parse {
    fn to_tokens(&self, ctx: &mut BsnCodegenCtx) -> TokenStream;
}

impl BsnTokenStream for BsnRoot {
    fn to_tokens(&self, ctx: &mut BsnCodegenCtx) -> TokenStream {
        let tokens = self.0.to_tokens(ctx);
        let errors = ctx.errors.iter().map(|e| e.to_compile_error());
        let bevy_scene = ctx.bevy_scene;

        // NOTE: Assigning the result to a variable first so that the LSP's
        // type inference can see assignments before it encounters
        // any compile errors. This keeps autocomplete working in broken states,
        // e.g. when typing the name of a field but no value yet.
        quote! {
            #bevy_scene::SceneScope({
                let _res = #tokens;
                #(#errors)*
                _res
            })
        }
    }
}

impl BsnTokenStream for BsnListRoot {
    fn to_tokens(&self, ctx: &mut BsnCodegenCtx) -> TokenStream {
        let tokens = self.0.to_tokens(ctx);
        let errors = ctx.errors.iter().map(|e| e.to_compile_error());
        let bevy_scene = ctx.bevy_scene;

        // NOTE: Assigning the result to a variable first so that the LSP's
        // type inference can see assignments before it encounters
        // any compile errors. This keeps autocomplete working in broken states,
        // e.g. when typing the name of a field but no value yet.
        quote! {
            {
                let _res = #bevy_scene::SceneListScope(#tokens);
                #(#errors)*
                _res
            }
        }
    }
}

impl<const ALLOW_FLAT: bool> Bsn<ALLOW_FLAT> {
    /// Converts to tokens and performs validation checks.
    /// Accumulates errors in [`BsnCodegenCtx`].
    pub fn try_to_tokens(&self, ctx: &mut BsnCodegenCtx) -> syn::Result<TokenStream> {
        let entries: Vec<_> = self
            .entries
            .iter()
            .map(|entry| {
                entry
                    .try_to_tokens(ctx)
                    .unwrap_or_else(|e| e.to_compile_error())
            })
            .collect();

        Ok(quote! { (#(#entries,)*) })
    }

    pub fn to_tokens(&self, ctx: &mut BsnCodegenCtx) -> TokenStream {
        self.try_to_tokens(ctx)
            .unwrap_or_else(|e| e.to_compile_error())
    }
}

impl BsnEntry {
    fn try_to_tokens(&self, ctx: &mut BsnCodegenCtx) -> syn::Result<TokenStream> {
        let (bevy_scene, bevy_ecs) = (ctx.bevy_scene, ctx.bevy_ecs);

        match self {
            BsnEntry::TemplatePatch(ty) => {
                let mut assigns = Vec::new();
                let target = PatchTarget {
                    path: &[Member::Named(Ident::new(
                        "value",
                        proc_macro2::Span::call_site(),
                    ))],
                    is_ref: true,
                };
                ty.to_patch_tokens(ctx, &mut assigns, true, target)?;
                let path = &ty.path;
                Ok(quote! {
                    <#path as #bevy_scene::PatchTemplate>::patch_template(move |value, _context| {
                        #(#assigns)*
                    })
                })
            }
            BsnEntry::FromTemplatePatch(ty) => {
                let mut assigns = Vec::new();
                let target = PatchTarget {
                    path: &[Member::Named(Ident::new(
                        "value",
                        proc_macro2::Span::call_site(),
                    ))],
                    is_ref: true,
                };
                ty.to_patch_tokens(ctx, &mut assigns, true, target)?;
                let path = &ty.path;
                Ok(quote! {
                    <#path as #bevy_scene::PatchFromTemplate>::patch(move |value, _context| {
                        #(#assigns)*
                    })
                })
            }
            BsnEntry::TemplateConst {
                type_path,
                const_ident,
            } => Ok(quote! {
                <#type_path as #bevy_scene::PatchTemplate>::patch_template(move |value, _context| {
                    *value = #type_path::#const_ident;
                })
            }),
            BsnEntry::SceneExpression(block) => Ok(quote! {{#block}}),
            BsnEntry::TemplateConstructor(BsnConstructor {
                type_path,
                function,
                args,
            }) => Ok(quote! {
                <#type_path as #bevy_scene::PatchTemplate>::patch_template(move |value, _context| {
                    *value = #type_path::#function(#args);
                })
            }),
            BsnEntry::FromTemplateConstructor(BsnConstructor {
                type_path,
                function,
                args,
            }) => Ok(quote! {
                <#type_path as #bevy_scene::PatchFromTemplate>::patch(move |value, _context| {
                    *value = <#type_path as #bevy_ecs::template::FromTemplate>::Template::#function(#args);
                })
            }),
            BsnEntry::RelatedSceneList(BsnRelatedSceneList {
                scene_list,
                relationship_path,
            }) => {
                let scenes = scene_list.0.to_tokens(ctx);
                Ok(quote! {
                    #bevy_scene::RelatedScenes::<<#relationship_path as #bevy_ecs::relationship::RelationshipTarget>
                        ::Relationship, _>::new(#scenes)
                })
            }
            BsnEntry::InheritedScene(s) => match s {
                BsnInheritedScene::Asset(lit) => Ok(quote! {
                    #bevy_scene::InheritSceneAsset::from(#lit)
                }),
                BsnInheritedScene::Fn { function, args } => Ok(quote! {
                    #bevy_scene::SceneScope(#function(#args))
                }),
            },
            BsnEntry::Name(ident) => {
                let (name, index) = (ident.to_string(), ctx.entity_refs.get(ident.to_string()));
                Ok(quote! {
                    #bevy_scene::NameEntityReference { name: #bevy_ecs::name::Name(#name.into()), index: #index }
                })
            }
            BsnEntry::NameExpression(expr) => Ok(quote! {
                <#bevy_ecs::name::Name as #bevy_scene::PatchFromTemplate>::patch(move |value, _context| {
                    *value = #bevy_ecs::Name({#expr}.into());
                })
            }),
        }
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
    /// Recursively generates token streams.
    fn to_patch_tokens(
        &self,
        ctx: &mut BsnCodegenCtx,
        assignments: &mut Vec<TokenStream>,
        is_root: bool,
        target: PatchTarget,
    ) -> syn::Result<()> {
        if !is_root {
            let (path, bevy_scene) = (&self.path, ctx.bevy_scene);
            assignments.push(quote! {#bevy_scene::macro_utils::touch_type::<#path>();});
        }

        if let Some(variant) = &self.enum_variant {
            self.push_enum_patch(ctx, variant, assignments, target)?;
        } else {
            self.push_struct_patch(ctx, assignments, target)?;
        }

        Ok(())
    }

    fn push_enum_patch(
        &self,
        ctx: &mut BsnCodegenCtx,
        variant: &Ident,
        assignments: &mut Vec<TokenStream>,
        target: PatchTarget,
    ) -> syn::Result<()> {
        let (bevy_scene, bevy_ecs, path) = (ctx.bevy_scene, ctx.bevy_ecs, &self.path);
        let variant_default = format_ident!("default_{}", variant.to_string().to_lowercase());
        let template_path = quote! { #bevy_scene::macro_utils::PathResolveHelper::<<#path as #bevy_ecs::template::FromTemplate>::Template> };

        let maybe_deref = target.is_ref.then(|| quote! {*});
        let maybe_borrow_mut = (!target.is_ref).then(|| quote! {&mut});
        let field_path = target.path;

        let (check_pattern, binding_pattern, field_updates) = match &self.fields {
            BsnFields::Named(fields) => {
                let mut seen = HashSet::with_capacity(fields.len());
                let mut names = Vec::new();
                let mut assigns = Vec::new();

                for field in fields {
                    let field_name = &field.name;
                    if !seen.insert(field_name.to_string()) {
                        ctx.errors.push(syn::Error::new_spanned(
                            field_name,
                            format!("Duplicate field `{}` found in BSN enum variant", field_name),
                        ));
                        continue;
                    }

                    names.push(field_name);

                    assigns.push(self.process_enum_field(ctx, field_name, field.value.as_ref())?);
                }

                (
                    quote! { #variant { .. } },
                    quote! { #variant { #(#names,)* .. } },
                    assigns,
                )
            }
            BsnFields::Tuple(fields) if fields.is_empty() => {
                (quote! { #variant }, quote! { #variant }, vec![])
            }
            BsnFields::Tuple(fields) => {
                let names: Vec<_> = (0..fields.len()).map(|i| format_ident!("t{}", i)).collect();
                let assigns = fields
                    .iter()
                    .enumerate()
                    .map(|(i, f)| self.process_enum_field(ctx, &names[i], Some(&f.value)))
                    .collect::<syn::Result<Vec<_>>>()?;

                (
                    quote! { #variant(..) },
                    quote! { #variant(#(#names,)* ..) },
                    assigns,
                )
            }
        };

        assignments.push(quote! {
            {
                let _node = #maybe_borrow_mut #(#field_path).*;
                if !matches!(_node, #template_path::#check_pattern) {
                    #maybe_deref _node = #template_path::#variant_default();
                }
                if let #template_path::#binding_pattern = _node {
                    #(#field_updates)*
                }
            }
        });
        Ok(())
    }

    fn push_struct_patch(
        &self,
        ctx: &mut BsnCodegenCtx,
        assignments: &mut Vec<TokenStream>,
        target: PatchTarget,
    ) -> syn::Result<()> {
        match &self.fields {
            BsnFields::Named(fields) => {
                let mut seen = HashSet::with_capacity(fields.len());

                for field in fields {
                    let field_name = &field.name;
                    if !seen.insert(field_name.to_string()) {
                        ctx.errors.push(syn::Error::new_spanned(
                            field_name,
                            format!("Duplicate field `{}` found in BSN struct", field_name),
                        ));
                        continue;
                    }

                    if field.value.is_none() {
                        ctx.errors.push(syn::Error::new_spanned(
                            field_name,
                            format!("Field `{}` is missing a value.", field_name),
                        ));
                    }

                    self.process_field(
                        ctx,
                        assignments,
                        target.path,
                        Member::Named(field_name.clone()),
                        field.value.as_ref(),
                    )?;
                }
            }
            BsnFields::Tuple(fields) => {
                for (i, field) in fields.iter().enumerate() {
                    if let Err(err) = self.process_field(
                        ctx,
                        assignments,
                        target.path,
                        Member::Unnamed(Index::from(i)),
                        Some(&field.value),
                    ) {
                        ctx.errors.push(err);
                    }
                }
            }
        }
        Ok(())
    }

    fn process_field(
        &self,
        ctx: &mut BsnCodegenCtx,
        assignments: &mut Vec<TokenStream>,
        base_path: &[Member],
        member: Member,
        value: Option<&BsnValue>,
    ) -> syn::Result<()> {
        match value {
            // Enables field autocomplete in Rust Analyzer
            Some(field_value_type!()) | None => {
                // NOTE: It is very important to still produce outputs for None field values. This is what
                // enables field autocomplete in Rust Analyzer
                assignments.push(
                    value
                        .map(|v| quote! { #(#base_path.)*#member = #v; })
                        .unwrap_or(quote! {
                            #(#base_path.)*#member;
                        }),
                );
            }

            Some(BsnValue::Name(ident)) => {
                let index = ctx.entity_refs.get(ident.to_string());
                let bevy_ecs = ctx.bevy_ecs;
                assignments.push(quote! {
                    #(#base_path.)*#member = #bevy_ecs::template::EntityReference::ScopedEntityIndex(
                        #bevy_ecs::template::ScopedEntityIndex {
                            scope: _context.current_entity_scope(), index: #index
                        }
                    );
                });
            }
            Some(BsnValue::Type(ty)) if ty.enum_variant.is_some() => {
                assignments.push(quote! {#(#base_path.)*#member = #ty;});
            }
            Some(BsnValue::Type(ty)) => {
                let mut new_path = base_path.to_vec();
                new_path.push(member);
                ty.to_patch_tokens(
                    ctx,
                    assignments,
                    false,
                    PatchTarget {
                        path: &new_path,
                        is_ref: false,
                    },
                )?;
            }
        }
        Ok(())
    }

    fn process_enum_field(
        &self,
        ctx: &mut BsnCodegenCtx,
        bind_name: &Ident,
        value: Option<&BsnValue>,
    ) -> syn::Result<TokenStream> {
        if value.is_none() {
            ctx.errors.push(syn::Error::new_spanned(
                bind_name,
                format!("Enum field `{}` is missing a value", bind_name),
            ));
        }

        if let Some(BsnValue::Type(ty)) = value
            && ty.enum_variant.is_none()
        {
            let mut type_assigns = Vec::new();
            ty.to_patch_tokens(
                ctx,
                &mut type_assigns,
                false,
                PatchTarget {
                    path: &[Member::Named(bind_name.clone())],
                    is_ref: true,
                },
            )?;
            return Ok(quote! {#(#type_assigns)*});
        }

        // NOTE: It is very important to still produce outputs for None field values. This is what
        // enables field autocomplete in Rust Analyzer
        value
            .map(|v| Ok(quote! { *#bind_name = #v; }))
            .unwrap_or(Ok(quote! { #bind_name; }))
    }
}

impl BsnTokenStream for BsnSceneListItems {
    fn to_tokens(&self, ctx: &mut BsnCodegenCtx) -> TokenStream {
        let bevy_scene = ctx.bevy_scene;
        let scenes = self.0.iter().map(|s| match s {
            BsnSceneListItem::Scene(bsn) => {
                let tokens = bsn.to_tokens(ctx);
                quote! {#bevy_scene::EntityScene(#tokens)}
            }
            BsnSceneListItem::Expression(stmts) => quote! {#(#stmts)*},
        });

        quote! { #bevy_scene::auto_nest_tuple!(#(#scenes),*) }
    }
}

impl ToTokens for BsnType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let (path, variant) = (
            &self.path,
            self.enum_variant.as_ref().map(|v| quote! {::#v}),
        );
        match &self.fields {
            BsnFields::Named(fields) => {
                let assigns = fields.iter().map(|f| {
                    let (name, value) = (&f.name, &f.value);
                    quote! {#name: #value}
                });
                quote! { #path #variant { #(#assigns,)* } }
            }
            BsnFields::Tuple(fields) => {
                let assigns = fields.iter().map(|f| &f.value);
                quote! { #path #variant ( #(#assigns,)* ) }
            }
        }
        .to_tokens(tokens);
    }
}

impl ToTokens for BsnValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            BsnValue::Expr(e) => quote! {{#e}.into()}.to_tokens(tokens),
            BsnValue::Closure(c) => quote! {(#c).into()}.to_tokens(tokens),
            BsnValue::Ident(i) => quote! {(#i).into()}.to_tokens(tokens),
            BsnValue::Lit(Lit::Str(s)) => quote! {#s.into()}.to_tokens(tokens),
            BsnValue::Lit(l) => l.to_tokens(tokens),
            BsnValue::Tuple(t) => {
                let inner = t.0.iter();
                quote! {(#(#inner),*)}.to_tokens(tokens);
            }
            BsnValue::Type(ty) => ty.to_tokens(tokens),
            BsnValue::Name(_) => {
                // Name requires additional context to convert to tokens
                unreachable!()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bsn::types::*;
    use syn::parse_quote;

    struct TestPaths {
        bevy_scene: Path,
        bevy_ecs: Path,
    }

    impl TestPaths {
        fn new() -> Self {
            Self {
                bevy_scene: parse_quote!(bevy_scene),
                bevy_ecs: parse_quote!(bevy_ecs),
            }
        }

        fn ctx<'a>(&'a self, refs: &'a mut EntityRefs) -> BsnCodegenCtx<'a> {
            BsnCodegenCtx {
                bevy_scene: &self.bevy_scene,
                bevy_ecs: &self.bevy_ecs,
                entity_refs: refs,
                errors: Vec::new(),
            }
        }
    }

    #[test]
    fn duplicate_field() {
        let mut refs = EntityRefs::default();
        let paths = TestPaths::new();
        let mut ctx = paths.ctx(&mut refs);
        let mut assignments = vec![];
        let duplicate = BsnType {
            path: parse_quote!(Transform),
            enum_variant: None,
            fields: BsnFields::Named(vec![
                BsnNamedField {
                    name: parse_quote!(x),
                    value: Some(BsnValue::Expr(quote!({}))),
                },
                BsnNamedField {
                    name: parse_quote!(x),
                    value: Some(BsnValue::Expr(quote!({}))),
                },
            ]),
        };

        let res = duplicate.push_struct_patch(
            &mut ctx,
            &mut assignments,
            PatchTarget {
                path: &[],
                is_ref: false,
            },
        );

        assert!(res.is_ok());
        assert_eq!(ctx.errors.len(), 1);
        assert!(ctx.errors[0]
            .to_string()
            .contains("Duplicate field `x` found in BSN struct"));
    }

    #[test]
    fn recursive_duplicate_field() {
        let mut refs = EntityRefs::default();
        let paths = TestPaths::new();
        let mut ctx = paths.ctx(&mut refs);
        let mut assignments = vec![];
        let nested_duplicate = BsnType {
            path: parse_quote!(Parent),
            enum_variant: None,
            fields: BsnFields::Named(vec![BsnNamedField {
                name: parse_quote!(child_field),
                value: Some(BsnValue::Type(BsnType {
                    path: parse_quote!(Child),
                    enum_variant: None,
                    fields: BsnFields::Named(vec![
                        BsnNamedField {
                            name: parse_quote!(x),
                            value: Some(BsnValue::Expr(quote!({}))),
                        },
                        BsnNamedField {
                            name: parse_quote!(x),
                            value: Some(BsnValue::Expr(quote!({}))),
                        },
                    ]),
                })),
            }]),
        };

        let res = nested_duplicate.to_patch_tokens(
            &mut ctx,
            &mut assignments,
            true,
            PatchTarget {
                path: &[],
                is_ref: false,
            },
        );

        assert!(res.is_ok());
        assert_eq!(ctx.errors.len(), 1);
        assert!(ctx.errors[0]
            .to_string()
            .contains("Duplicate field `x` found in BSN struct"));
    }

    #[test]
    fn missing_struct_field() {
        let mut refs = EntityRefs::default();
        let paths = TestPaths::new();
        let mut ctx = paths.ctx(&mut refs);
        let mut assignments = Vec::new();
        let missing = BsnType {
            path: parse_quote!(Transform),
            enum_variant: None,
            fields: BsnFields::Named(vec![BsnNamedField {
                name: parse_quote!(x),
                value: None,
            }]),
        };

        let res = missing.push_struct_patch(
            &mut ctx,
            &mut assignments,
            PatchTarget {
                path: &[Member::Named(parse_quote!(value))],
                is_ref: false,
            },
        );

        assert!(res.is_ok());
        assert_eq!(ctx.errors.len(), 1);
        assert!(ctx.errors[0]
            .to_string()
            .contains("Field `x` is missing a value"));
    }

    #[test]
    fn enum_duplicate_field() {
        // Arrange
        let mut refs = EntityRefs::default();
        let paths = TestPaths::new();
        let mut ctx = paths.ctx(&mut refs);
        let mut assignments = vec![];
        let duplicate = BsnType {
            path: parse_quote!(MyEnum),
            enum_variant: Some(parse_quote!(Variant)),
            fields: BsnFields::Named(vec![
                BsnNamedField {
                    name: parse_quote!(x),
                    value: Some(BsnValue::Expr(quote!(1))),
                },
                BsnNamedField {
                    name: parse_quote!(x),
                    value: Some(BsnValue::Expr(quote!(2))),
                },
            ]),
        };

        // Act
        let res = duplicate.push_enum_patch(
            &mut ctx,
            &parse_quote!(Variant),
            &mut assignments,
            PatchTarget {
                path: &[],
                is_ref: false,
            },
        );

        // Assert
        assert!(res.is_ok());
        assert_eq!(ctx.errors.len(), 1);
        assert!(ctx.errors[0]
            .to_string()
            .contains("Duplicate field `x` found in BSN enum variant"));
    }

    #[test]
    fn bsn_root_preserves_inference_on_error() {
        // Arrange
        let expected = "bevy_scene :: SceneScope ({ let _res = () ;".to_string()
            + " :: core :: compile_error ! { \"Test Error\" }"
            + " _res })";

        let mut refs = EntityRefs::default();
        let paths = TestPaths::new();
        let mut ctx = paths.ctx(&mut refs);
        ctx.errors.push(syn::Error::new(
            proc_macro2::Span::call_site(),
            "Test Error",
        ));
        let root = BsnRoot(Bsn::<true> { entries: vec![] });

        // Act
        let res = root.to_tokens(&mut ctx).to_string();

        // Assert
        assert_eq!(res, expected,);
    }

    #[test]
    fn bsn_list_root_preserves_inference_on_error() {
        // Arrange
        let expected =
            "{ let _res = bevy_scene :: SceneListScope (bevy_scene :: auto_nest_tuple ! ()) ;"
                .to_string()
                + " :: core :: compile_error ! { \"Test Error\" }"
                + " _res }";

        let mut refs = EntityRefs::default();
        let paths = TestPaths::new();
        let mut ctx = paths.ctx(&mut refs);
        ctx.errors.push(syn::Error::new(
            proc_macro2::Span::call_site(),
            "Test Error",
        ));
        let root = BsnListRoot(BsnSceneListItems(vec![]));

        // Act
        let res = root.to_tokens(&mut ctx).to_string();

        // Assert
        assert_eq!(res, expected,);
    }
}
