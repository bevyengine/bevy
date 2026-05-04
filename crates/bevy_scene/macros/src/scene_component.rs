use bevy_ecs_macro_logic::component::DeriveComponent;
use bevy_macro_utils::{fq_std::FQDefault, BevyManifest, PathType};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    parse_str,
    token::Paren,
    DeriveInput, LitStr, Path,
};

pub(crate) fn derive_scene_component(ast: &mut DeriveInput) -> TokenStream {
    let mut derive_component = match DeriveComponent::parse(ast) {
        Ok(value) => value,
        Err(e) => return e.into_compile_error(),
    };

    let (bevy_ecs, bevy_scene) = BevyManifest::shared(|manifest| {
        (
            manifest.get_path("bevy_ecs"),
            manifest.get_path("bevy_scene"),
        )
    });

    let scene = match parse_attrs(ast) {
        Ok(attrs) => attrs,
        Err(err) => {
            return err.into_compile_error();
        }
    };

    let scene = scene.unwrap_or_default();

    let (scene_impl, props_type) = match scene {
        Scene::Function(path) => (quote! {#path()}, quote! {()}),
        Scene::Asset(lit_str) => (
            quote! {#bevy_scene::InheritSceneAsset::from(#lit_str)},
            quote! {()},
        ),
        Scene::FunctionProps { function, props } => (quote! {#function(props)}, quote! {#props}),
    };

    let struct_name = &ast.ident;
    let (_, type_generics, _) = &ast.generics.split_for_impl();
    derive_component.additional_requires.push(quote! {
        required_components.register_required(|| #bevy_scene::SceneComponentInfo::new::<#struct_name #type_generics>(false));
    });
    let component_impl = match derive_component.impl_component(ast, &bevy_ecs) {
        Ok(value) => value,
        Err(err) => return err.into_compile_error(),
    };
    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();
    quote! {
        #component_impl

        impl #impl_generics #bevy_scene::SceneComponent for #struct_name #type_generics #where_clause {
            type Props = #props_type;
            fn scene(props: Self::Props) -> impl #bevy_scene::Scene {
                (
                    #scene_impl,
                    <#bevy_scene::InitTemplate::<<#struct_name #type_generics as #bevy_ecs::template::FromTemplate>::Template> as #FQDefault>::default(),
                    #bevy_scene::template_value(#bevy_scene::SceneComponentInfo::new::<#struct_name #type_generics>(true)),
                )
            }
        }
    }
}

fn parse_attrs(ast: &DeriveInput) -> syn::Result<Option<Scene>> {
    let mut scene = None;
    for attr in &ast.attrs {
        if attr.path().is_ident("scene") {
            scene = Some(attr.parse_args::<Scene>()?);
        }
    }
    Ok(scene)
}

/// Parsed scene information
pub(crate) enum Scene {
    Function(Path),
    FunctionProps { function: Path, props: Path },
    Asset(LitStr),
}

impl Default for Scene {
    fn default() -> Self {
        Scene::Function(default_path())
    }
}

fn default_path() -> Path {
    // Self::scene will always parse correctly
    parse_str::<Path>("Self::scene").unwrap()
}

impl Parse for Scene {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(if input.peek(LitStr) {
            Scene::Asset(input.parse::<LitStr>()?)
        } else {
            let path = input.parse::<Path>()?;
            if let PathType::Type = PathType::new(&path) {
                return Ok(Scene::FunctionProps {
                    function: default_path(),
                    props: path,
                });
            }

            if input.peek(Paren) {
                let content;
                parenthesized!(content in input);
                let props = content.parse::<Path>()?;
                Scene::FunctionProps {
                    function: path,
                    props,
                }
            } else {
                Scene::Function(path)
            }
        })
    }
}
