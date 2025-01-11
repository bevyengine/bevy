extern crate proc_macro;

use proc_macro::TokenStream;
use std::{
    collections::HashMap,
    env,
    ffi::OsString,
    path::PathBuf,
    sync::{LazyLock, Mutex},
};
use toml_edit::{DocumentMut, Item};

/// The path to the `Cargo.toml` file for the Bevy project.
pub struct BevyManifest {
    manifest: DocumentMut,
}

const BEVY: &str = "bevy";
const BEVY_INTERNAL: &str = "bevy_internal";

impl BevyManifest {
    /// Returns a shared instance of the [`BevyManifest`] struct. All callers invoking this function
    /// with the same value of the `CARGO_MANIFEST_DIR` environment variable receive the same
    /// instance.
    pub fn shared() -> &'static Self {
        static BEVY_MANIFESTS: LazyLock<Mutex<HashMap<OsString, &'static BevyManifest>>> =
            LazyLock::new(|| Mutex::new(HashMap::new()));
        let manifest_dir =
            env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not defined.");
        BEVY_MANIFESTS
            .lock()
            .unwrap()
            .entry(manifest_dir)
            .or_insert_with_key(|manifest_dir| {
                let mut path = PathBuf::from(manifest_dir);
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
                Box::leak(Box::new(BevyManifest {
                    manifest: manifest.parse::<DocumentMut>().unwrap_or_else(|_| {
                        panic!("Failed to parse cargo manifest: {}", path.display())
                    }),
                }))
            })
    }

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
    /// Will panic if the path is not able to be parsed. For a non-panicking option, see [`try_parse_str`]
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
