use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro_crate::crate_name;
use syn::{DeriveInput, Path};

#[derive(FromMeta, Debug, Default)]
pub struct ModuleAttributeArgs {
    pub bevy_property: Option<String>,
    /// If true, it will use the meta "bevy" crate for dependencies by default (ex: bevy:app). If this is set to false, the individual bevy crates
    /// will be used (ex: "bevy_app"). Defaults to "true" if the "bevy" crate is in your cargo.toml
    #[darling(default)]
    pub meta: Option<bool>,
}

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

pub static MODULE_ATTRIBUTE_NAME: &'static str = "module";

pub fn get_modules(ast: &DeriveInput) -> Modules {
    let module_attribute_args = ast
        .attrs
        .iter()
        .find(|a| a.path.get_ident().as_ref().unwrap().to_string() == MODULE_ATTRIBUTE_NAME)
        .map_or_else(
            || ModuleAttributeArgs::default(),
            |a| {
                ModuleAttributeArgs::from_meta(&a.parse_meta().unwrap())
                    .unwrap_or_else(|_err| ModuleAttributeArgs::default())
            },
        );

    let mut modules = if module_attribute_args.meta.unwrap_or_else(|| use_meta()) {
        Modules::meta()
    } else {
        Modules::external()
    };

    if let Some(path) = module_attribute_args.bevy_property {
        modules.bevy_property = path;
    }

    modules
}

pub fn get_path(path_str: &str) -> Path {
    syn::parse(path_str.parse::<TokenStream>().unwrap()).unwrap()
}
