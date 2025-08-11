extern crate proc_macro;

use alloc::collections::BTreeMap;
use parking_lot::{lock_api::RwLockReadGuard, MappedRwLockReadGuard, RwLock, RwLockWriteGuard};
use proc_macro::TokenStream;
use std::{
    env,
    path::{Path, PathBuf},
    time::SystemTime,
};
use toml_edit::{Document, Item};

/// The path to the `Cargo.toml` file for the Bevy project.
#[derive(Debug)]
pub struct BevyManifest {
    manifest: Document<Box<str>>,
    modified_time: SystemTime,
}

const BEVY: &str = "bevy";

impl BevyManifest {
    /// Returns a global shared instance of the [`BevyManifest`] struct.
    pub fn shared() -> MappedRwLockReadGuard<'static, BevyManifest> {
        static MANIFESTS: RwLock<BTreeMap<PathBuf, BevyManifest>> = RwLock::new(BTreeMap::new());
        let manifest_path = Self::get_manifest_path();
        let modified_time = Self::get_manifest_modified_time(&manifest_path)
            .expect("The Cargo.toml should have a modified time");

        if let Ok(manifest) =
            RwLockReadGuard::try_map(MANIFESTS.read(), |manifests| manifests.get(&manifest_path))
            && manifest.modified_time == modified_time
        {
            return manifest;
        }

        let manifest = BevyManifest {
            manifest: Self::read_manifest(&manifest_path),
            modified_time,
        };

        let key = manifest_path.clone();
        let mut manifests = MANIFESTS.write();
        manifests.insert(key, manifest);

        RwLockReadGuard::map(RwLockWriteGuard::downgrade(manifests), |manifests| {
            manifests.get(&manifest_path).unwrap()
        })
    }

    fn get_manifest_path() -> PathBuf {
        env::var_os("CARGO_MANIFEST_DIR")
            .map(|path| {
                let mut path = PathBuf::from(path);
                path.push("Cargo.toml");
                assert!(
                    path.exists(),
                    "Cargo manifest does not exist at path {}",
                    path.display()
                );
                path
            })
            .expect("CARGO_MANIFEST_DIR is not defined.")
    }

    fn get_manifest_modified_time(
        cargo_manifest_path: &Path,
    ) -> Result<SystemTime, std::io::Error> {
        std::fs::metadata(cargo_manifest_path).and_then(|metadata| metadata.modified())
    }

    fn read_manifest(path: &Path) -> Document<Box<str>> {
        let manifest = std::fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Unable to read cargo manifest: {}", path.display()))
            .into_boxed_str();
        Document::parse(manifest)
            .unwrap_or_else(|_| panic!("Failed to parse cargo manifest: {}", path.display()))
    }

    /// Attempt to retrieve the [path](syn::Path) of a particular package in
    /// the [manifest](BevyManifest) by [name](str).
    pub fn maybe_get_path(&self, name: &str) -> Option<syn::Path> {
        let find_in_deps = |deps: &Item| -> Option<syn::Path> {
            let package = if deps.get(name).is_some() {
                return Some(Self::parse_str(name));
            } else if deps.get(BEVY).is_some() {
                BEVY
            } else {
                // Note: to support bevy crate aliases, we could do scanning here to find a crate with a "package" name that
                // matches our request, but that would then mean we are scanning every dependency (and dev dependency) for every
                // macro execution that hits this branch (which includes all built-in bevy crates). Our current stance is that supporting
                // remapped crate names in derive macros is not worth that "compile time" price of admission. As a workaround, people aliasing
                // bevy crate names can use "use REMAPPED as bevy_X" or "use REMAPPED::x as bevy_x".
                return None;
            };

            let mut path = Self::parse_str::<syn::Path>(&format!("::{package}"));
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

    /// Attempt to parse the provided [path](str) as a [syntax tree node](syn::parse::Parse)
    pub fn try_parse_str<T: syn::parse::Parse>(path: &str) -> Option<T> {
        syn::parse(path.parse::<TokenStream>().ok()?).ok()
    }

    /// Returns the path for the crate with the given name.
    pub fn get_path(&self, name: &str) -> syn::Path {
        self.maybe_get_path(name)
            .unwrap_or_else(|| Self::parse_str(name))
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
}
