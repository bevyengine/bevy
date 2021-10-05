extern crate proc_macro;

mod attrs;
mod symbol;

pub use attrs::*;
pub use symbol::*;

use cargo_manifest::{DepsSet, Manifest};
use proc_macro::TokenStream;
use std::{env, path::PathBuf};

pub struct BevyManifest {
    manifest: Manifest,
}

impl Default for BevyManifest {
    fn default() -> Self {
        Self {
            manifest: env::var_os("CARGO_MANIFEST_DIR")
                .map(PathBuf::from)
                .map(|mut path| {
                    path.push("Cargo.toml");
                    Manifest::from_path(path).unwrap()
                })
                .unwrap(),
        }
    }
}

impl BevyManifest {
    pub fn get_path(&self, name: &str) -> syn::Path {
        const BEVY: &str = "bevy";
        const BEVY_INTERNAL: &str = "bevy_internal";

        let find_in_deps = |deps: &DepsSet| -> Option<syn::Path> {
            let package = if let Some(dep) = deps.get(BEVY) {
                dep.package().unwrap_or(BEVY)
            } else if let Some(dep) = deps.get(BEVY_INTERNAL) {
                dep.package().unwrap_or(BEVY_INTERNAL)
            } else {
                return None;
            };

            let mut path = get_path(package);
            if let Some(module) = name.strip_prefix("bevy_") {
                path.segments.push(parse_str(module));
            }
            Some(path)
        };

        let deps = self.manifest.dependencies.as_ref();
        let deps_dev = self.manifest.dev_dependencies.as_ref();

        deps.and_then(find_in_deps)
            .or_else(|| deps_dev.and_then(find_in_deps))
            .unwrap_or_else(|| get_path(name))
    }
}

fn get_path(path: &str) -> syn::Path {
    parse_str(path)
}

fn parse_str<T: syn::parse::Parse>(path: &str) -> T {
    syn::parse(path.parse::<TokenStream>().unwrap()).unwrap()
}
