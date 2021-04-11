use cargo_manifest::{DepsSet, Manifest};
use proc_macro::TokenStream;
use std::{env, path::PathBuf};
use syn::Path;

#[derive(Debug)]
pub struct Modules {
    pub bevy_reflect: String,
}

impl Modules {
    pub fn meta(name: &str) -> Modules {
        Modules {
            bevy_reflect: format!("{}::reflect", name),
        }
    }

    pub fn external() -> Modules {
        Modules {
            bevy_reflect: "bevy_reflect".to_string(),
        }
    }

    pub fn internal() -> Modules {
        Modules {
            bevy_reflect: "crate".to_string(),
        }
    }
}

pub fn get_modules() -> Modules {
    const BEVY: &str = "bevy";
    const BEVY_EXTERNAL: &str = "bevy_reflect";
    const BEVY_INTERNAL: &str = "bevy_internal";

    fn find_in_deps(deps: DepsSet) -> Option<Modules> {
        if let Some(dep) = deps.get(BEVY) {
            Some(Modules::meta(dep.package().unwrap_or(BEVY)))
        } else if let Some(dep) = deps.get(BEVY_INTERNAL) {
            Some(Modules::meta(dep.package().unwrap_or(BEVY_INTERNAL)))
        } else if deps.get(BEVY_EXTERNAL).is_some() {
            Some(Modules::external())
        } else {
            None
        }
    }

    let manifest = env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .map(|mut path| {
            path.push("Cargo.toml");
            Manifest::from_path(path).unwrap()
        })
        .unwrap();
    let deps = manifest.dependencies;
    let deps_dev = manifest.dev_dependencies;

    manifest
        .package
        .and_then(|p| {
            if p.name == BEVY_EXTERNAL {
                Some(Modules::internal())
            } else {
                None
            }
        })
        .or_else(|| deps.and_then(find_in_deps))
        .or_else(|| deps_dev.and_then(find_in_deps))
        .unwrap_or_else(Modules::external)
}

pub fn get_path(path_str: &str) -> Path {
    syn::parse(path_str.parse::<TokenStream>().unwrap()).unwrap()
}
