use find_crate::Manifest;
use proc_macro::TokenStream;
use syn::{Attribute, Path};

#[derive(Debug)]
pub struct Modules {
    pub bevy_render: String,
    pub bevy_asset: String,
    pub bevy_core: String,
    pub bevy_app: String,
}

impl Modules {
    pub fn meta(name: &str) -> Modules {
        Modules {
            bevy_asset: format!("{}::asset", name),
            bevy_render: format!("{}::render", name),
            bevy_core: format!("{}::core", name),
            bevy_app: format!("{}::app", name),
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

fn get_meta() -> Option<Modules> {
    let manifest = Manifest::new().unwrap();
    if let Some(package) = manifest.find(|name| name == "bevy") {
        Some(Modules::meta(&package.name))
    } else if let Some(package) = manifest.find(|name| name == "bevy_internal") {
        Some(Modules::meta(&package.name))
    } else {
        None
    }
}

const AS_CRATE_ATTRIBUTE_NAME: &str = "as_crate";

pub fn get_modules(attributes: &[Attribute]) -> Modules {
    let mut modules = get_meta().unwrap_or_else(Modules::external);
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
