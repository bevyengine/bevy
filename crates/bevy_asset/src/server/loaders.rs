use crate::{
    loader::{AssetLoader, ErasedAssetLoader},
    path::AssetPath,
};
use async_broadcast::RecvError;
use bevy_tasks::IoTaskPool;
use bevy_utils::tracing::{error, warn};
#[cfg(feature = "trace")]
use bevy_utils::{
    tracing::{info_span, instrument::Instrument},
    ConditionalSendFuture,
};
use bevy_utils::{HashMap, TypeIdMap};
use std::{any::TypeId, sync::Arc};
use thiserror::Error;

#[derive(Default)]
pub(crate) struct AssetLoaders {
    loaders: Vec<MaybeAssetLoader>,
    type_id_to_loaders: TypeIdMap<Vec<usize>>,
    extension_to_loaders: HashMap<Box<str>, Vec<usize>>,
    type_name_to_loader: HashMap<&'static str, usize>,
    preregistered_loaders: HashMap<&'static str, usize>,
}

impl AssetLoaders {
    /// Get the [`AssetLoader`] stored at the specific index
    fn get_by_index(&self, index: usize) -> Option<MaybeAssetLoader> {
        self.loaders.get(index).cloned()
    }

    /// Registers a new [`AssetLoader`]. [`AssetLoader`]s must be registered before they can be used.
    pub(crate) fn push<L: AssetLoader>(&mut self, loader: L) {
        let type_name = std::any::type_name::<L>();
        let loader_asset_type = TypeId::of::<L::Asset>();
        let loader_asset_type_name = std::any::type_name::<L::Asset>();

        #[cfg(feature = "trace")]
        let loader = InstrumentedAssetLoader(loader);
        let loader = Arc::new(loader);

        let (loader_index, is_new) =
            if let Some(index) = self.preregistered_loaders.remove(type_name) {
                (index, false)
            } else {
                (self.loaders.len(), true)
            };

        if is_new {
            let mut duplicate_extensions = Vec::new();
            for extension in AssetLoader::extensions(&*loader) {
                let list = self
                    .extension_to_loaders
                    .entry((*extension).into())
                    .or_default();

                if !list.is_empty() {
                    duplicate_extensions.push(extension);
                }

                list.push(loader_index);
            }

            self.type_name_to_loader.insert(type_name, loader_index);

            let list = self
                .type_id_to_loaders
                .entry(loader_asset_type)
                .or_default();

            let duplicate_asset_registration = !list.is_empty();
            if !duplicate_extensions.is_empty() && duplicate_asset_registration {
                warn!("Duplicate AssetLoader registered for Asset type `{loader_asset_type_name}` with extensions `{duplicate_extensions:?}`. \
                Loader must be specified in a .meta file in order to load assets of this type with these extensions.");
            }

            list.push(loader_index);

            self.loaders.push(MaybeAssetLoader::Ready(loader));
        } else {
            let maybe_loader = std::mem::replace(
                self.loaders.get_mut(loader_index).unwrap(),
                MaybeAssetLoader::Ready(loader.clone()),
            );
            match maybe_loader {
                MaybeAssetLoader::Ready(_) => unreachable!(),
                MaybeAssetLoader::Pending { sender, .. } => {
                    IoTaskPool::get()
                        .spawn(async move {
                            let _ = sender.broadcast(loader).await;
                        })
                        .detach();
                }
            }
        }
    }

    /// Pre-register an [`AssetLoader`] that will later be added.
    ///
    /// Assets loaded with matching extensions will be blocked until the
    /// real loader is added.
    pub(crate) fn reserve<L: AssetLoader>(&mut self, extensions: &[&str]) {
        let loader_asset_type = TypeId::of::<L::Asset>();
        let loader_asset_type_name = std::any::type_name::<L::Asset>();
        let type_name = std::any::type_name::<L>();

        let loader_index = self.loaders.len();

        self.preregistered_loaders.insert(type_name, loader_index);
        self.type_name_to_loader.insert(type_name, loader_index);
        let mut duplicate_extensions = Vec::new();
        for extension in extensions {
            let list = self
                .extension_to_loaders
                .entry((*extension).into())
                .or_default();

            if !list.is_empty() {
                duplicate_extensions.push(extension);
            }

            list.push(loader_index);
        }

        let list = self
            .type_id_to_loaders
            .entry(loader_asset_type)
            .or_default();

        let duplicate_asset_registration = !list.is_empty();
        if !duplicate_extensions.is_empty() && duplicate_asset_registration {
            warn!("Duplicate AssetLoader preregistered for Asset type `{loader_asset_type_name}` with extensions `{duplicate_extensions:?}`. \
            Loader must be specified in a .meta file in order to load assets of this type with these extensions.");
        }

        list.push(loader_index);

        let (mut sender, receiver) = async_broadcast::broadcast(1);
        sender.set_overflow(true);
        self.loaders
            .push(MaybeAssetLoader::Pending { sender, receiver });
    }

