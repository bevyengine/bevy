use alloc::{borrow::Cow, string::String, vec, vec::Vec};
use bevy_ecs::{
    reflect::AppTypeRegistry,
    world::{FromWorld, World},
};
use core::marker::PhantomData;
use futures_lite::AsyncWriteExt;
use ron::ser::PrettyConfig;

use bevy_reflect::{
    serde::{ReflectDeserializer, ReflectSerializer},
    Reflect, ReflectFromPtr, TypePath, TypeRegistryArc,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

use crate::{
    io::Reader,
    saver::{save_using_saver, AssetSaver, SaveAssetError, SavedAsset, SavedAssetBuilder},
    Asset, AssetLoader, AssetPath, AssetServer, EphemeralHandleBehavior,
    HandleDeserializeProcessor, HandleSerializeProcessor, LoadContext, LoadedUntypedAsset,
    ReflectAsset,
};

/// A loader for reflected asset types written in the RON format.
///
/// This loader supports **any** reflected asset type in its registry. This loader expects assets to
/// be stored as a map with a single entry, where the key is the full type path of the reflected
/// type, and the value is the serialized data. For example, for the following type:
///
/// ```rust
/// # use bevy_reflect::Reflect;
/// # use bevy_asset::{Asset, ReflectAsset};
/// #[derive(Reflect, Asset)]
/// #[reflect(Asset)]
/// struct MyStruct {
///     value: i32,
/// }
/// ```
///
/// The serialized format would be:
///
/// ```ron
/// {
///     "MyStruct": (
///         value: 123
///     )
/// }
/// ```
///
/// Since this loader can load any reflectable type, the concrete asset type returned by this loader
/// is [`LoadedUntypedAsset`], which holds the actual asset handle. The actual asset data is loaded
/// as a subasset with label `#Typed`. In other words, if you want to load `"my_thing.ron"` as
/// `MyStruct` type, you need to call `asset_server.load::<MyStruct>("my_thing.ron#Typed")`. The
/// root asset being a [`LoadedUntypedAsset`] allows users to load as any type, and then change
/// their behavior based on the loaded type (e.g., if it's `MyStruct` do behavior 1, and if it's
/// `MyEnum` do behavior 2).
///
/// This loader requires that the held type implements **and** reflects [`Asset`].
///
/// This loader also supports loading asset types with [`Handle`]s to other assets (which will be
/// automatically loaded too).
///
/// Warning: When performing an untyped load using [`LoadBuilder::load_untyped`], load the `#Typed`
/// subasset. If you instead load the root asset, you will get a [`Handle<LoadedUntypedAsset>`]
/// which stores an [`UntypedHandle`] whose type ID is for [`LoadedUntypedAsset`], which then
/// internally stores your desired asset type.
///
/// [`Handle`]: crate::Handle
/// [`LoadBuilder::load_untyped`]: crate::LoadBuilder::load_untyped
/// [`Handle<LoadedUntypedAsset>`]: crate::Handle
/// [`UntypedHandle`]: crate::UntypedHandle
#[derive(TypePath, Clone)]
pub struct RonLoader {
    /// The extensions that this loader will load for.
    ///
    /// By default, this is set to `ron`.
    pub extensions: Cow<'static, [&'static str]>,
    /// The type registry that will be used when deserializing values.
    pub registry: TypeRegistryArc,
}

impl FromWorld for RonLoader {
    fn from_world(world: &mut World) -> Self {
        Self {
            extensions: (&["ron"]).into(),
            registry: world.resource::<AppTypeRegistry>().0.clone(),
        }
    }
}

impl AssetLoader for RonLoader {
    type Asset = LoadedUntypedAsset;
    type Settings = ();
    type Error = ReflectedRonDeserializeError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut buffer = vec![];
        reader
            .read_to_end(&mut buffer)
            .await
            .map_err(Into::<RonDeserializeError>::into)?;
        let registry = self.registry.read();

        // Create a subasset LoadContext, so that dependencies are tracked on the subasset, and not
        // just the root asset.
        let mut subasset_context = load_context.begin_labeled_asset();
        let mut handle_processor = HandleDeserializeProcessor {
            load_from_path: &mut subasset_context,
        };
        let reflect_deserializer =
            ReflectDeserializer::with_processor(&registry, &mut handle_processor);

