use darling::FromMeta;
use proc_macro::TokenStream;
use syn::{DeriveInput, Path};

#[derive(FromMeta, Debug)]
pub struct ModuleAttributeArgs {
    #[darling(default)]
    pub bevy_property: Option<String>,
    /// If true, it will use the meta "bevy" crate for dependencies by default (ex: bevy:app). If this is set to false, the individual bevy crates
    /// will be used (ex: "bevy_app"). Defaults to "true"
    #[darling(default)]
    pub meta: bool,
}

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

#[cfg(feature = "default_bevy_meta")]
impl Default for ModuleAttributeArgs {
    fn default() -> Self {
        ModuleAttributeArgs {
            bevy_property: None,
            meta: true,
        }
    }
}

#[cfg(not(feature = "default_bevy_meta"))]
impl Default for ModuleAttributeArgs {
    fn default() -> Self {
        ModuleAttributeArgs {
            bevy_property: None,
            meta: false,
        }
    }
}


pub static MODULE_ATTRIBUTE_NAME: &'static str = "module";

pub fn get_modules(ast: &DeriveInput) -> Modules {
    let module_attribute_args = ast
        .attrs
        .iter()
        .find(|a| a.path.get_ident().as_ref().unwrap().to_string() == MODULE_ATTRIBUTE_NAME)
        .map(|a| {
            ModuleAttributeArgs::from_meta(&a.parse_meta().unwrap())
                .unwrap_or_else(|_err| ModuleAttributeArgs::default())
        });
    if let Some(module_attribute_args) = module_attribute_args {
        let mut modules = if module_attribute_args.meta {
            Modules::meta()
        } else {
            Modules::external()
        };

        if let Some(path) = module_attribute_args.bevy_property {
            modules.bevy_property = path;
        }

        modules
    } else {
        Modules::meta()
    }
}

pub fn get_path(path_str: &str) -> Path {
    syn::parse(path_str.parse::<TokenStream>().unwrap()).unwrap()
}
