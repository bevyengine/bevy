#[cfg(feature = "embedded_watcher")]
mod embedded_watcher;

#[cfg(feature = "embedded_watcher")]
pub use embedded_watcher::*;

use crate::io::{
    memory::{Dir, MemoryAssetReader, Value},
    AssetSource, AssetSourceBuilders,
};
use crate::AssetServer;
use alloc::boxed::Box;
use bevy_app::App;
use bevy_ecs::{resource::Resource, world::World};
use std::path::{Path, PathBuf};

#[cfg(feature = "embedded_watcher")]
use alloc::borrow::ToOwned;

/// The name of the `embedded` [`AssetSource`],
/// as stored in the [`AssetSourceBuilders`] resource.
pub const EMBEDDED: &str = "embedded";

/// A [`Resource`] that manages "rust source files" in a virtual in memory [`Dir`], which is intended
/// to be shared with a [`MemoryAssetReader`].
/// Generally this should not be interacted with directly. The [`embedded_asset`] will populate this.
///
/// [`embedded_asset`]: crate::embedded_asset
#[derive(Resource, Default)]
pub struct EmbeddedAssetRegistry {
    dir: Dir,
    #[cfg(feature = "embedded_watcher")]
    root_paths: alloc::sync::Arc<
        parking_lot::RwLock<bevy_platform::collections::HashMap<Box<Path>, PathBuf>>,
    >,
}

impl EmbeddedAssetRegistry {
    /// Inserts a new asset. `full_path` is the full path (as [`file`] would return for that file, if it was capable of
    /// running in a non-rust file). `asset_path` is the path that will be used to identify the asset in the `embedded`
    /// [`AssetSource`]. `value` is the bytes that will be returned for the asset. This can be _either_ a `&'static [u8]`
    /// or a [`Vec<u8>`](alloc::vec::Vec).
    #[cfg_attr(
        not(feature = "embedded_watcher"),
        expect(
            unused_variables,
            reason = "The `full_path` argument is not used when `embedded_watcher` is disabled."
        )
    )]
    pub fn insert_asset(&self, full_path: PathBuf, asset_path: &Path, value: impl Into<Value>) {
        #[cfg(feature = "embedded_watcher")]
        self.root_paths
            .write()
            .insert(full_path.into(), asset_path.to_owned());
        self.dir.insert_asset(asset_path, value);
    }

    /// Inserts new asset metadata. `full_path` is the full path (as [`file`] would return for that file, if it was capable of
    /// running in a non-rust file). `asset_path` is the path that will be used to identify the asset in the `embedded`
    /// [`AssetSource`]. `value` is the bytes that will be returned for the asset. This can be _either_ a `&'static [u8]`
    /// or a [`Vec<u8>`](alloc::vec::Vec).
    #[cfg_attr(
        not(feature = "embedded_watcher"),
        expect(
            unused_variables,
            reason = "The `full_path` argument is not used when `embedded_watcher` is disabled."
        )
    )]
    pub fn insert_meta(&self, full_path: &Path, asset_path: &Path, value: impl Into<Value>) {
        #[cfg(feature = "embedded_watcher")]
        self.root_paths
            .write()
            .insert(full_path.into(), asset_path.to_owned());
        self.dir.insert_meta(asset_path, value);
    }

    /// Removes an asset stored using `full_path` (the full path as [`file`] would return for that file, if it was capable of
    /// running in a non-rust file). If no asset is stored with at `full_path` its a no-op.
    /// It returning `Option` contains the originally stored `Data` or `None`.
    pub fn remove_asset(&self, full_path: &Path) -> Option<super::memory::Data> {
        self.dir.remove_asset(full_path)
    }

    /// Registers the [`EMBEDDED`] [`AssetSource`] with the given [`AssetSourceBuilders`].
    pub fn register_source(&self, sources: &mut AssetSourceBuilders) {
        let dir = self.dir.clone();
        let processed_dir = self.dir.clone();

        #[cfg_attr(
            not(feature = "embedded_watcher"),
            expect(
                unused_mut,
                reason = "Variable is only mutated when `embedded_watcher` feature is enabled."
            )
        )]
        let mut source = AssetSource::build()
            .with_reader(move || Box::new(MemoryAssetReader { root: dir.clone() }))
            .with_processed_reader(move || {
                Box::new(MemoryAssetReader {
                    root: processed_dir.clone(),
                })
            })
            // Note that we only add a processed watch warning because we don't want to warn
            // noisily about embedded watching (which is niche) when users enable file watching.
            .with_processed_watch_warning(
                "Consider enabling the `embedded_watcher` cargo feature.",
            );

        #[cfg(feature = "embedded_watcher")]
        {
            let root_paths = self.root_paths.clone();
            let dir = self.dir.clone();
            let processed_root_paths = self.root_paths.clone();
            let processed_dir = self.dir.clone();
            source = source
                .with_watcher(move |sender| {
                    Some(Box::new(EmbeddedWatcher::new(
                        dir.clone(),
                        root_paths.clone(),
                        sender,
                        core::time::Duration::from_millis(300),
                    )))
                })
                .with_processed_watcher(move |sender| {
                    Some(Box::new(EmbeddedWatcher::new(
                        processed_dir.clone(),
                        processed_root_paths.clone(),
                        sender,
                        core::time::Duration::from_millis(300),
                    )))
                });
        }
        sources.insert(EMBEDDED, source);
    }
}