        let reflected_asset = ron::Options::default()
            .from_bytes_seed(&buffer, reflect_deserializer)
            .map_err(Into::<RonDeserializeError>::into)?;

        // Unwrap is ok because the `ReflectDeserializer` will produce values representing a
        // particular type.
        let asset_type_info = reflected_asset.get_represented_type_info().unwrap();
        let asset_type_id = asset_type_info.type_id();
        // Unwrap is ok because the `ReflectDeserializer` could not have deserialized this value if
        // the type weren't in the registry.
        let type_registration = registry.get(asset_type_id).unwrap();
        let Some(reflect_asset) = type_registration.data::<ReflectAsset>() else {
            return Err(ReflectedRonDeserializeError::MissingReflectAsset(
                asset_type_info.type_path(),
            ));
        };

        // At this point, we need the concrete type. `ReflectDeserializer` internally uses the
        // `ReflectFromReflect` for the loaded type to convert into a concrete type. If that fails,
        // that must mean `ReflectFromReflect` wasn't registered for the type, so there's nothing we
        // can do.
        let Ok(reflected_asset) = reflected_asset.try_into_reflect() else {
            // We're kind of cheating here: the current implementation of `ReflectDeserializer` (at
            // the time of writing) only doesn't return the concrete type if `ReflectFromReflect` is
            // not registered. We return that here for better error messages (even though an
            // arbitrary implementation might not return the concrete type for other reasons, or
            // might try other fallbacks).
            return Err(ReflectedRonDeserializeError::MissingReflectFromReflect(
                asset_type_info.type_path(),
            ));
        };

        // Unwrap is ok because `finish_load_context` only fails if the Box<dyn Reflect> holds the
        // wrong type. This is only possible if someone creates ReflectAsset for type A and inserts
        // it into the registration of type B. We won't handle this "malicious" case.
        let loaded_asset = reflect_asset
            .finish_load_context(subasset_context, reflected_asset)
            .unwrap();

        let handle = load_context.add_erased_loaded_labeled_asset("Typed", loaded_asset);
        Ok(LoadedUntypedAsset { handle })
    }

    fn extensions(&self) -> &[&str] {
        &self.extensions
    }
}

/// A loader for loading serializable assets written in the RON format.
///
/// This is a typed counterpart to [`RonLoader`]. Therefore, this loader only supports a single
/// asset type `A` (rather than any reflectable type). This however allows the serialized format to
/// omit the type. For example, for the following type:
///
/// ```rust
/// # use bevy_reflect::TypePath;
/// # use bevy_asset::Asset;
/// # use serde::Deserialize;
/// #[derive(Asset, TypePath, Deserialize)]
/// struct MyStruct {
///     value: i32,
/// }
/// # bevy_asset::common_loaders::ron::TypedRonLoader::<MyStruct>::new(vec![]);
/// ```
///
/// The serialized format would be:
///
/// ```ron
/// (
///     value: 123
/// )
/// ```
///
/// Unlike [`RonLoader`], this loader does not support loading types with [`Handle`]s, since
/// [`Handle`]s cannot be serialized or deserialized. Consider using [`RonLoader`] for these types.
///
/// [`Handle`]: crate::Handle
#[derive(TypePath)]
pub struct TypedRonLoader<A: Asset + DeserializeOwned> {
    extensions: Vec<&'static str>,
    marker: PhantomData<fn() -> A>,
}

// Manual impl since the derive would add an `A: Clone` bound.
impl<A: Asset + DeserializeOwned> Clone for TypedRonLoader<A> {
    fn clone(&self) -> Self {
        Self {
            extensions: self.extensions.clone(),
            marker: PhantomData,
        }
    }
}

impl<A: Asset + DeserializeOwned> TypedRonLoader<A> {
    /// Creates a loader for this type that will use the given extensions.
    ///
    /// It is highly recommended to use a custom extension (e.g., `MyStruct.ron`, or
    /// `particle_effect`) and not just `ron`. Having multiple loaders that just use the `ron`
    /// extension would prevent untyped loads from working correctly (only the last such loader will
    /// be used for all loads).
    pub fn new(extensions: Vec<&'static str>) -> Self {
        Self {
            extensions,
            marker: PhantomData,
        }
    }
}

