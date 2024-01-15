#[cfg(feature = "embedded_watcher")]
mod embedded_watcher;

#[cfg(feature = "embedded_watcher")]
pub use embedded_watcher::*;

use crate::io::{
    memory::{Dir, MemoryAssetReader, Value},
    AssetSource, AssetSourceBuilders,
};
use bevy_ecs::system::Resource;
use std::path::{Path, PathBuf};

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
    root_paths: std::sync::Arc<parking_lot::RwLock<bevy_utils::HashMap<PathBuf, PathBuf>>>,
}

impl EmbeddedAssetRegistry {
    /// Inserts a new asset. `full_path` is the full path (as [`file`] would return for that file, if it was capable of
    /// running in a non-rust file). `asset_path` is the path that will be used to identify the asset in the `embedded`
    /// [`AssetSource`]. `value` is the bytes that will be returned for the asset. This can be _either_ a `&'static [u8]`
    /// or a [`Vec<u8>`].
    #[allow(unused)]
    pub fn insert_asset(&self, full_path: PathBuf, asset_path: &Path, value: impl Into<Value>) {
        #[cfg(feature = "embedded_watcher")]
        self.root_paths
            .write()
            .insert(full_path.to_owned(), asset_path.to_owned());
        self.dir.insert_asset(asset_path, value);
    }

    /// Inserts new asset metadata. `full_path` is the full path (as [`file`] would return for that file, if it was capable of
    /// running in a non-rust file). `asset_path` is the path that will be used to identify the asset in the `embedded`
    /// [`AssetSource`]. `value` is the bytes that will be returned for the asset. This can be _either_ a `&'static [u8]`
    /// or a [`Vec<u8>`].
    #[allow(unused)]
    pub fn insert_meta(&self, full_path: &Path, asset_path: &Path, value: impl Into<Value>) {
        #[cfg(feature = "embedded_watcher")]
        self.root_paths
            .write()
            .insert(full_path.to_owned(), asset_path.to_owned());
        self.dir.insert_meta(asset_path, value);
    }

    /// Registers a `embedded` [`AssetSource`] that uses this [`EmbeddedAssetRegistry`].
    // NOTE: unused_mut because embedded_watcher feature is the only mutable consumer of `let mut source`
    #[allow(unused_mut)]
    pub fn register_source(&self, sources: &mut AssetSourceBuilders) {
        let dir = self.dir.clone();
        let processed_dir = self.dir.clone();
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
            let processd_dir = self.dir.clone();
            source = source
                .with_watcher(move |sender| {
                    Some(Box::new(EmbeddedWatcher::new(
                        dir.clone(),
                        root_paths.clone(),
                        sender,
                        std::time::Duration::from_millis(300),
                    )))
                })
                .with_processed_watcher(move |sender| {
                    Some(Box::new(EmbeddedWatcher::new(
                        processd_dir.clone(),
                        processed_root_paths.clone(),
                        sender,
                        std::time::Duration::from_millis(300),
                    )))
                });
        }
        sources.insert(EMBEDDED, source);
    }
}

/// Returns the [`Path`] for a given `embedded` asset.
/// This is used internally by [`embedded_asset`] and can be used to get a [`Path`]
/// that matches the [`AssetPath`](crate::AssetPath) used by that asset.
///
/// [`embedded_asset`]: crate::embedded_asset
#[macro_export]
macro_rules! embedded_path {
    ($path_str: expr) => {{
        embedded_path!("/src/", $path_str)
    }};

    ($source_path: expr, $path_str: expr) => {{
        let crate_name = module_path!().split(':').next().unwrap();
        let after_src = file!().split($source_path).nth(1).unwrap();
        let file_path = std::path::Path::new(after_src)
            .parent()
            .unwrap()
            .join($path_str);
        std::path::Path::new(crate_name).join(file_path)
    }};
}

/// Creates a new `embedded` asset by embedding the bytes of the given path into the current binary
/// and registering those bytes with the `embedded` [`AssetSource`].
///
/// This accepts the current [`App`](bevy_app::App) as the first parameter and a path `&str` (relative to the current file) as the second.
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
/// `rock.wgsl` can now be loaded by the [`AssetServer`](crate::AssetServer) with the following path:
///
/// ```no_run
/// # use bevy_asset::{Asset, AssetServer};
/// # use bevy_reflect::TypePath;
/// # let asset_server: AssetServer = panic!();
/// #[derive(Asset, TypePath)]
/// # struct Shader;
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
    ($app: ident, $path: expr) => {{
        embedded_asset!($app, "/src/", $path)
    }};

    ($app: ident, $source_path: expr, $path: expr) => {{
        let mut embedded = $app
            .world
            .resource_mut::<$crate::io::embedded::EmbeddedAssetRegistry>();
        let path = $crate::embedded_path!($source_path, $path);
        #[cfg(feature = "embedded_watcher")]
        let full_path = std::path::Path::new(file!()).parent().unwrap().join($path);
        #[cfg(not(feature = "embedded_watcher"))]
        let full_path = std::path::PathBuf::new();
        embedded.insert_asset(full_path, &path, include_bytes!($path));
    }};
}

/// Loads an "internal" asset by embedding the string stored in the given `path_str` and associates it with the given handle.
#[macro_export]
macro_rules! load_internal_asset {
    ($app: ident, $handle: expr, $path_str: expr, $loader: expr) => {{
        let mut assets = $app.world.resource_mut::<$crate::Assets<_>>();
        assets.insert($handle, ($loader)(
            include_str!($path_str),
            std::path::Path::new(file!())
                .parent()
                .unwrap()
                .join($path_str)
                .to_string_lossy()
        ));
    }};
    // we can't support params without variadic arguments, so internal assets with additional params can't be hot-reloaded
    ($app: ident, $handle: ident, $path_str: expr, $loader: expr $(, $param:expr)+) => {{
        let mut assets = $app.world.resource_mut::<$crate::Assets<_>>();
        assets.insert($handle, ($loader)(
            include_str!($path_str),
            std::path::Path::new(file!())
                .parent()
                .unwrap()
                .join($path_str)
                .to_string_lossy(),
            $($param),+
        ));
    }};
}

/// Loads an "internal" binary asset by embedding the bytes stored in the given `path_str` and associates it with the given handle.
#[macro_export]
macro_rules! load_internal_binary_asset {
    ($app: ident, $handle: expr, $path_str: expr, $loader: expr) => {{
        let mut assets = $app.world.resource_mut::<$crate::Assets<_>>();
        assets.insert(
            $handle,
            ($loader)(
                include_bytes!($path_str).as_ref(),
                std::path::Path::new(file!())
                    .parent()
                    .unwrap()
                    .join($path_str)
                    .to_string_lossy()
                    .into(),
            ),
        );
    }};
}
