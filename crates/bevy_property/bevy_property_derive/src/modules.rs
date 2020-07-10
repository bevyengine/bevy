use proc_macro::TokenStream;
use proc_macro_crate::crate_name;
use syn::Path;

#[derive(Debug)]
pub struct Modules {
    pub bevy_property: String,
}

impl Modules {
    pub fn meta() -> Modules {
        Modules {
            bevy_property: "bevy::property".to_string(),
        }
    }

    pub fn external() -> Modules {
        Modules {
            bevy_property: "bevy_property".to_string(),
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