impl<A: Asset + DeserializeOwned> AssetLoader for TypedRonLoader<A> {
    type Asset = A;
    type Settings = ();
    type Error = RonDeserializeError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut buffer = vec![];
        reader.read_to_end(&mut buffer).await?;
        Ok(ron::de::from_bytes(&buffer)?)
    }

    fn extensions(&self) -> &[&str] {
        &self.extensions
    }
}

/// Settings for saving data in the RON format.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RonSaverSettings {
    /// The configuration for pretty-printing the RON data.
    ///
    /// The default is [`Some`], since it is likely that users will want to view or manually edit
    /// the RON after saving.
    pub pretty_config: Option<PrettyConfig>,
}

impl Default for RonSaverSettings {
    fn default() -> Self {
        Self {
            // PrettyConfig defaults new lines to \r\n on Windows. We hard-code this to \n for
            // consistency during testing, and for consistency across platforms. Your final binary
            // has no guarantee which platform it will run on, so having your asset data be
            // "platform-independent" is desirable.
            pretty_config: Some(PrettyConfig::default().new_line("\n")),
        }
    }
}

/// A saver for reflected asset types to write in the RON format.
///
/// Data written with this saver can later be loaded with [`RonLoader`]. Note: the data format
/// written by [`RonSaver`] is **incompatible** with [`TypedRonLoader`] (since this saver writes
/// the type name of the asset being saved, whereas [`TypedRonLoader`] expects this to be implicit).
/// Use [`TypedRonSaver`] if you intend to load the data with [`TypedRonLoader`] later.
///
/// This saver requires that the held asset type implements **and** reflects [`Asset`].
#[derive(TypePath, Clone)]
pub struct RonSaver {
    /// The type registry that will be used when serializing reflected values.
    pub registry: TypeRegistryArc,
}

impl FromWorld for RonSaver {
    fn from_world(world: &mut World) -> Self {
        Self {
            registry: world.resource::<AppTypeRegistry>().0.clone(),
        }
    }
}

impl RonSaver {
    /// Saves `asset` to `path` with this saver and `settings`.
    ///
    /// This is a wrapper around [`save_using_saver`] making it convenient to save any reflected
    /// assets.
    pub async fn save<A: Asset + Reflect>(
        &self,
        asset_path: AssetPath<'static>,
        asset: &A,
        settings: &RonSaverSettings,
        asset_server: AssetServer,
    ) -> Result<(), SaveAssetError> {
        // TODO: It would be nice to just have a method that returned the SavedAsset, so that if you
        // wanted to call `save_using_saver` directly, you could without reinventing this wheel.
        // Unfortunately, `SavedAsset` currently requires storing a reference for the final asset,
        // and since we need to create the LoadedUntypedAsset in this method, we can't return the
        // final `SavedAsset`. We might just be able to use the `Moo` type inside of `SavedAsset`,
        // but there were some lifetimes I couldn't figure out - we should come back to this though!
        let mut builder = SavedAssetBuilder::new(asset_server.clone(), asset_path.clone());
        let subasset =
            builder.add_labeled_asset_with_new_handle("Typed", SavedAsset::from_asset(asset));

        let wrapper = LoadedUntypedAsset {
            handle: subasset.untyped(),
        };
        let saved_asset = builder.build(&wrapper);

        save_using_saver(asset_server, self, &asset_path, saved_asset, settings).await
    }
}

impl AssetSaver for RonSaver {
    type Asset = LoadedUntypedAsset;
    type Settings = RonSaverSettings;
    type Error = ReflectedRonSerializeError;
    type OutputLoader = RonLoader;

