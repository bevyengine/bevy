use bevy_macro_utils::{get_manifest, get_module_path_by_manifest, get_path};
use syn::Attribute;

pub struct Modules {
    pub bevy_app: syn::Path,
    pub bevy_asset: syn::Path,
    pub bevy_core: syn::Path,
    pub bevy_render: syn::Path,
    pub bevy_utils: syn::Path,
}

const AS_CRATE_ATTRIBUTE_NAME: &str = "as_crate";

fn validate_as_crate_attribute(tokens: &str) -> bool {
    tokens.len() > 2 && tokens.starts_with('(') && tokens.ends_with(')')
}

pub fn get_modules(attributes: &[Attribute]) -> Modules {
    let manifest = get_manifest();
    let mut modules = Modules {
        bevy_app: get_module_path_by_manifest("bevy_app", &manifest),
        bevy_asset: get_module_path_by_manifest("bevy_asset", &manifest),
        bevy_core: get_module_path_by_manifest("bevy_core", &manifest),
        bevy_render: get_module_path_by_manifest("bevy_render", &manifest),
        bevy_utils: get_module_path_by_manifest("bevy_utils", &manifest),
    };
    for attribute in attributes.iter() {
        if *attribute.path.get_ident().as_ref().unwrap() == AS_CRATE_ATTRIBUTE_NAME {
            let value = attribute.tokens.to_string();
            if !validate_as_crate_attribute(&value) {
                panic!("The attribute `#[as_crate{}]` is invalid. It must follow the format `#[as_crate(<crate name>)]`", value);
            } else if get_path(&value[1..value.len() - 1]) == modules.bevy_render {
                modules.bevy_render = get_path("crate");
            }
        }
    }

    modules
}
