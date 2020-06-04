use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro_crate::crate_name;
use syn::{DeriveInput, Path};

#[derive(FromMeta, Debug, Default)]
pub struct ModuleAttributeArgs {
    #[darling(default)]
    pub bevy_render: Option<String>,
    #[darling(default)]
    pub bevy_asset: Option<String>,
    #[darling(default)]
    pub bevy_core: Option<String>,
    #[darling(default)]
    pub bevy_app: Option<String>,
    #[darling(default)]
    pub legion: Option<String>,

    /// If true, it will use the meta "bevy" crate for dependencies by default (ex: bevy:app). If this is set to false, the individual bevy crates
    /// will be used (ex: "bevy_app"). Defaults to "true" if the "bevy" crate is in your cargo.toml
    #[darling(default)]
    pub meta: Option<bool>,
}

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

    if let Some(path) = module_attribute_args.bevy_asset {
        modules.bevy_asset = path;
    }

    if let Some(path) = module_attribute_args.bevy_render {
        modules.bevy_render = path;
    }

    if let Some(path) = module_attribute_args.bevy_core {
        modules.bevy_core = path;
    }

    if let Some(path) = module_attribute_args.bevy_app {
        modules.bevy_app = path;
    }

    modules
}

pub fn get_path(path_str: &str) -> Path {
    syn::parse(path_str.parse::<TokenStream>().unwrap()).unwrap()
}