    async fn save(
        &self,
        writer: &mut crate::io::Writer,
        asset: SavedAsset<'_, '_, Self::Asset>,
        settings: &Self::Settings,
        _asset_path: AssetPath<'_>,
    ) -> Result<<Self::OutputLoader as AssetLoader>::Settings, Self::Error> {
        let Some(subasset) = asset.get_erased_labeled_by_id(&asset.get().handle) else {
            return Err(ReflectedRonSerializeError::MissingSubasset);
        };
        let subasset = subasset.get();

        // We need this scope because otherwise `registry` will live across the await point, causing
        // this future to no longer be Send. Note: it's **not sufficient** to just drop the
        // `registry` later. Rust considers the local active **even after** an explicit drop. This
        // is because it is technically still allowed to access the local even after the stored
        // value has been moved. See https://github.com/rust-lang/rfcs/pull/3943 for more and how
        // this may get fixed.
        let serialized = {
            let registry = self.registry.read();
            let Some(type_registration) = registry.get(subasset.type_id()) else {
                return Err(ReflectedRonSerializeError::ValueNotReflect);
            };

            let Some(reflect_from_ptr) = type_registration.data::<ReflectFromPtr>() else {
                // This should basically never happen. It should only really happen for artificial cases
                // (e.g., you are manually constructing type registrations).
                return Err(ReflectedRonSerializeError::MissingReflectFromPtr);
            };

            // Unwrap is ok because we assume that the ReflectFromPtr we got for the subasset type was
            // also constructed for the subasset type. If not, that's kind of malicious, so not worth
            // worrying about.
            let subasset_reflect = reflect_from_ptr.as_reflect(subasset).unwrap();

            let handle_processor = HandleSerializeProcessor {
                ephemeral_handle_behavior: EphemeralHandleBehavior::Error,
            };
            let reflect_serializer =
                ReflectSerializer::with_processor(subasset_reflect, &registry, &handle_processor);

            let mut serialized = String::new();
            let mut ron_serializer =
                ron::Serializer::new(&mut serialized, settings.pretty_config.clone())
                    .map_err(Into::<RonSerializeError>::into)?;

            reflect_serializer
                .serialize(&mut ron_serializer)
                .map_err(Into::<RonSerializeError>::into)?;

            serialized
        };

        writer
            .write_all(serialized.as_bytes())
            .await
            .map_err(Into::<RonSerializeError>::into)?;

        Ok(())
    }
}

/// A saver for writing serializable types in the RON format.
///
/// Data written with this saver can later be loaded with [`TypedRonLoader<T>`]. Note: the data
/// format written by [`TypedRonSaver`] is **incompatible** with [`RonLoader`] (since [`RonLoader`]
/// expects the data to include the type name of the asset being loaded). Use [`RonSaver`] if you
/// intend to load the data with [`RonLoader`] later.
///
/// This is a typed counterpart to [`RonSaver`]. Therefore, this saver only supports writing a
/// single asset `T` (rather than any reflectable type).
///
/// Unlike [`RonSaver`], this saver does not support saving types with [`Handle`]s, since [`Handle`]
/// cannot be serialized or deserialized. Consider using [`RonSaver`] for these types.
///
/// [`Handle`]: crate::Handle
#[derive(TypePath)]
pub struct TypedRonSaver<A: Asset + Serialize + DeserializeOwned>(PhantomData<fn() -> A>);

impl<A: Asset + Serialize + DeserializeOwned> Default for TypedRonSaver<A> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

// Manual impl since the derive would add an `A: Clone` bound.
impl<A: Asset + Serialize + DeserializeOwned> Clone for TypedRonSaver<A> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<A: Asset + Serialize + DeserializeOwned> AssetSaver for TypedRonSaver<A> {
    type Asset = A;
    type Settings = RonSaverSettings;
    type Error = RonSerializeError;
    type OutputLoader = TypedRonLoader<A>;

    async fn save(
        &self,
        writer: &mut crate::io::Writer,
        asset: SavedAsset<'_, '_, Self::Asset>,
        settings: &Self::Settings,
        _asset_path: AssetPath<'_>,
    ) -> Result<<Self::OutputLoader as AssetLoader>::Settings, Self::Error> {
        let string = match settings.pretty_config.clone() {
            Some(config) => ron::ser::to_string_pretty(asset.get(), config)?,
            None => ron::ser::to_string(asset.get())?,
        };

        Ok(writer.write_all(string.as_bytes()).await?)
    }
}

/// An error type for RON loading.
#[derive(Error, Debug)]
pub enum RonDeserializeError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    RonSpan(#[from] ron::de::SpannedError),
}

