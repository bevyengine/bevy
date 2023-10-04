extern crate proc_macro;

use proc_macro::TokenStream;
use std::{env, path::PathBuf};
use toml_edit::{Document, Item};

/// The path to the `Cargo.toml` file for the Bevy project.
pub struct BevyManifest {
    manifest: Document,
}

impl Default for BevyManifest {
    fn default() -> Self {
        Self {
            manifest: env::var_os("CARGO_MANIFEST_DIR")
                .map(PathBuf::from)
                .map(|mut path| {
                    path.push("Cargo.toml");
                    if !path.exists() {
                        panic!(
                            "No Cargo manifest found for crate. Expected: {}",
                            path.display()
                        );
                    }
                    let manifest = std::fs::read_to_string(path.clone()).unwrap_or_else(|_| {
                        panic!("Unable to read cargo manifest: {}", path.display())
                    });
                    manifest.parse::<Document>().unwrap_or_else(|_| {
                        panic!("Failed to parse cargo manifest: {}", path.display())
                    })
                })
                .expect("CARGO_MANIFEST_DIR is not defined."),
        }
    }
}
const BEVY: &str = "bevy";
const BEVY_INTERNAL: &str = "bevy_internal";

impl BevyManifest {
    /// Attempt to retrieve the [path](syn::Path) of a particular package in
    /// the [manifest](BevyManifest) by [name](str).
    pub fn maybe_get_path(&self, name: &str) -> Option<syn::Path> {
        fn dep_package(dep: &Item) -> Option<&str> {
            if dep.as_str().is_some() {
                None
            } else {
                dep.get("package").map(|name| name.as_str().unwrap())
            }
        }

        let find_in_deps = |deps: &Item| -> Option<syn::Path> {
            let package = if let Some(dep) = deps.get(name) {
                return Some(Self::parse_str(dep_package(dep).unwrap_or(name)));
            } else if let Some(dep) = deps.get(BEVY) {
                dep_package(dep).unwrap_or(BEVY)
            } else if let Some(dep) = deps.get(BEVY_INTERNAL) {
                dep_package(dep).unwrap_or(BEVY_INTERNAL)
            } else {
                return None;
            };

            let mut path = Self::parse_str::<syn::Path>(package);
            if let Some(module) = name.strip_prefix("bevy_") {
                path.segments.push(Self::parse_str(module));
            }
            Some(path)
        };

        let deps = self.manifest.get("dependencies");
        let deps_dev = self.manifest.get("dev-dependencies");

        deps.and_then(find_in_deps)
            .or_else(|| deps_dev.and_then(find_in_deps))
    }

    /// Returns the path for the crate with the given name.
    ///
    /// This is a convenience method for constructing a [manifest] and
    /// calling the [`get_path`] method.
    ///
    /// This method should only be used where you just need the path and can't
    /// cache the [manifest]. If caching is possible, it's recommended to create
    /// the [manifest] yourself and use the [`get_path`] method.
    ///
    /// [`get_path`]: Self::get_path
    /// [manifest]: Self
    pub fn get_path_direct(name: &str) -> syn::Path {
        Self::default().get_path(name)
    }

    /// Returns the path for the crate with the given name.
    pub fn get_path(&self, name: &str) -> syn::Path {
        self.maybe_get_path(name)
            .unwrap_or_else(|| Self::parse_str(name))
    }

    /// Attempt to parse the provided [path](str) as a [syntax tree node](syn::parse::Parse)
    pub fn try_parse_str<T: syn::parse::Parse>(path: &str) -> Option<T> {
        syn::parse(path.parse::<TokenStream>().ok()?).ok()
    }

    /// Attempt to parse provided [path](str) as a [syntax tree node](syn::parse::Parse).
    ///
    /// # Panics
    ///
    /// Will panic if the path is not able to be parsed. For a non-panicing option, see [`try_parse_str`]
    ///
    /// [`try_parse_str`]: Self::try_parse_str
    pub fn parse_str<T: syn::parse::Parse>(path: &str) -> T {
        Self::try_parse_str(path).unwrap()
    }

    /// Attempt to get a subcrate [path](syn::Path) under Bevy by [name](str)
    pub fn get_subcrate(&self, subcrate: &str) -> Option<syn::Path> {
        self.maybe_get_path(BEVY)
            .map(|bevy_path| {
                let mut segments = bevy_path.segments;
                segments.push(BevyManifest::parse_str(subcrate));
                syn::Path {
                    leading_colon: None,
                    segments,
                }
            })
            .or_else(|| self.maybe_get_path(&format!("bevy_{subcrate}")))
    }
}
