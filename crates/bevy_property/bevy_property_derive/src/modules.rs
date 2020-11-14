use find_crate::Manifest;
use proc_macro::TokenStream;
use syn::Path;

#[derive(Debug)]
pub struct Modules {
    pub bevy_property: String,
}

impl Modules {
    pub fn meta(name: &str) -> Modules {
        Modules {
            bevy_property: format!("{}::property", name),
        }
    }

    pub fn external() -> Modules {
        Modules {
            bevy_property: "bevy_property".to_string(),
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

pub fn get_modules() -> Modules {
    get_meta().unwrap_or_else(Modules::external)
}

pub fn get_path(path_str: &str) -> Path {
    syn::parse(path_str.parse::<TokenStream>().unwrap()).unwrap()
}