/// Trait for the [`load_embedded_asset!`] macro, to access [`AssetServer`]
/// from arbitrary things.
///
/// [`load_embedded_asset!`]: crate::load_embedded_asset
pub trait GetAssetServer {
    fn get_asset_server(&self) -> &AssetServer;
}

impl GetAssetServer for App {
    fn get_asset_server(&self) -> &AssetServer {
        self.world().get_asset_server()
    }
}

impl GetAssetServer for World {
    fn get_asset_server(&self) -> &AssetServer {
        self.resource()
    }
}

impl GetAssetServer for AssetServer {
    fn get_asset_server(&self) -> &AssetServer {
        self
    }
}

/// Load an [embedded asset](crate::embedded_asset).
///
/// This is useful if the embedded asset in question is not publicly exposed, but
/// you need to use it internally.
///
/// # Syntax
///
/// This macro takes two arguments and an optional third one:
/// 1. The asset source. It may be `AssetServer`, `World` or `App`.
/// 2. The path to the asset to embed, as a string literal.
/// 3. Optionally, a closure of the same type as in [`AssetServer::load_with_settings`].
///    Consider explicitly typing the closure argument in case of type error.
///
/// # Usage
///
/// The advantage compared to using directly [`AssetServer::load`] is:
/// - This also accepts [`World`] and [`App`] arguments.
/// - This uses the exact same path as `embedded_asset!`, so you can keep it
///   consistent.
///
/// As a rule of thumb:
/// - If the asset in used in the same module as it is declared using `embedded_asset!`,
///   use this macro.
/// - Otherwise, use `AssetServer::load`.
#[macro_export]
macro_rules! load_embedded_asset {
    (@get: $path: literal, $provider: expr) => {{
        let path = $crate::embedded_path!($path);
        let path = $crate::AssetPath::from_path_buf(path).with_source("embedded");
        let asset_server = $crate::io::embedded::GetAssetServer::get_asset_server($provider);
        (path, asset_server)
    }};
    ($provider: expr, $path: literal, $settings: expr) => {{
        let (path, asset_server) = $crate::load_embedded_asset!(@get: $path, $provider);
        asset_server.load_with_settings(path, $settings)
    }};
    ($provider: expr, $path: literal) => {{
        let (path, asset_server) = $crate::load_embedded_asset!(@get: $path, $provider);
        asset_server.load(path)
    }};
}

/// Returns the [`Path`] for a given `embedded` asset.
/// This is used internally by [`embedded_asset`] and can be used to get a [`Path`]
/// that matches the [`AssetPath`](crate::AssetPath) used by that asset.
///
/// [`embedded_asset`]: crate::embedded_asset
#[macro_export]
macro_rules! embedded_path {
    ($path_str: expr) => {{
        $crate::embedded_path!("src", $path_str)
    }};

    ($source_path: expr, $path_str: expr) => {{
        let crate_name = module_path!().split(':').next().unwrap();
        $crate::io::embedded::_embedded_asset_path(
            crate_name,
            $source_path.as_ref(),
            file!().as_ref(),
            $path_str.as_ref(),
        )
    }};
}