/// An error type for RON loading using reflection.
#[derive(Error, Debug)]
pub enum ReflectedRonDeserializeError {
    #[error(transparent)]
    Ron(#[from] RonDeserializeError),
    #[error("Attempted to load type \"{0}\", which does not have `ReflectFromReflect` in the type registry. This may occur for types that have opted-out of `FromReflect`")]
    MissingReflectFromReflect(&'static str),
    #[error("Attempted to load type \"{0}\", which does not have `ReflectAsset` in the type registry. Make sure to add `#[reflect(Asset)]` to your type")]
    MissingReflectAsset(&'static str),
}

/// An error type for RON saving.
#[derive(Error, Debug)]
pub enum RonSerializeError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    Ron(#[from] ron::Error),
}

/// An error type for RON saving using reflection.
#[derive(Error, Debug)]
pub enum ReflectedRonSerializeError {
    #[error(
        "Expected the root LoadedUntypedHandle to hold a handle to its subasset, but it did not have such a subasset"
    )]
    MissingSubasset,
    #[error("Subasset type was not registered in the TypeRegistry. Ensure your subasset's type implements Reflect (and if it's generic it needs to be manually registered)")]
    ValueNotReflect,
    #[error("Subasset type does not have ReflectFromPtr registered in its type data")]
    MissingReflectFromPtr,
    #[error(transparent)]
    Ron(#[from] RonSerializeError),
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use alloc::{string::String, vec};

    use bevy_ecs::reflect::AppTypeRegistry;
    use bevy_reflect::{Reflect, TypePath};
    use bevy_tasks::{futures::check_ready, IoTaskPool};
    use serde::{Deserialize, Serialize};

    use crate::{
        common_loaders::ron::{
            RonLoader, RonSaver, RonSaverSettings, TypedRonLoader, TypedRonSaver,
        },
        saver::{save_using_saver, SavedAsset},
        tests::{create_app, read_asset_as_string, run_app_until},
        Asset, AssetApp, AssetServer, Assets, Handle, LoadState, ReflectAsset, UntypedHandle,
    };

    #[derive(Asset, TypePath, Serialize, Deserialize, Debug, PartialEq)]
    struct SerializedAsset(u32);

    #[derive(Asset, Reflect)]
    #[reflect(Asset)]
    struct ReflectedAsset(u32);

    #[derive(Asset, Reflect)]
    #[reflect(Asset)]
    struct ReflectedAssetWithHandles {
        data: String,
        handle: Handle<ReflectedAsset>,
        untyped: UntypedHandle,
    }

    #[test]
    fn roundtrip_typed_save_and_load() {
        let (mut app, dir) = create_app();

        app.init_asset::<SerializedAsset>().register_asset_loader(
            TypedRonLoader::<SerializedAsset>::new(vec!["particle_effect"]),
        );

        let asset_server = app.world().resource::<AssetServer>().clone();

        let mut task = {
            let asset_server = asset_server.clone();
            IoTaskPool::get().spawn(async move {
                save_using_saver(
                    asset_server,
                    &TypedRonSaver::<SerializedAsset>::default(),
                    &"fire.particle_effect".into(),
                    SavedAsset::from_asset(&SerializedAsset(10)),
                    &RonSaverSettings::default(),
                )
                .await
                .unwrap();
            })
        };

        // Wait for the save to be complete.
        run_app_until(&mut app, |_| check_ready(&mut task).map(|_| ()));

        assert_eq!(
            read_asset_as_string(&dir, Path::new("fire.particle_effect")),
            "(10)"
        );

        let handle = asset_server.load::<SerializedAsset>("fire.particle_effect");
        run_app_until(&mut app, |_| asset_server.is_loaded(&handle).then_some(()));

        let loaded_value = app
            .world()
            .resource::<Assets<SerializedAsset>>()
            .get(&handle)
            .unwrap();
        assert_eq!(loaded_value, &SerializedAsset(10));
    }

    #[test]
    fn roundtrip_untyped_save_and_load() {
        let (mut app, dir) = create_app();

        app.init_asset::<ReflectedAsset>()
            .init_asset::<ReflectedAssetWithHandles>()
            .init_asset_loader::<RonLoader>()
            // Register the types explicitly so we don't rely on the feature flags for reflect auto
            // registration being enabled.
            .register_type::<ReflectedAsset>()
            .register_type::<ReflectedAssetWithHandles>();

        let asset_server = app.world().resource::<AssetServer>().clone();
        let type_registry = app.world().resource::<AppTypeRegistry>().0.clone();

        let saver = RonSaver {
            registry: type_registry,
        };

        let mut task1 = {
            let saver = saver.clone();
            let asset_server = asset_server.clone();
            IoTaskPool::get().spawn(async move {
                saver
                    .save(
                        "ref1.ron".into(),
                        &ReflectedAsset(10),
                        &RonSaverSettings::default(),
                        asset_server.clone(),
                    )
                    .await
                    .unwrap();
            })
        };
        let mut task2 = {
            let saver = saver.clone();
            let asset_server = asset_server.clone();
            IoTaskPool::get().spawn(async move {
                saver
                    .save(
                        "ref2.ron".into(),
                        &ReflectedAsset(20),
                        &RonSaverSettings::default(),
                        asset_server.clone(),
                    )
                    .await
                    .unwrap();
            })
        };
        // Wait for the saves to complete.
        run_app_until(&mut app, |_| check_ready(&mut task1).map(|_| ()));
        run_app_until(&mut app, |_| check_ready(&mut task2).map(|_| ()));
        assert_eq!(
            read_asset_as_string(&dir, Path::new("ref1.ron")),
            r#"{
    "bevy_asset::common_loaders::ron::tests::ReflectedAsset": (10),
}"#
        );
        assert_eq!(
            read_asset_as_string(&dir, Path::new("ref2.ron")),
            r#"{
    "bevy_asset::common_loaders::ron::tests::ReflectedAsset": (20),
}"#
        );

        let ref_handle_1 = asset_server.load::<ReflectedAsset>("ref1.ron#Typed");
        let ref_handle_2 = asset_server
            .load::<ReflectedAsset>("ref2.ron#Typed")
            .untyped();
        let ref_id_1 = ref_handle_1.id();
        let ref_id_2 = ref_handle_2.id();

        let mut task3 = {
            let asset_with_handles = ReflectedAssetWithHandles {
                data: "hiya".into(),
                handle: ref_handle_1,
                untyped: ref_handle_2,
            };

            let asset_server = asset_server.clone();
            IoTaskPool::get().spawn(async move {
                saver
                    .save(
                        "with_handles.ron".into(),
                        &asset_with_handles,
                        &RonSaverSettings::default(),
                        asset_server.clone(),
                    )
                    .await
                    .unwrap();
            })
        };
        // Wait for the save to be complete.
        run_app_until(&mut app, |_| check_ready(&mut task3).map(|_| ()));

        assert_eq!(
            read_asset_as_string(&dir, Path::new("with_handles.ron")),
            r#"{
    "bevy_asset::common_loaders::ron::tests::ReflectedAssetWithHandles": (
        data: "hiya",
        handle: Path("ref1.ron#Typed"),
        untyped: (
            asset_type: "bevy_asset::common_loaders::ron::tests::ReflectedAsset",
            reference: Path("ref2.ron#Typed"),
        ),
    ),
}"#
        );

