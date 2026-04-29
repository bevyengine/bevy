use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_str, DeriveInput, LitStr, Path, Token,
};

/// Parsed scene information
pub(crate) enum Scene {
    Function(Path),
    Asset(LitStr),
}

impl Scene {
    pub(crate) fn default_scene_function() -> Self {
        // Self::scene will always parse correctly
        Scene::Function(parse_str::<Path>("Self::scene").unwrap())
    }
}

impl Parse for Scene {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            if input.peek(LitStr) {
                Scene::Asset(input.parse::<LitStr>()?)
            } else {
                Scene::Function(input.parse::<Path>()?)
            }
        } else {
            Scene::default_scene_function()
        })
    }
}

pub(crate) fn derive_scene_constructor(
    ast: &DeriveInput,
    bevy_ecs: &Path,
    bevy_scene: &Path,
    scene: Scene,
    scene_props: Option<Path>,
) -> TokenStream {
    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    let scene_impl = match scene {
        Scene::Function(path) => {
            if scene_props.is_some() {
                quote! {#path(props)}
            } else {
                quote! {#path()}
            }
        }
        Scene::Asset(lit_str) => quote! {#bevy_scene::InheritSceneAsset::from(#lit_str)},
    };
    let props_type = match scene_props {
        Some(props) => quote! {#props},
        None => quote! {()},
    };
    quote! {
        impl #impl_generics #bevy_scene::SceneConstructor for #struct_name #type_generics #where_clause {
            type Props = #props_type;
            fn scene(props: Self::Props) -> impl Scene {
                (
                    #scene_impl,
                    #bevy_scene::InitTemplate::<<#struct_name #type_generics as #bevy_ecs::template::FromTemplate>::Template>::default(),
                    #bevy_scene::template_value(#bevy_scene::SceneComponentInfo::new::<#struct_name #type_generics>(true)),
                )
            }
        }
    }
}