/// Implementation detail of `embedded_path`, do not use this!
///
/// Returns an embedded asset path, given:
///   - `crate_name`: name of the crate where the asset is embedded
///   - `src_prefix`: path prefix of the crate's source directory, relative to the workspace root
///   - `file_path`: `std::file!()` path of the source file where `embedded_path!` is called
///   - `asset_path`: path of the embedded asset relative to `file_path`
#[doc(hidden)]
pub fn _embedded_asset_path(
    crate_name: &str,
    src_prefix: &Path,
    file_path: &Path,
    asset_path: &Path,
) -> PathBuf {
    let file_path = if cfg!(not(target_family = "windows")) {
        // Work around bug: https://github.com/bevyengine/bevy/issues/14246
        // Note, this will break any paths on Linux/Mac containing "\"
        PathBuf::from(file_path.to_str().unwrap().replace("\\", "/"))
    } else {
        PathBuf::from(file_path)
    };
    let mut maybe_parent = file_path.parent();
    let after_src = loop {
        let Some(parent) = maybe_parent else {
            panic!("Failed to find src_prefix {src_prefix:?} in {file_path:?}")
        };
        if parent.ends_with(src_prefix) {
            break file_path.strip_prefix(parent).unwrap();
        }
        maybe_parent = parent.parent();
    };
    let asset_path = after_src.parent().unwrap().join(asset_path);
    Path::new(crate_name).join(asset_path)
}

/// Creates a new `embedded` asset by embedding the bytes of the given path into the current binary
/// and registering those bytes with the `embedded` [`AssetSource`].
///
/// This accepts the current [`App`] as the first parameter and a path `&str` (relative to the current file) as the second.
///
/// By default this will generate an [`AssetPath`] using the following rules:
///
/// 1. Search for the first `$crate_name/src/` in the path and trim to the path past that point.
/// 2. Re-add the current `$crate_name` to the front of the path
///
/// For example, consider the following file structure in the theoretical `bevy_rock` crate, which provides a Bevy [`Plugin`](bevy_app::Plugin)
/// that renders fancy rocks for scenes.
///
/// ```text
/// bevy_rock
/// ├── src
/// │   ├── render
/// │   │   ├── rock.wgsl
/// │   │   └── mod.rs
/// │   └── lib.rs
/// └── Cargo.toml
/// ```
///
/// `rock.wgsl` is a WGSL shader asset that the `bevy_rock` plugin author wants to bundle with their crate. They invoke the following
/// in `bevy_rock/src/render/mod.rs`:
///
/// `embedded_asset!(app, "rock.wgsl")`
///
/// `rock.wgsl` can now be loaded by the [`AssetServer`] as follows:
///
/// ```no_run
/// # use bevy_asset::{Asset, AssetServer, load_embedded_asset};
/// # use bevy_reflect::TypePath;
/// # let asset_server: AssetServer = panic!();
/// # #[derive(Asset, TypePath)]
/// # struct Shader;
/// // If we are loading the shader in the same module we used `embedded_asset!`:
/// let shader = load_embedded_asset!(&asset_server, "rock.wgsl");
/// # let _: bevy_asset::Handle<Shader> = shader;
///
/// // If the goal is to expose the asset **to the end user**:
/// let shader = asset_server.load::<Shader>("embedded://bevy_rock/render/rock.wgsl");
/// ```
///
/// Some things to note in the path:
/// 1. The non-default `embedded://` [`AssetSource`]
/// 2. `src` is trimmed from the path
///
/// The default behavior also works for cargo workspaces. Pretend the `bevy_rock` crate now exists in a larger workspace in
/// `$SOME_WORKSPACE/crates/bevy_rock`. The asset path would remain the same, because [`embedded_asset`] searches for the
/// _first instance_ of `bevy_rock/src` in the path.
///
/// For most "standard crate structures" the default works just fine. But for some niche cases (such as cargo examples),
/// the `src` path will not be present. You can override this behavior by adding it as the second argument to [`embedded_asset`]:
///
/// `embedded_asset!(app, "/examples/rock_stuff/", "rock.wgsl")`
///
/// When there are three arguments, the second argument will replace the default `/src/` value. Note that these two are
/// equivalent:
///
/// `embedded_asset!(app, "rock.wgsl")`
/// `embedded_asset!(app, "/src/", "rock.wgsl")`
///
/// This macro uses the [`include_bytes`] macro internally and _will not_ reallocate the bytes.
/// Generally the [`AssetPath`] generated will be predictable, but if your asset isn't
/// available for some reason, you can use the [`embedded_path`] macro to debug.
///
/// Hot-reloading `embedded` assets is supported. Just enable the `embedded_watcher` cargo feature.
///
/// [`AssetPath`]: crate::AssetPath
/// [`embedded_asset`]: crate::embedded_asset
/// [`embedded_path`]: crate::embedded_path
#[macro_export]
macro_rules! embedded_asset {
    ($app: expr, $path: expr) => {{
        $crate::embedded_asset!($app, "src", $path)
    }};

    ($app: expr, $source_path: expr, $path: expr) => {{
        let mut embedded = $app
            .world_mut()
            .resource_mut::<$crate::io::embedded::EmbeddedAssetRegistry>();
        let path = $crate::embedded_path!($source_path, $path);
        let watched_path = $crate::io::embedded::watched_path(file!(), $path);
        embedded.insert_asset(watched_path, &path, include_bytes!($path));
    }};
}