        // We dropped the handles when the save task completed, so just wait for the two ref handles
        // to be unloaded (so we can be sure that A. these handles get loaded, and B. these assets
        // are considered dependencies).
        run_app_until(&mut app, |_| {
            (matches!(asset_server.load_state(ref_id_1), LoadState::NotLoaded)
                && matches!(asset_server.load_state(ref_id_2), LoadState::NotLoaded))
            .then_some(())
        });

        let root_handle = asset_server.load::<ReflectedAssetWithHandles>("with_handles.ron#Typed");
        run_app_until(&mut app, |_| {
            asset_server
                .is_loaded_with_dependencies(&root_handle)
                .then_some(())
        });

        let root_asset = app
            .world()
            .resource::<Assets<ReflectedAssetWithHandles>>()
            .get(root_handle.id())
            .unwrap();
        let reflected_assets = app.world().resource::<Assets<ReflectedAsset>>();
        assert_eq!(root_asset.data, "hiya");
        assert_eq!(reflected_assets.get(&root_asset.handle).unwrap().0, 10);
        // Try to cast the handle type, and panic on failure.
        let typed_from_untyped = root_asset.untyped.clone().try_typed().unwrap();
        assert_eq!(reflected_assets.get(&typed_from_untyped).unwrap().0, 20);
    }
}
