use proc_macro::TokenStream;
use proc_macro_crate::crate_name;
use syn::Path;

#[derive(Debug)]
pub struct Modules {
    pub bevy_render: String,
    pub bevy_asset: String,
    pub bevy_core: String,
    pub bevy_app: String,
    pub legion: String,
}

impl Modules {
    pub fn meta() -> Modules {
        Modules {
            bevy_asset: "bevy::asset".to_string(),
            bevy_render: "bevy::render".to_string(),
            bevy_core: "bevy::core".to_string(),
            bevy_app: "bevy::app".to_string(),
            legion: "bevy".to_string(),
        }
    }

    pub fn external() -> Modules {
        Modules {
            bevy_asset: "bevy_asset".to_string(),
            bevy_render: "bevy_render".to_string(),
            bevy_core: "bevy_core".to_string(),
            bevy_app: "bevy_app".to_string(),
            legion: "legion".to_string(),
        }
    }
}

fn use_meta() -> bool {
    crate_name("bevy").is_ok()
}

pub fn get_modules() -> Modules {
    if use_meta() {
        Modules::meta()
    } else {
        Modules::external()
    }
}

pub fn get_path(path_str: &str) -> Path {
    syn::parse(path_str.parse::<TokenStream>().unwrap()).unwrap()
}
