use darling::FromMeta;
use proc_macro::TokenStream;
use syn::{DeriveInput, Path};

#[derive(FromMeta, Debug)]
pub struct ModuleAttributeArgs {
    #[darling(default)]
    pub bevy_render: Option<String>,
    #[darling(default)]
    pub bevy_asset: Option<String>,
    #[darling(default)]
    pub bevy_core: Option<String>,
    #[darling(default)]
    pub bevy_property: Option<String>,
    #[darling(default)]
    pub bevy_app: Option<String>,
    #[darling(default)]
    pub legion: Option<String>,

    /// If true, it will use the meta "bevy" crate for dependencies by default (ex: bevy:app). If this is set to false, the individual bevy crates
    /// will be used (ex: "bevy_app"). Defaults to "true"
    #[darling(default)]
    pub meta: bool,
}

pub struct Modules {
    pub bevy_render: String,
    pub bevy_asset: String,
    pub bevy_core: String,
    pub bevy_property: String,
    pub bevy_app: String,
    pub legion: String,
}

impl Modules {
    pub fn meta() -> Modules {
        Modules {
            bevy_asset: "bevy::asset".to_string(),
            bevy_render: "bevy::render".to_string(),
            bevy_core: "bevy::core".to_string(),
            bevy_property: "bevy::property".to_string(),
            bevy_app: "bevy::app".to_string(),
            legion: "bevy".to_string(),
        }
    }

    pub fn external() -> Modules {
        Modules {
            bevy_asset: "bevy_asset".to_string(),
            bevy_render: "bevy_render".to_string(),
            bevy_core: "bevy_core".to_string(),
            bevy_property: "bevy_property".to_string(),
            bevy_app: "bevy_app".to_string(),
            legion: "legion".to_string(),
        }
    }
}

impl Default for ModuleAttributeArgs {
    fn default() -> Self {
        ModuleAttributeArgs {
            bevy_asset: None,
            bevy_render: None,
            bevy_core: None,
            bevy_property: None,
            bevy_app: None,
            legion: None,
            meta: true,
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

        if let Some(path) = module_attribute_args.bevy_asset {
            modules.bevy_asset = path;
        }

        if let Some(path) = module_attribute_args.bevy_render {
            modules.bevy_render = path;
        }

        if let Some(path) = module_attribute_args.bevy_property {
            modules.bevy_property = path;
        }

        if let Some(path) = module_attribute_args.bevy_core {
            modules.bevy_core = path;
        }

        if let Some(path) = module_attribute_args.bevy_app {
            modules.bevy_app = path;
        }

        modules
    } else {
        Modules::meta()
    }
}

pub fn get_path(path_str: &str) -> Path {
    syn::parse(path_str.parse::<TokenStream>().unwrap()).unwrap()
}