/// Returns the path used by the watcher.
#[doc(hidden)]
#[cfg(feature = "embedded_watcher")]
pub fn watched_path(source_file_path: &'static str, asset_path: &'static str) -> PathBuf {
    PathBuf::from(source_file_path)
        .parent()
        .unwrap()
        .join(asset_path)
}

/// Returns an empty PathBuf.
#[doc(hidden)]
#[cfg(not(feature = "embedded_watcher"))]
pub fn watched_path(_source_file_path: &'static str, _asset_path: &'static str) -> PathBuf {
    PathBuf::from("")
}

/// Loads an "internal" asset by embedding the string stored in the given `path_str` and associates it with the given handle.
#[macro_export]
macro_rules! load_internal_asset {
    ($app: ident, $handle: expr, $path_str: expr, $loader: expr) => {{
        let mut assets = $app.world_mut().resource_mut::<$crate::Assets<_>>();
        assets.insert($handle.id(), ($loader)(
            include_str!($path_str),
            std::path::Path::new(file!())
                .parent()
                .unwrap()
                .join($path_str)
                .to_string_lossy()
        )).unwrap();
    }};
    // we can't support params without variadic arguments, so internal assets with additional params can't be hot-reloaded
    ($app: ident, $handle: ident, $path_str: expr, $loader: expr $(, $param:expr)+) => {{
        let mut assets = $app.world_mut().resource_mut::<$crate::Assets<_>>();
        assets.insert($handle.id(), ($loader)(
            include_str!($path_str),
            std::path::Path::new(file!())
                .parent()
                .unwrap()
                .join($path_str)
                .to_string_lossy(),
            $($param),+
        )).unwrap();
    }};
}

/// Loads an "internal" binary asset by embedding the bytes stored in the given `path_str` and associates it with the given handle.
#[macro_export]
macro_rules! load_internal_binary_asset {
    ($app: ident, $handle: expr, $path_str: expr, $loader: expr) => {{
        let mut assets = $app.world_mut().resource_mut::<$crate::Assets<_>>();
        assets
            .insert(
                $handle.id(),
                ($loader)(
                    include_bytes!($path_str).as_ref(),
                    std::path::Path::new(file!())
                        .parent()
                        .unwrap()
                        .join($path_str)
                        .to_string_lossy()
                        .into(),
                ),
            )
            .unwrap();
    }};
}

#[cfg(test)]
mod tests {
    use super::{EmbeddedAssetRegistry, _embedded_asset_path};
    use std::path::Path;

    // Relative paths show up if this macro is being invoked by a local crate.
    // In this case we know the relative path is a sub- path of the workspace
    // root.

