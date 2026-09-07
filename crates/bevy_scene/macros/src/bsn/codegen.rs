use crate::_bsn::types::{
    Bsn, BsnConstructor, BsnEntry, BsnFields, BsnFnArg, BsnFnArgs, BsnFnCall, BsnListRoot,
    BsnNamedField, BsnRelatedSceneList, BsnRoot, BsnScene, BsnSceneFn, BsnSceneListItem,
    BsnSceneListItems, BsnStructUpdate, BsnType, BsnUnnamedField, BsnValue,
};
use bevy_macro_utils::{fq_std::FQDefault, path_to_string};
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use std::collections::{hash_map::Entry, HashMap, HashSet};
use syn::{parse::Parse, ExprTuple, Ident, Lit, Member, Path};

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

#[derive(Default)]
pub(crate) struct HoistedExpressions {
    expressions: Vec<TokenStream>,
    next: usize,
}

impl HoistedExpressions {
    fn next_ident(&mut self) -> Ident {
        let index = self.next;
        let ident = format_ident!("_expr{index}");
        self.next += 1;
        ident
    }

    pub fn hoist(&mut self, value: &BsnValue) -> Ident {
        let ident = self.next_ident();
        self.expressions.push(quote! {let #ident = #value;});
        ident
    }
}

/// Context used in the [`Bsn`] code generation pipeline.
/// Used to accumulate validation errors without short-circuiting.
pub(crate) struct BsnCodegenCtx<'a> {
    pub bevy_scene: &'a Path,
    pub bevy_ecs: &'a Path,
    pub invocation_index: ExprTuple,
    pub entity_refs: &'a mut EntityRefs,
    pub hoisted_expressions: &'a mut HoistedExpressions,
    /// Accumulated parsing and validation errors.
    pub errors: Vec<syn::Error>,
}
impl<'a> BsnCodegenCtx<'a> {
    fn fixed_entity_ref(&mut self, ident: &Ident) -> (String, usize) {
        let string = ident.to_string();
        (ident.to_string(), self.entity_refs.get(string))
    }
}

pub trait BsnTokenStream: Parse {
    fn to_tokens(&self, ctx: &mut BsnCodegenCtx) -> TokenStream;
}

impl BsnTokenStream for BsnRoot {
    fn to_tokens(&self, ctx: &mut BsnCodegenCtx) -> TokenStream {
        let tokens = self.0.to_tokens(ctx);
        let errors = ctx.errors.iter().map(|e| e.to_compile_error());
        let bevy_scene = ctx.bevy_scene;
        let hoisted_exprs = ctx.hoisted_expressions.expressions.drain(..);
        let call_id = if !ctx.entity_refs.refs.is_empty() {
            quote! {
                static _CALL_ID: #bevy_scene::macro_utils::CallCounter = #bevy_scene::macro_utils::CallCounter::new();
                let _call_id = _CALL_ID.increment();
            }
        } else {
            quote! {}
        };

        // NOTE: Assigning the result to a variable first so that the LSP's
        // type inference can see assignments before it encounters
        // any compile errors. This keeps autocomplete working in broken states,
        // e.g. when typing the name of a field but no value yet.
        quote! {
            #bevy_scene::SceneScope({
                #call_id
                #(#hoisted_exprs)*
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
        let hoisted_exprs = ctx.hoisted_expressions.expressions.drain(..);
        let call_id = if !ctx.entity_refs.refs.is_empty() {
            quote! {
                static _CALL_ID: #bevy_scene::macro_utils::CallCounter = #bevy_scene::macro_utils::CallCounter::new();
                let _call_id = _CALL_ID.increment();
            }
        } else {
            quote! {}
        };

        // NOTE: Assigning the result to a variable first so that the LSP's
        // type inference can see assignments before it encounters
        // any compile errors. This keeps autocomplete working in broken states,
        // e.g. when typing the name of a field but no value yet.
        quote! {
            {
                #call_id
                #(#hoisted_exprs)*
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
        let bevy_scene = ctx.bevy_scene;
        let mut combined_patches = Vec::new();
        let mut scene_impls = Vec::new();
        for entry in &self.entries {
            match entry.try_to_tokens(ctx) {
                Ok(EntryResult::CombinedSceneFunction(patch)) => combined_patches.push(patch),
                Ok(EntryResult::NewSceneImpl(scene_impl)) => {
                    if !combined_patches.is_empty() {
                        let patches = combined_patches.drain(..);
                        scene_impls.push(quote! {
                            #bevy_scene::SceneFunction(move |_context, _scene| {
                                #(#patches)*
                            })
                        });
                    }
                    scene_impls.push(scene_impl)
                }
                Err(err) => scene_impls.push(err.to_compile_error()),
            }
        }
        if !combined_patches.is_empty() {
            let patches = combined_patches.drain(..);
            scene_impls.push(quote! {
                #bevy_scene::SceneFunction(move |_context, _scene| {
                    #(#patches)*
                })
            });
        }
        Ok(quote! { #bevy_scene::auto_nest_tuple!(#(#scene_impls),*) })
    }

    pub fn to_tokens(&self, ctx: &mut BsnCodegenCtx) -> TokenStream {
        self.try_to_tokens(ctx)
            .unwrap_or_else(|e| e.to_compile_error())
    }
}

enum EntryResult {
    CombinedSceneFunction(TokenStream),
    NewSceneImpl(TokenStream),
}

impl BsnEntry {
    fn try_to_tokens(&self, ctx: &mut BsnCodegenCtx) -> syn::Result<EntryResult> {
        let (bevy_scene, bevy_ecs) = (ctx.bevy_scene, ctx.bevy_ecs);

        Ok(match self {
            BsnEntry::TemplatePatch(ty) => {
                if ty.variant.is_some() {
                    let template = ty.enum_tokens(ctx, false)?;
                    EntryResult::CombinedSceneFunction(quote! {
                        _scene.insert_template(#template);
                    })
                } else {
                    let path = &[Member::Named(Ident::new(
                        "__value",
                        proc_macro2::Span::call_site(),
                    ))];
                    let assigns = ty.patch_tokens(ctx, path, true, false, false)?;
                    let path = &ty.path;
                    EntryResult::CombinedSceneFunction(if assigns.is_empty() {
                        quote! {
                            let _ = _scene.get_or_insert_template::<#path>(_context);
                        }
                    } else {
                        quote! {
                            let __value = _scene.get_or_insert_template::<#path>(_context);
                            #(#assigns)*
                        }
                    })
                }
            }
            BsnEntry::FromTemplatePatch(ty) => {
                if ty.variant.is_some() {
                    let template = ty.enum_tokens(ctx, true)?;
                    EntryResult::CombinedSceneFunction(quote! {
                        _scene.insert_template(#template);
                    })
                } else {
                    let path = &[Member::Named(Ident::new(
                        "__value",
                        proc_macro2::Span::call_site(),
                    ))];
                    let assigns = ty.patch_tokens(ctx, path, true, false, false)?;
                    let path = &ty.path;
                    EntryResult::CombinedSceneFunction(if assigns.is_empty() {
                        quote! {
                            let _ = _scene.get_or_insert_template::<<#path as #bevy_ecs::template::FromTemplate>::Template>(_context);
                        }
                    } else {
                        quote! {
                            let __value = _scene.get_or_insert_template::<<#path as #bevy_ecs::template::FromTemplate>::Template>(_context);
                            #(#assigns)*
                        }
                    })
                }
            }
            BsnEntry::TemplateConstructor {
                constructor:
                    BsnConstructor {
                        type_path,
                        function,
                        args,
                    },
                dot_expression,
            } => EntryResult::CombinedSceneFunction({
                let args = args.to_tokens(ctx);
                if let Some(dot_expr) = dot_expression {
                    quote! {
                        _scene.insert_template::<#type_path>(#type_path::#function #args #dot_expr);
                    }
                } else {
                    quote! {
                        _scene.insert_template::<#type_path>(#type_path::#function #args);
                    }
                }
            }),
            BsnEntry::FromTemplateConstructor {
                constructor:
                    BsnConstructor {
                        type_path,
                        function,
                        args,
                    },
                dot_expression,
            } => EntryResult::CombinedSceneFunction({
                let args = args.to_tokens(ctx);
                if let Some(dot_expr) = dot_expression {
                    quote! {
                        _scene.insert_template(<#type_path as #bevy_ecs::template::FromTemplate>::Template::#function #args #dot_expr);
                    }
                } else {
                    quote! {
                        _scene.insert_template(<#type_path as #bevy_ecs::template::FromTemplate>::Template::#function #args);
                    }
                }
            }),
            BsnEntry::RelatedSceneList(BsnRelatedSceneList {
                scene_list,
                relationship_path,
            }) => {
                let scenes = scene_list.0.to_tokens(ctx);
                EntryResult::NewSceneImpl(quote! {
                    #bevy_scene::RelatedScenes::<<#relationship_path as #bevy_ecs::relationship::RelationshipTarget>
                    ::Relationship, _>::new(#scenes)
                })
            }
            BsnEntry::UncachedScene(s) => EntryResult::NewSceneImpl(s.to_tokens(ctx)?),
            BsnEntry::CachedScene(s) => EntryResult::NewSceneImpl(s.to_tokens(ctx)?),
            BsnEntry::Name(ident) => {
                let (name, index) = ctx.fixed_entity_ref(ident);
                let invocation = ctx.invocation_index.clone();
                EntryResult::CombinedSceneFunction(quote! {
                    #bevy_scene::NameEntityReference { name: #bevy_ecs::name::Name(#name.into()), reference: #bevy_ecs::template::SceneEntityReference::new(#invocation, #index, _call_id,) }.resolve_inline(_context, _scene);
                })
            }
            BsnEntry::TemplateValue(token_stream) => EntryResult::CombinedSceneFunction(quote! {
                _scene.insert_template(#token_stream);
            }),
            BsnEntry::Function(BsnFnCall { args, path }) => {
                let args = args.to_tokens(ctx);
                EntryResult::CombinedSceneFunction(quote! {
                    _scene.insert_template(#path #args);
                })
            }
        })
    }
}

impl BsnScene {
    fn to_tokens(&self, ctx: &mut BsnCodegenCtx) -> syn::Result<TokenStream> {
        let bevy_scene = ctx.bevy_scene;
        match self {
            BsnScene::Asset(lit) => Ok(quote! {
                #bevy_scene::CachedSceneAsset::from(#lit)
            }),
            BsnScene::Fn(func) => Ok(func.to_tokens(ctx)),
            BsnScene::SceneComponent(bsn_type) => {
                let props = format_ident!("__props");
                let props_ref = format_ident!("__props_ref");
                let props_path = &[Member::Named(props_ref.clone())];
                let props_assignments =
                    bsn_type.patch_tokens(ctx, props_path, false, true, true)?;
                let path = &bsn_type.path;
                let template_patch = if bsn_type.variant.is_some() {
                    let enum_tokens = bsn_type.enum_tokens(ctx, true)?;
                    let bevy_scene = ctx.bevy_scene;
                    quote! {
                        <#path as #bevy_scene::PatchFromTemplate>::patch(move |__value, _context| {
                            *__value = #enum_tokens;
                        })
                    }
                } else {
                    let value_path = &[Member::Named(Ident::new(
                        "__value",
                        proc_macro2::Span::call_site(),
                    ))];
                    let template_assignments =
                        bsn_type.patch_tokens(ctx, value_path, true, false, true)?;
                    let bevy_scene = ctx.bevy_scene;
                    quote! {
                        <#path as #bevy_scene::PatchFromTemplate>::patch(move |__value, _context| {
                            #(#template_assignments)*
                        })
                    }
                };
                Ok(quote! {{
                    let mut #props = <<#path as #bevy_scene::SceneComponent>::Props as #FQDefault>::default();
                    let #props_ref = &mut #props;
                    #(#props_assignments)*
                    (<#path as #bevy_scene::SceneComponent>::scene(#props), #template_patch)
                }})
            }
            BsnScene::Expression(tokens) => Ok(quote! {
                #tokens
            }),
        }
    }
}

impl BsnType {
    fn patch_tokens(
        &self,
        ctx: &mut BsnCodegenCtx,
        path: &[Member],
        is_root: bool,
        is_props: bool,
        is_scene_component: bool,
    ) -> syn::Result<Vec<TokenStream>> {
        let mut assignments = Vec::new();
        if self.variant.is_some() {
            if is_props {
                assignments.extend(self.struct_patch_tokens(
                    ctx,
                    path,
                    is_root,
                    true,
                    is_scene_component,
                )?);
            } else {
                let value = self.enum_tokens(ctx, is_root)?;

                assignments.push(quote! {
                    #(#path).* = #value.into();
                });
            }
        } else {
            assignments.extend(self.struct_patch_tokens(
                ctx,
                path,
                is_root,
                is_props,
                is_scene_component,
            )?);
        }

        Ok(assignments)
    }

    fn init_tokens(&self, ctx: &mut BsnCodegenCtx, is_root: bool) -> syn::Result<TokenStream> {
        if self.variant.is_some() {
            self.enum_tokens(ctx, is_root)
        } else {
            self.struct_init_tokens(ctx, is_root)
        }
    }

    fn enum_tokens(
        &self,
        ctx: &mut BsnCodegenCtx,
        is_root_template: bool,
    ) -> syn::Result<TokenStream> {
        let variant = self.variant.as_ref().unwrap();
        let (bevy_scene, bevy_ecs, path) = (ctx.bevy_scene, ctx.bevy_ecs, &self.path);
        let template_path = if is_root_template {
            quote! { #bevy_scene::macro_utils::PathResolveHelper::<<#path as #bevy_ecs::template::FromTemplate>::Template> }
        } else {
            quote! { #path }
        };

        Ok(match &self.fields {
            BsnFields::Named {
                fields,
                struct_update,
            } => {
                if struct_update.is_some() {
                    ctx.errors.push(syn::Error::new_spanned(
                        variant,
                        "Struct update syntax is not supported in enums",
                    ));
                }
                let mut seen = HashSet::with_capacity(fields.len());
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

                    assigns.push(field.to_init_tokens(ctx)?);
                }

                quote! {
                    #template_path::#variant {
                      #(#assigns)*
                    }
                }
            }
            BsnFields::Tuple(fields) => {
                let mut assigns = Vec::new();
                for field in fields {
                    assigns.push(field.to_init_tokens(ctx)?);
                }

                quote! {
                    #template_path::#variant(
                      #(#assigns)*
                    )
                }
            }
            BsnFields::Unit => {
                quote! {
                    #template_path::#variant
                }
            }
        })
    }

    fn struct_patch_tokens(
        &self,
        ctx: &mut BsnCodegenCtx,
        path: &[Member],
        is_root: bool,
        is_props: bool,
        is_scene_component: bool,
    ) -> syn::Result<Vec<TokenStream>> {
        let mut assignments = Vec::new();
        if !is_root {
            let (path, bevy_scene) = (&self.path, ctx.bevy_scene);
            assignments.push(quote! {#bevy_scene::macro_utils::touch_type::<#path>();});
        }
        match &self.fields {
            BsnFields::Named {
                fields,
                struct_update,
            } => {
                if let Some(struct_update) = struct_update {
                    let tokens = (*struct_update.value).to_tokens(ctx)?;
                    if path.len() == 1 {
                        assignments.push(quote! { *#(#path)* = #tokens; });
                    } else {
                        assignments.push(quote! { #(#path).* = #tokens; });
                    }
                }

                let mut seen = HashSet::with_capacity(fields.len());
                for field in fields {
                    let field_name = &field.name;
                    if is_props != field.is_prop {
                        if !is_scene_component && field.is_prop {
                            let type_path = &self.path;
                            ctx.errors.push(syn::Error::new_spanned(
                                field_name,
                                format!(
                                    "Scene prop fields are not supported in normal component patches\
                                     . If you would like to set a component scene's prop field, it \
                                     should be set using \"scene component\" syntax: \
                                     bsn! {{ @{} {{ @{field_name}: VALUE }} }}",
                                     path_to_string(type_path)
                                ),
                            ));
                        }
                        continue;
                    }
                    if !seen.insert(field_name.to_string()) {
                        ctx.errors.push(syn::Error::new_spanned(
                            field_name,
                            format!("Duplicate field `{}` found in BSN struct", field_name),
                        ));
                        continue;
                    }

                    let new_path = if field.is_prop {
                        &[Member::Named(format_ident!("__props"))]
                    } else {
                        path
                    };

                    match field.to_patch_tokens(ctx, new_path) {
                        Ok(tokens) => assignments.push(tokens),
                        Err(err) => ctx.errors.push(err),
                    }
                }
            }
            BsnFields::Tuple(fields) => {
                // Tuple fields can't be props
                if is_props {
                    return Ok(Vec::new());
                }
                for field in fields.iter() {
                    match field.to_patch_tokens(ctx, path) {
                        Ok(tokens) => assignments.push(tokens),
                        Err(err) => ctx.errors.push(err),
                    }
                }
            }
            BsnFields::Unit => {}
        }
        Ok(assignments)
    }
    fn struct_init_tokens(
        &self,
        ctx: &mut BsnCodegenCtx,
        is_root: bool,
    ) -> syn::Result<TokenStream> {
        let (bevy_scene, bevy_ecs, path) = (ctx.bevy_scene, ctx.bevy_ecs, &self.path);
        let template_path = if is_root {
            quote! { #bevy_scene::macro_utils::PathResolveHelper::<<#path as #bevy_ecs::template::FromTemplate>::Template> }
        } else {
            quote! { #path }
        };

        Ok(match &self.fields {
            BsnFields::Named {
                fields,
                struct_update,
            } => {
                let mut seen = HashSet::with_capacity(fields.len());
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

                    assigns.push(field.to_init_tokens(ctx)?);
                }

                let struct_update = struct_update
                    .as_ref()
                    .map(|struct_update| quote! { #struct_update })
                    .unwrap_or_else(|| quote! {..#FQDefault::default()});
                quote! {
                    #template_path {
                      #(#assigns)*
                      #struct_update
                    }
                }
            }
            BsnFields::Tuple(fields) => {
                let mut assigns = Vec::new();
                for field in fields {
                    assigns.push(field.to_init_tokens(ctx)?);
                }

                quote! {
                    #template_path(
                      #(#assigns,)*
                    )
                }
            }
            BsnFields::Unit => {
                quote! {
                    #template_path
                }
            }
        })
    }
}

impl BsnNamedField {
    fn to_init_tokens(&self, ctx: &mut BsnCodegenCtx) -> syn::Result<TokenStream> {
        let name = &self.name;
        Ok(match &self.value {
            Some(value) => {
                let tokens = value.to_tokens(ctx)?;
                quote! { #name: #tokens, }
            }
            None => {
                if self.is_name_shorthand {
                    quote! { #name: #name.into(), }
                } else {
                    ctx.errors.push(syn::Error::new_spanned(
                        name,
                        format!("Field `{}` is missing a value", name),
                    ));
                    // NOTE: It is very important to still produce outputs for None field values. This is what
                    // enables field autocomplete in Rust Analyzer
                    quote! { #name, }
                }
            }
        })
    }

    fn to_patch_tokens(
        &self,
        ctx: &mut BsnCodegenCtx,
        base_path: &[Member],
    ) -> syn::Result<TokenStream> {
        let name = &self.name;
        Ok(match &self.value {
            Some(value) => {
                if let BsnValue::Type(bsn_type) = value {
                    let mut new_path = base_path.to_vec();
                    new_path.push(Member::Named(name.clone()));
                    let assignments = bsn_type.patch_tokens(ctx, &new_path, false, false, false)?;
                    quote! { #(#assignments)* }
                } else {
                    let tokens = value.to_tokens(ctx)?;
                    quote! { #(#base_path.)*#name = #tokens; }
                }
            }
            None => {
                if self.is_name_shorthand {
                    quote! { #(#base_path.)*#name = #name.into(); }
                } else {
                    ctx.errors.push(syn::Error::new_spanned(
                        name,
                        format!("Field `{}` is missing a value", name),
                    ));
                    // NOTE: It is very important to still produce outputs for None field values. This is what
                    // enables field autocomplete in Rust Analyzer
                    quote! { #(#base_path.)*#name; }
                }
            }
        })
    }
}

impl BsnUnnamedField {
    fn to_patch_tokens(
        &self,
        ctx: &mut BsnCodegenCtx,
        base_path: &[Member],
    ) -> syn::Result<TokenStream> {
        let index = &self.index;
        let value = &self.value;
        if let BsnValue::Type(bsn_type) = value {
            let mut new_path = base_path.to_vec();
            new_path.push(index.clone());
            let patches = bsn_type.patch_tokens(ctx, &new_path, false, false, false)?;
            Ok(quote! {#(#patches)*})
        } else {
            let tokens = value.to_tokens(ctx)?;
            Ok(quote! { #(#base_path.)*#index = #tokens; })
        }
    }

    fn to_init_tokens(&self, ctx: &mut BsnCodegenCtx) -> syn::Result<TokenStream> {
        let value = &self.value;
        let tokens = value.to_tokens(ctx)?;
        Ok(quote! { #tokens, })
    }
}

impl BsnValue {
    fn to_tokens(&self, ctx: &mut BsnCodegenCtx) -> syn::Result<TokenStream> {
        Ok(match self {
            value @ (BsnValue::Ident(_)
            | BsnValue::Expr(_)
            | BsnValue::Closure(_)
            | BsnValue::Tuple(_)) => {
                let ident = ctx.hoisted_expressions.hoist(value);
                ident.to_token_stream()
            }
            BsnValue::Type(ty) => ty.init_tokens(ctx, false)?,
            BsnValue::Name(ident) => {
                let index = ctx.entity_refs.get(ident.to_string());
                let bevy_ecs = ctx.bevy_ecs;
                let invocation = ctx.invocation_index.clone();
                quote! {
                    #bevy_ecs::template::EntityTemplate::from_reference(#invocation, #index,  _call_id)
                }
            }
            BsnValue::Range {
                start,
                end,
                inclusive,
            } => {
                let start = (**start).to_tokens(ctx)?;
                let end = (**end).to_tokens(ctx)?;
                if *inclusive {
                    quote! {#start..=#end}
                } else {
                    quote! {#start..#end}
                }
            }
            value => value.to_token_stream(),
        })
    }
}

impl ToTokens for BsnStructUpdate {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let value = &self.value;
        quote! { ..#value }.to_tokens(tokens);
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
            BsnSceneListItem::Expression(tokens) => tokens.clone(),
        });

        quote! { #bevy_scene::auto_nest_tuple!(#(#scenes),*) }
    }
}

impl BsnSceneFn {
    fn to_tokens(&self, ctx: &mut BsnCodegenCtx) -> TokenStream {
        let bevy_scene = ctx.bevy_scene;
        let args = self.args.to_tokens(ctx);
        let path = &self.path;
        quote! {#bevy_scene::SceneScope(#path #args)}
    }
}

impl BsnTokenStream for BsnFnArgs {
    fn to_tokens(&self, ctx: &mut BsnCodegenCtx) -> TokenStream {
        let args = self.0.iter().map(|a| a.to_tokens(ctx));
        quote! { (#(#args),*) }
    }
}

impl BsnTokenStream for BsnFnArg {
    fn to_tokens(&self, ctx: &mut BsnCodegenCtx) -> TokenStream {
        let bevy_ecs = ctx.bevy_ecs;
        match self {
            BsnFnArg::EntityName(ident) => {
                let index = ctx.entity_refs.get(ident.to_string());
                let invocation = ctx.invocation_index.clone();
                quote! {
                    #bevy_ecs::template::EntityTemplate::SceneEntityReference(
                        #bevy_ecs::template::SceneEntityReference::new(#invocation, #index, _call_id)
                    )
                }
            }
            BsnFnArg::Tokens(token_stream) => token_stream.clone(),
        }
    }
}
impl ToTokens for BsnValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            BsnValue::Expr(e) => quote! {{#e}.into()}.to_tokens(tokens),
            BsnValue::Closure(c) => quote! {(#c).into()}.to_tokens(tokens),
            BsnValue::Ident(i) => quote! {(#i).into()}.to_tokens(tokens),
            BsnValue::Lit(Lit::Str(s)) => quote! {#s.into()}.to_tokens(tokens),
            BsnValue::Lit(l) => {
                if l.suffix().is_empty() {
                    l.to_tokens(tokens)
                } else {
                    quote! {(#l).into()}.to_tokens(tokens)
                }
            }
            BsnValue::Tuple(t) => {
                let inner = t.0.iter();
                quote! {(#(#inner),*)}.to_tokens(tokens);
            }
            BsnValue::Range {
                start,
                end,
                inclusive,
            } => {
                let start = start.into_token_stream();
                let end = end.into_token_stream();
                if *inclusive {
                    quote! {#start..=#end}.to_tokens(tokens);
                } else {
                    quote! {#start..#end}.to_tokens(tokens);
                }
            }
            BsnValue::Type(_) | BsnValue::Name(_) => {
                // Name and Type require additional context to convert to tokens
                unreachable!();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::_bsn::types::*;
    use syn::parse_quote;

    struct TestPaths {
        bevy_scene: Path,
        bevy_ecs: Path,
    }

    fn named_fields(fields: Vec<BsnNamedField>) -> BsnFields {
        BsnFields::Named {
            fields,
            struct_update: None,
        }
    }

    fn named_field(name: Ident, value: BsnValue) -> BsnNamedField {
        BsnNamedField {
            is_name_shorthand: false,
            is_prop: false,
            name,
            value: Some(value),
        }
    }

    impl TestPaths {
        fn new() -> Self {
            Self {
                bevy_scene: parse_quote!(bevy_scene),
                bevy_ecs: parse_quote!(bevy_ecs),
            }
        }

        fn ctx<'a>(
            &'a self,
            refs: &'a mut EntityRefs,
            hoisted_expressions: &'a mut HoistedExpressions,
        ) -> BsnCodegenCtx<'a> {
            BsnCodegenCtx {
                bevy_scene: &self.bevy_scene,
                bevy_ecs: &self.bevy_ecs,
                entity_refs: refs,
                invocation_index: parse_quote!(("", 0, 0)),
                hoisted_expressions,
                errors: Vec::new(),
            }
        }
    }

    #[test]
    fn duplicate_field() {
        let mut refs = EntityRefs::default();
        let paths = TestPaths::new();
        let mut exprs = HoistedExpressions::default();
        let mut ctx = paths.ctx(&mut refs, &mut exprs);
        let duplicate = BsnType {
            path: parse_quote!(Transform),
            variant: None,
            fields: named_fields(vec![
                named_field(parse_quote!(x), BsnValue::Expr(quote!({}))),
                named_field(parse_quote!(x), BsnValue::Expr(quote!({}))),
            ]),
        };

        let res = duplicate.patch_tokens(&mut ctx, &[], false, false, false);

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
        let mut exprs = HoistedExpressions::default();
        let mut ctx = paths.ctx(&mut refs, &mut exprs);
        let nested_duplicate = BsnType {
            path: parse_quote!(Parent),
            variant: None,
            fields: named_fields(vec![named_field(
                parse_quote!(Child),
                BsnValue::Type(BsnType {
                    path: parse_quote!(Child),
                    variant: None,
                    fields: named_fields(vec![
                        named_field(parse_quote!(x), BsnValue::Expr(quote!({}))),
                        named_field(parse_quote!(x), BsnValue::Expr(quote!({}))),
                    ]),
                }),
            )]),
        };

        let res = nested_duplicate.patch_tokens(&mut ctx, &[], true, false, false);

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
        let mut exprs = HoistedExpressions::default();
        let mut ctx = paths.ctx(&mut refs, &mut exprs);
        let missing = BsnType {
            path: parse_quote!(Transform),
            variant: None,
            fields: named_fields(vec![BsnNamedField {
                is_prop: false,
                is_name_shorthand: false,
                name: parse_quote!(x),
                value: None,
            }]),
        };

        let res = missing.patch_tokens(
            &mut ctx,
            &[Member::Named(parse_quote!(value))],
            false,
            false,
            false,
        );

        assert!(res.is_ok());
        assert_eq!(ctx.errors.len(), 1);
        assert!(ctx.errors[0]
            .to_string()
            .contains("Field `x` is missing a value"));
    }

    #[test]
    fn enum_variant_field_values_use_implicit_into() {
        let mut refs = EntityRefs::default();
        let paths = TestPaths::new();
        let mut exprs = HoistedExpressions::default();
        let mut ctx = paths.ctx(&mut refs, &mut exprs);
        let font = BsnType {
            path: parse_quote!(TextFont),
            variant: None,
            fields: named_fields(vec![named_field(
                parse_quote!(font_size),
                BsnValue::Type(BsnType {
                    path: parse_quote!(TextSize),
                    variant: Some(parse_quote!(Large)),
                    fields: named_fields(Vec::new()),
                }),
            )]),
        };

        let res = font.patch_tokens(
            &mut ctx,
            &[Member::Named(parse_quote!(value))],
            true,
            false,
            false,
        );

        assert!(res.is_ok());
        assert!(ctx.errors.is_empty());
        assert_eq!(
            res.unwrap()[0].to_string(),
            "value . font_size = TextSize :: Large { } . into () ;"
        );
    }

    #[test]
    fn enum_duplicate_field() {
        // Arrange
        let mut refs = EntityRefs::default();
        let paths = TestPaths::new();
        let mut exprs = HoistedExpressions::default();
        let mut ctx = paths.ctx(&mut refs, &mut exprs);
        let duplicate = BsnType {
            path: parse_quote!(MyEnum),
            variant: Some(parse_quote!(Variant)),
            fields: named_fields(vec![
                named_field(parse_quote!(x), BsnValue::Expr(quote!(1))),
                named_field(parse_quote!(x), BsnValue::Expr(quote!(2))),
            ]),
        };

        // Act
        let res = duplicate.patch_tokens(&mut ctx, &[], true, false, false);

        // Assert
        assert!(res.is_ok());
        assert_eq!(ctx.errors.len(), 1);
        assert!(ctx.errors[0]
            .to_string()
            .contains("Duplicate field `x` found in BSN enum variant"));
    }

    #[test]
    fn enum_variant_expr_is_hoisted() {
        let mut refs = EntityRefs::default();
        let paths = TestPaths::new();
        let mut exprs = HoistedExpressions::default();
        let mut ctx = paths.ctx(&mut refs, &mut exprs);
        let handle = BsnType {
            path: parse_quote!(FontSourceTemplate),
            variant: Some(parse_quote!(Handle)),
            fields: BsnFields::Tuple(vec![BsnUnnamedField {
                value: BsnValue::Expr(quote!(some_borrow.clone())),
                index: Member::Unnamed(0.into()),
            }]),
        };

        let res = handle.patch_tokens(&mut ctx, &[], true, false, false);

        assert!(res.is_ok());
        assert_eq!(ctx.errors.len(), 0);
        assert_eq!(exprs.expressions.len(), 1);
        assert_eq!(
            exprs.expressions[0].to_string(),
            "let _expr0 = { some_borrow . clone () } . into () ;"
        );
        let assignment_output: String = res.unwrap().iter().map(|t| t.to_string()).collect();
        assert!(
            assignment_output.contains("_expr0"),
            "expected hoisted ident in assignment output: {assignment_output}"
        );
        assert!(
            !assignment_output.contains("some_borrow"),
            "borrow should not appear inline in assignment output: {assignment_output}"
        );
    }

    #[test]
    fn bsn_root_preserves_inference_on_error() {
        // Arrange
        let expected = "bevy_scene :: SceneScope ({ let _res = bevy_scene :: auto_nest_tuple \
            ! () ; :: core :: compile_error ! { \"Test Error\" } _res })";

        let mut refs = EntityRefs::default();
        let paths = TestPaths::new();
        let mut exprs = HoistedExpressions::default();
        let mut ctx = paths.ctx(&mut refs, &mut exprs);
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
        let mut exprs = HoistedExpressions::default();
        let mut ctx = paths.ctx(&mut refs, &mut exprs);
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