    /// Get the [`AssetLoader`] by name
    pub(crate) fn get_by_name(&self, name: &str) -> Option<MaybeAssetLoader> {
        let index = self.type_name_to_loader.get(name).copied()?;

        self.get_by_index(index)
    }

    /// Find an [`AssetLoader`] based on provided search criteria
    pub(crate) fn find(
        &self,
        type_name: Option<&str>,
        asset_type_id: Option<TypeId>,
        extension: Option<&str>,
        asset_path: Option<&AssetPath<'_>>,
    ) -> Option<MaybeAssetLoader> {
        // If provided the type name of the loader, return that immediately
        if let Some(type_name) = type_name {
            return self.get_by_name(type_name);
        }

        // The presence of a label will affect loader choice
        let label = asset_path.as_ref().and_then(|path| path.label());

        // Try by asset type
        let candidates = if let Some(type_id) = asset_type_id {
            if label.is_none() {
                Some(self.type_id_to_loaders.get(&type_id)?)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(candidates) = candidates {
            if candidates.is_empty() {
                return None;
            } else if candidates.len() == 1 {
                let index = candidates.first().copied().unwrap();
                return self.get_by_index(index);
            }
        }

        // Asset type is insufficient, use extension information
        let try_extension = |extension| {
            if let Some(indices) = self.extension_to_loaders.get(extension) {
                if let Some(candidates) = candidates {
                    if candidates.is_empty() {
                        indices.last()
                    } else {
                        indices
                            .iter()
                            .rev()
                            .find(|index| candidates.contains(index))
                    }
                } else {
                    indices.last()
                }
            } else {
                None
            }
        };

        // Try the provided extension
        if let Some(extension) = extension {
            if let Some(&index) = try_extension(extension) {
                return self.get_by_index(index);
            }
        }

        // Try extracting the extension from the path
        if let Some(full_extension) = asset_path.and_then(|path| path.get_full_extension()) {
            if let Some(&index) = try_extension(full_extension.as_str()) {
                return self.get_by_index(index);
            }

            // Try secondary extensions from the path
            for extension in AssetPath::iter_secondary_extensions(&full_extension) {
                if let Some(&index) = try_extension(extension) {
                    return self.get_by_index(index);
                }
            }
        }

        // Fallback if no resolution step was conclusive
        match candidates?
            .last()
            .copied()
            .and_then(|index| self.get_by_index(index))
        {
            Some(loader) => {
                warn!(
                    "Multiple AssetLoaders found for Asset: {:?}; Path: {:?}; Extension: {:?}",
                    asset_type_id, asset_path, extension
                );
                Some(loader)
            }
            None => {
                warn!(
                    "No AssetLoader found for Asset: {:?}; Path: {:?}; Extension: {:?}",
                    asset_type_id, asset_path, extension
                );
                None
            }
        }
    }

    /// Get the [`AssetLoader`] for a given asset type
    pub(crate) fn get_by_type(&self, type_id: TypeId) -> Option<MaybeAssetLoader> {
        let index = self.type_id_to_loaders.get(&type_id)?.last().copied()?;

        self.get_by_index(index)
    }

    /// Get the [`AssetLoader`] for a given extension
    pub(crate) fn get_by_extension(&self, extension: &str) -> Option<MaybeAssetLoader> {
        let index = self.extension_to_loaders.get(extension)?.last().copied()?;

        self.get_by_index(index)
    }

    /// Get the [`AssetLoader`] for a given path
    pub(crate) fn get_by_path(&self, path: &AssetPath<'_>) -> Option<MaybeAssetLoader> {
        let extension = path.get_full_extension()?;

        let result = std::iter::once(extension.as_str())
            .chain(AssetPath::iter_secondary_extensions(&extension))
            .filter_map(|extension| self.extension_to_loaders.get(extension)?.last().copied())
            .find_map(|index| self.get_by_index(index))?;

        Some(result)
    }
}

#[derive(Error, Debug, Clone)]
pub(crate) enum GetLoaderError {
    #[error(transparent)]
    CouldNotResolve(#[from] RecvError),
}

#[derive(Clone)]
pub(crate) enum MaybeAssetLoader {
    Ready(Arc<dyn ErasedAssetLoader>),
    Pending {
        sender: async_broadcast::Sender<Arc<dyn ErasedAssetLoader>>,
        receiver: async_broadcast::Receiver<Arc<dyn ErasedAssetLoader>>,
    },
}

impl MaybeAssetLoader {
    pub(crate) async fn get(self) -> Result<Arc<dyn ErasedAssetLoader>, GetLoaderError> {
        match self {
            MaybeAssetLoader::Ready(loader) => Ok(loader),
            MaybeAssetLoader::Pending { mut receiver, .. } => Ok(receiver.recv().await?),
        }
    }
}

#[cfg(feature = "trace")]
struct InstrumentedAssetLoader<T>(T);

#[cfg(feature = "trace")]
impl<T: AssetLoader> AssetLoader for InstrumentedAssetLoader<T> {
    type Asset = T::Asset;
    type Settings = T::Settings;
    type Error = T::Error;

    fn load<'a>(
        &'a self,
        reader: &'a mut crate::io::Reader,
        settings: &'a Self::Settings,
        load_context: &'a mut crate::LoadContext,
    ) -> impl ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        let span = info_span!(
            "asset loading",
            loader = std::any::type_name::<T>(),
            asset = load_context.asset_path().to_string(),
        );
        self.0.load(reader, settings, load_context).instrument(span)
    }

    fn extensions(&self) -> &[&str] {
        self.0.extensions()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        marker::PhantomData,
        path::Path,
        sync::mpsc::{channel, Receiver, Sender},
    };

    use bevy_reflect::TypePath;
    use bevy_tasks::block_on;

    use crate::{self as bevy_asset, Asset};

    use super::*;

    // The compiler notices these fields are never read and raises a dead_code lint which kill CI.
    #[allow(dead_code)]
    #[derive(Asset, TypePath, Debug)]
    struct A(usize);

    #[allow(dead_code)]
    #[derive(Asset, TypePath, Debug)]
    struct B(usize);

    #[allow(dead_code)]
    #[derive(Asset, TypePath, Debug)]
    struct C(usize);

    struct Loader<A: Asset, const N: usize, const E: usize> {
        sender: Sender<()>,
        _phantom: PhantomData<A>,
    }

    impl<T: Asset, const N: usize, const E: usize> Loader<T, N, E> {
        fn new() -> (Self, Receiver<()>) {
            let (tx, rx) = channel();

            let loader = Self {
                sender: tx,
                _phantom: PhantomData,
            };

            (loader, rx)
        }
    }

    impl<T: Asset, const N: usize, const E: usize> AssetLoader for Loader<T, N, E> {
        type Asset = T;

        type Settings = ();

        type Error = String;

        async fn load<'a>(
            &'a self,
            _: &'a mut crate::io::Reader<'_>,
            _: &'a Self::Settings,
            _: &'a mut crate::LoadContext<'_>,
        ) -> Result<Self::Asset, Self::Error> {
            self.sender.send(()).unwrap();

            Err(format!(
                "Loaded {}:{}",
                std::any::type_name::<Self::Asset>(),
                N
            ))
        }

        fn extensions(&self) -> &[&str] {
            self.sender.send(()).unwrap();

            match E {
                1 => &["a"],
                2 => &["b"],
                3 => &["c"],
                4 => &["d"],
                _ => &[],
            }
        }
    }

