use proc_macro::TokenStream;
use proc_macro_crate::crate_name;
use syn::{Attribute, Path};

#[derive(Debug)]
pub struct Modules {
    pub bevy_render: String,
    pub bevy_asset: String,
    pub bevy_core: String,
    pub bevy_app: String,
}

impl Modules {
    pub fn meta() -> Modules {
        Modules {
            bevy_asset: "bevy::asset".to_string(),
            bevy_render: "bevy::render".to_string(),
            bevy_core: "bevy::core".to_string(),
            bevy_app: "bevy::app".to_string(),
        }
    }

    pub fn external() -> Modules {
        Modules {
            bevy_asset: "bevy_asset".to_string(),
            bevy_render: "bevy_render".to_string(),
            bevy_core: "bevy_core".to_string(),
            bevy_app: "bevy_app".to_string(),
        }
    }
}

fn use_meta() -> bool {
    crate_name("bevy").is_ok()
}

const AS_CRATE_ATTRIBUTE_NAME: &str = "as_crate";

pub fn get_modules(attributes: &[Attribute]) -> Modules {
    let mut modules = if use_meta() {
        Modules::meta()
    } else {
        Modules::external()
    };

    for attribute in attributes.iter() {
        if *attribute.path.get_ident().as_ref().unwrap() == AS_CRATE_ATTRIBUTE_NAME {
            let value = attribute.tokens.to_string();
            if value[1..value.len() - 1] == modules.bevy_render {
                modules.bevy_render = "crate".to_string();
            }
        }
    }

    modules
}

pub fn get_path(path_str: &str) -> Path {
    syn::parse(path_str.parse::<TokenStream>().unwrap()).unwrap()
}