    #[test]
    fn embedded_asset_path_from_local_crate() {
        let asset_path = _embedded_asset_path(
            "my_crate",
            "src".as_ref(),
            "src/foo/plugin.rs".as_ref(),
            "the/asset.png".as_ref(),
        );
        assert_eq!(asset_path, Path::new("my_crate/foo/the/asset.png"));
    }

    // A blank src_path removes the embedded's file path altogether only the
    // asset path remains.
    #[test]
    fn embedded_asset_path_from_local_crate_blank_src_path_questionable() {
        let asset_path = _embedded_asset_path(
            "my_crate",
            "".as_ref(),
            "src/foo/some/deep/path/plugin.rs".as_ref(),
            "the/asset.png".as_ref(),
        );
        assert_eq!(asset_path, Path::new("my_crate/the/asset.png"));
    }

    #[test]
    #[should_panic(expected = "Failed to find src_prefix \"NOT-THERE\" in \"src")]
    fn embedded_asset_path_from_local_crate_bad_src() {
        let _asset_path = _embedded_asset_path(
            "my_crate",
            "NOT-THERE".as_ref(),
            "src/foo/plugin.rs".as_ref(),
            "the/asset.png".as_ref(),
        );
    }

    #[test]
    fn embedded_asset_path_from_local_example_crate() {
        let asset_path = _embedded_asset_path(
            "example_name",
            "examples/foo".as_ref(),
            "examples/foo/example.rs".as_ref(),
            "the/asset.png".as_ref(),
        );
        assert_eq!(asset_path, Path::new("example_name/the/asset.png"));
    }

    // Absolute paths show up if this macro is being invoked by an external
    // dependency, e.g. one that's being checked out from a crates repo or git.
    #[test]
    fn embedded_asset_path_from_external_crate() {
        let asset_path = _embedded_asset_path(
            "my_crate",
            "src".as_ref(),
            "/path/to/crate/src/foo/plugin.rs".as_ref(),
            "the/asset.png".as_ref(),
        );
        assert_eq!(asset_path, Path::new("my_crate/foo/the/asset.png"));
    }

    #[test]
    fn embedded_asset_path_from_external_crate_root_src_path() {
        let asset_path = _embedded_asset_path(
            "my_crate",
            "/path/to/crate/src".as_ref(),
            "/path/to/crate/src/foo/plugin.rs".as_ref(),
            "the/asset.png".as_ref(),
        );
        assert_eq!(asset_path, Path::new("my_crate/foo/the/asset.png"));
    }

    // Although extraneous slashes are permitted at the end, e.g., "src////",
    // one or more slashes at the beginning are not.
    #[test]
    #[should_panic(expected = "Failed to find src_prefix \"////src\" in")]
    fn embedded_asset_path_from_external_crate_extraneous_beginning_slashes() {
        let asset_path = _embedded_asset_path(
            "my_crate",
            "////src".as_ref(),
            "/path/to/crate/src/foo/plugin.rs".as_ref(),
            "the/asset.png".as_ref(),
        );
        assert_eq!(asset_path, Path::new("my_crate/foo/the/asset.png"));
    }

    // We don't handle this edge case because it is ambiguous with the
    // information currently available to the embedded_path macro.
    #[test]
    fn embedded_asset_path_from_external_crate_is_ambiguous() {
        let asset_path = _embedded_asset_path(
            "my_crate",
            "src".as_ref(),
            "/path/to/.cargo/registry/src/crate/src/src/plugin.rs".as_ref(),
            "the/asset.png".as_ref(),
        );
        // Really, should be "my_crate/src/the/asset.png"
        assert_eq!(asset_path, Path::new("my_crate/the/asset.png"));
    }

    #[test]
    fn remove_embedded_asset() {
        let reg = EmbeddedAssetRegistry::default();
        let path = std::path::PathBuf::from("a/b/asset.png");
        reg.insert_asset(path.clone(), &path, &[]);
        assert!(reg.dir.get_asset(&path).is_some());
        assert!(reg.remove_asset(&path).is_some());
        assert!(reg.dir.get_asset(&path).is_none());
        assert!(reg.remove_asset(&path).is_none());
    }
}