    /// Basic framework for creating, storing, loading, and checking an [`AssetLoader`] inside an [`AssetLoaders`]
    #[test]
    fn basic() {
        let mut loaders = AssetLoaders::default();

        let (loader, rx) = Loader::<A, 1, 0>::new();

        assert!(rx.try_recv().is_err());

        loaders.push(loader);

        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_err());

        let loader = block_on(
            loaders
                .get_by_name(std::any::type_name::<Loader<A, 1, 0>>())
                .unwrap()
                .get(),
        )
        .unwrap();

        loader.extensions();

        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_err());
    }

    /// Ensure that if multiple loaders have different types but no extensions, they can be found
    #[test]
    fn type_resolution() {
        let mut loaders = AssetLoaders::default();

        let (loader_a1, rx_a1) = Loader::<A, 1, 0>::new();
        let (loader_b1, rx_b1) = Loader::<B, 1, 0>::new();
        let (loader_c1, rx_c1) = Loader::<C, 1, 0>::new();

        loaders.push(loader_a1);
        loaders.push(loader_b1);
        loaders.push(loader_c1);

        assert!(rx_a1.try_recv().is_ok());
        assert!(rx_b1.try_recv().is_ok());
        assert!(rx_c1.try_recv().is_ok());

        let loader = block_on(loaders.get_by_type(TypeId::of::<A>()).unwrap().get()).unwrap();

        loader.extensions();

        assert!(rx_a1.try_recv().is_ok());
        assert!(rx_b1.try_recv().is_err());
        assert!(rx_c1.try_recv().is_err());

        let loader = block_on(loaders.get_by_type(TypeId::of::<B>()).unwrap().get()).unwrap();

        loader.extensions();

        assert!(rx_a1.try_recv().is_err());
        assert!(rx_b1.try_recv().is_ok());
        assert!(rx_c1.try_recv().is_err());

        let loader = block_on(loaders.get_by_type(TypeId::of::<C>()).unwrap().get()).unwrap();

        loader.extensions();

        assert!(rx_a1.try_recv().is_err());
        assert!(rx_b1.try_recv().is_err());
        assert!(rx_c1.try_recv().is_ok());
    }

    /// Ensure that the last loader added is selected
    #[test]
    fn type_resolution_shadow() {
        let mut loaders = AssetLoaders::default();

        let (loader_a1, rx_a1) = Loader::<A, 1, 0>::new();
        let (loader_a2, rx_a2) = Loader::<A, 2, 0>::new();
        let (loader_a3, rx_a3) = Loader::<A, 3, 0>::new();

        loaders.push(loader_a1);
        loaders.push(loader_a2);
        loaders.push(loader_a3);

        assert!(rx_a1.try_recv().is_ok());
        assert!(rx_a2.try_recv().is_ok());
        assert!(rx_a3.try_recv().is_ok());

        let loader = block_on(loaders.get_by_type(TypeId::of::<A>()).unwrap().get()).unwrap();

        loader.extensions();

        assert!(rx_a1.try_recv().is_err());
        assert!(rx_a2.try_recv().is_err());
        assert!(rx_a3.try_recv().is_ok());
    }

    /// Ensure that if multiple loaders have like types but differing extensions, they can be found
    #[test]
    fn extension_resolution() {
        let mut loaders = AssetLoaders::default();

        let (loader_a1, rx_a1) = Loader::<A, 1, 1>::new();
        let (loader_b1, rx_b1) = Loader::<A, 1, 2>::new();
        let (loader_c1, rx_c1) = Loader::<A, 1, 3>::new();

        loaders.push(loader_a1);
        loaders.push(loader_b1);
        loaders.push(loader_c1);

        assert!(rx_a1.try_recv().is_ok());
        assert!(rx_b1.try_recv().is_ok());
        assert!(rx_c1.try_recv().is_ok());

        let loader = block_on(loaders.get_by_extension("a").unwrap().get()).unwrap();

        loader.extensions();

        assert!(rx_a1.try_recv().is_ok());
        assert!(rx_b1.try_recv().is_err());
        assert!(rx_c1.try_recv().is_err());

        let loader = block_on(loaders.get_by_extension("b").unwrap().get()).unwrap();

        loader.extensions();

        assert!(rx_a1.try_recv().is_err());
        assert!(rx_b1.try_recv().is_ok());
        assert!(rx_c1.try_recv().is_err());

        let loader = block_on(loaders.get_by_extension("c").unwrap().get()).unwrap();

        loader.extensions();

        assert!(rx_a1.try_recv().is_err());
        assert!(rx_b1.try_recv().is_err());
        assert!(rx_c1.try_recv().is_ok());
    }

    /// Ensure that if multiple loaders have like types but differing extensions, they can be found
    #[test]
    fn path_resolution() {
        let mut loaders = AssetLoaders::default();

        let (loader_a1, rx_a1) = Loader::<A, 1, 1>::new();
        let (loader_b1, rx_b1) = Loader::<A, 1, 2>::new();
        let (loader_c1, rx_c1) = Loader::<A, 1, 3>::new();

        loaders.push(loader_a1);
        loaders.push(loader_b1);
        loaders.push(loader_c1);

        assert!(rx_a1.try_recv().is_ok());
        assert!(rx_b1.try_recv().is_ok());
        assert!(rx_c1.try_recv().is_ok());

        let path = AssetPath::from_path(Path::new("asset.a"));

        let loader = block_on(loaders.get_by_path(&path).unwrap().get()).unwrap();

        loader.extensions();

        assert!(rx_a1.try_recv().is_ok());
        assert!(rx_b1.try_recv().is_err());
        assert!(rx_c1.try_recv().is_err());

        let path = AssetPath::from_path(Path::new("asset.b"));

        let loader = block_on(loaders.get_by_path(&path).unwrap().get()).unwrap();

        loader.extensions();

        assert!(rx_a1.try_recv().is_err());
        assert!(rx_b1.try_recv().is_ok());
        assert!(rx_c1.try_recv().is_err());

        let path = AssetPath::from_path(Path::new("asset.c"));

        let loader = block_on(loaders.get_by_path(&path).unwrap().get()).unwrap();

        loader.extensions();

        assert!(rx_a1.try_recv().is_err());
        assert!(rx_b1.try_recv().is_err());
        assert!(rx_c1.try_recv().is_ok());
    }

    /// Full resolution algorithm
    #[test]
    fn total_resolution() {
        let mut loaders = AssetLoaders::default();

        let (loader_a1_a, rx_a1_a) = Loader::<A, 1, 1>::new();

        let (loader_b1_b, rx_b1_b) = Loader::<B, 1, 2>::new();

        let (loader_c1_a, rx_c1_a) = Loader::<C, 1, 1>::new();
        let (loader_c1_b, rx_c1_b) = Loader::<C, 1, 2>::new();
        let (loader_c1_c, rx_c1_c) = Loader::<C, 1, 3>::new();

        loaders.push(loader_a1_a);
        loaders.push(loader_b1_b);
        loaders.push(loader_c1_a);
        loaders.push(loader_c1_b);
        loaders.push(loader_c1_c);

        assert!(rx_a1_a.try_recv().is_ok());
        assert!(rx_b1_b.try_recv().is_ok());
        assert!(rx_c1_a.try_recv().is_ok());
        assert!(rx_c1_b.try_recv().is_ok());
        assert!(rx_c1_c.try_recv().is_ok());

        // Type and Extension agree

        let loader = block_on(
            loaders
                .find(
                    None,
                    Some(TypeId::of::<A>()),
                    None,
                    Some(&AssetPath::from_path(Path::new("asset.a"))),
                )
                .unwrap()
                .get(),
        )
        .unwrap();

        loader.extensions();

        assert!(rx_a1_a.try_recv().is_ok());
        assert!(rx_b1_b.try_recv().is_err());
        assert!(rx_c1_a.try_recv().is_err());
        assert!(rx_c1_b.try_recv().is_err());
        assert!(rx_c1_c.try_recv().is_err());

        let loader = block_on(
            loaders
                .find(
                    None,
                    Some(TypeId::of::<B>()),
                    None,
                    Some(&AssetPath::from_path(Path::new("asset.b"))),
                )
                .unwrap()
                .get(),
        )
        .unwrap();

        loader.extensions();

        assert!(rx_a1_a.try_recv().is_err());
        assert!(rx_b1_b.try_recv().is_ok());
        assert!(rx_c1_a.try_recv().is_err());
        assert!(rx_c1_b.try_recv().is_err());
        assert!(rx_c1_c.try_recv().is_err());

        let loader = block_on(
            loaders
                .find(
                    None,
                    Some(TypeId::of::<C>()),
                    None,
                    Some(&AssetPath::from_path(Path::new("asset.c"))),
                )
                .unwrap()
                .get(),
        )
        .unwrap();

        loader.extensions();

        assert!(rx_a1_a.try_recv().is_err());
        assert!(rx_b1_b.try_recv().is_err());
        assert!(rx_c1_a.try_recv().is_err());
        assert!(rx_c1_b.try_recv().is_err());
        assert!(rx_c1_c.try_recv().is_ok());

        // Type should override Extension

        let loader = block_on(
            loaders
                .find(
                    None,
                    Some(TypeId::of::<C>()),
                    None,
                    Some(&AssetPath::from_path(Path::new("asset.a"))),
                )
                .unwrap()
                .get(),
        )
        .unwrap();

        loader.extensions();

        assert!(rx_a1_a.try_recv().is_err());
        assert!(rx_b1_b.try_recv().is_err());
        assert!(rx_c1_a.try_recv().is_ok());
        assert!(rx_c1_b.try_recv().is_err());
        assert!(rx_c1_c.try_recv().is_err());

        let loader = block_on(
            loaders
                .find(
                    None,
                    Some(TypeId::of::<C>()),
                    None,
                    Some(&AssetPath::from_path(Path::new("asset.b"))),
                )
                .unwrap()
                .get(),
        )
        .unwrap();

        loader.extensions();

        assert!(rx_a1_a.try_recv().is_err());
        assert!(rx_b1_b.try_recv().is_err());
        assert!(rx_c1_a.try_recv().is_err());
        assert!(rx_c1_b.try_recv().is_ok());
        assert!(rx_c1_c.try_recv().is_err());

        // Type should override bad / missing extension

        let loader = block_on(
            loaders
                .find(
                    None,
                    Some(TypeId::of::<A>()),
                    None,
                    Some(&AssetPath::from_path(Path::new("asset.x"))),
                )
                .unwrap()
                .get(),
        )
        .unwrap();

        loader.extensions();

        assert!(rx_a1_a.try_recv().is_ok());
        assert!(rx_b1_b.try_recv().is_err());
        assert!(rx_c1_a.try_recv().is_err());
        assert!(rx_c1_b.try_recv().is_err());
        assert!(rx_c1_c.try_recv().is_err());

        let loader = block_on(
            loaders
                .find(
                    None,
                    Some(TypeId::of::<A>()),
                    None,
                    Some(&AssetPath::from_path(Path::new("asset"))),
                )
                .unwrap()
                .get(),
        )
        .unwrap();

        loader.extensions();

        assert!(rx_a1_a.try_recv().is_ok());
        assert!(rx_b1_b.try_recv().is_err());
        assert!(rx_c1_a.try_recv().is_err());
        assert!(rx_c1_b.try_recv().is_err());
        assert!(rx_c1_c.try_recv().is_err());
    }

    /// Ensure that if there is a complete ambiguity in [`AssetLoader`] to use, prefer most recently registered by asset type.
    #[test]
    fn ambiguity_resolution() {
        let mut loaders = AssetLoaders::default();

        let (loader_a1_a, rx_a1_a) = Loader::<A, 1, 1>::new();
        let (loader_a2_a, rx_a2_a) = Loader::<A, 2, 1>::new();
        let (loader_a3_a, rx_a3_a) = Loader::<A, 3, 1>::new();

        loaders.push(loader_a1_a);
        loaders.push(loader_a2_a);
        loaders.push(loader_a3_a);

        assert!(rx_a1_a.try_recv().is_ok());
        assert!(rx_a2_a.try_recv().is_ok());
        assert!(rx_a3_a.try_recv().is_ok());

        let loader = block_on(
            loaders
                .find(
                    None,
                    Some(TypeId::of::<A>()),
                    None,
                    Some(&AssetPath::from_path(Path::new("asset.a"))),
                )
                .unwrap()
                .get(),
        )
        .unwrap();

        loader.extensions();

        assert!(rx_a1_a.try_recv().is_err());
        assert!(rx_a2_a.try_recv().is_err());
        assert!(rx_a3_a.try_recv().is_ok());

        let loader = block_on(
            loaders
                .find(
                    None,
                    Some(TypeId::of::<A>()),
                    None,
                    Some(&AssetPath::from_path(Path::new("asset.x"))),
                )
                .unwrap()
                .get(),
        )
        .unwrap();

        loader.extensions();

        assert!(rx_a1_a.try_recv().is_err());
        assert!(rx_a2_a.try_recv().is_err());
        assert!(rx_a3_a.try_recv().is_ok());

        let loader = block_on(
            loaders
                .find(
                    None,
                    Some(TypeId::of::<A>()),
                    None,
                    Some(&AssetPath::from_path(Path::new("asset"))),
                )
                .unwrap()
                .get(),
        )
        .unwrap();

        loader.extensions();

        assert!(rx_a1_a.try_recv().is_err());
        assert!(rx_a2_a.try_recv().is_err());
        assert!(rx_a3_a.try_recv().is_ok());
    }
}
