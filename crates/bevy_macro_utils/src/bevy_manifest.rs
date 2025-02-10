extern crate proc_macro;

use std::sync::MutexGuard;

use cargo_manifest_proc_macros::{
    CargoManifest, CrateReExportingPolicy, KnownReExportingCrate, PathPiece,
    TryResolveCratePathError,
};
use proc_macro::TokenStream;

struct BevyReExportingPolicy;

impl CrateReExportingPolicy for BevyReExportingPolicy {
    fn get_re_exported_crate_path(&self, crate_name: &str) -> Option<PathPiece> {
        crate_name.strip_prefix("bevy_").map(|s| {
            let mut path = PathPiece::new();
            path.push(syn::parse_str::<syn::PathSegment>(s).unwrap());
            path
        })
    }
}

const BEVY: &str = "bevy";

const KNOWN_RE_EXPORTING_CRATE_BEVY: KnownReExportingCrate = KnownReExportingCrate {
    re_exporting_crate_package_name: BEVY,
    crate_re_exporting_policy: &BevyReExportingPolicy {},
};

const ALL_KNOWN_RE_EXPORTING_CRATES: &[&KnownReExportingCrate] = &[&KNOWN_RE_EXPORTING_CRATE_BEVY];

/// The path to the `Cargo.toml` file for the Bevy project.
pub struct BevyManifest(MutexGuard<'static, CargoManifest>);

impl BevyManifest {
    /// Returns a global shared instance of the [`BevyManifest`] struct.
    pub fn shared() -> Self {
        Self(CargoManifest::shared())
    }

    /// Attempt to retrieve the [path](syn::Path) of a particular package in
    /// the [manifest](BevyManifest) by [name](str).
    pub fn maybe_get_path(&self, name: &str) -> Result<syn::Path, TryResolveCratePathError> {
        self.0
            .try_resolve_crate_path(name, ALL_KNOWN_RE_EXPORTING_CRATES)
    }

    /// Returns the path for the crate with the given name.
    pub fn get_path(&self, name: &str) -> syn::Path {
        self.maybe_get_path(name)
            //.expect("Failed to get path for crate")
            .unwrap_or_else(|_err| Self::parse_str(name))
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
    pub fn get_subcrate(&self, subcrate: &str) -> Result<syn::Path, TryResolveCratePathError> {
        self.maybe_get_path(BEVY)
            .map(|bevy_path| {
                let mut segments = bevy_path.segments;
                segments.push(BevyManifest::parse_str(subcrate));
                syn::Path {
                    leading_colon: None,
                    segments,
                }
            })
            .or_else(|_err| self.maybe_get_path(&format!("bevy_{subcrate}")))
    }
}
