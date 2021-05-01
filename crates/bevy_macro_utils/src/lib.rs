extern crate proc_macro;

use cargo_manifest::{DepsSet, Manifest};
use proc_macro::TokenStream;
use std::{env, path::PathBuf};

pub fn get_path(path: &str) -> syn::Path {
    parse_str(path)
}

pub fn get_module_path(name: &str) -> syn::Path {
    get_module_path_by_manifest(name, &get_manifest())
}

pub fn get_module_path_by_manifest(name: &str, manifest: &Manifest) -> syn::Path {
    const BEVY: &str = "bevy";
    const BEVY_INTERNAL: &str = "bevy_internal";

    let find_in_deps = |deps: &DepsSet| -> Option<syn::Path> {
        let package = if let Some(dep) = deps.get(BEVY) {
            Some(dep.package().unwrap_or(BEVY))
        } else if let Some(dep) = deps.get(BEVY_INTERNAL) {
            Some(dep.package().unwrap_or(BEVY_INTERNAL))
        } else {
            None
        };

        package.map(get_path).map(|mut p| {
            if let Some(module) = name.strip_prefix("bevy_") {
                p.segments.push(parse_str(module));
            }
            p
        })
    };

    let deps = manifest.dependencies.as_ref();
    let deps_dev = manifest.dev_dependencies.as_ref();

    deps.and_then(find_in_deps)
        .or_else(|| deps_dev.and_then(find_in_deps))
        .unwrap_or_else(|| get_path(name))
}

pub fn get_manifest() -> Manifest {
    env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .map(|mut path| {
            path.push("Cargo.toml");
            Manifest::from_path(path).unwrap()
        })
        .unwrap()
}

fn parse_str<T: syn::parse::Parse>(path: &str) -> T {
    syn::parse(path.parse::<TokenStream>().unwrap()).unwrap()
}
