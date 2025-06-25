use crate::{renderer::RenderAdapterInfo, ExtractSchedule, RenderSystems};
use alloc::sync::Arc;
use bevy_app::{App, Plugin};
use bevy_ecs::{
    change_detection::{Res, ResMut},
    error::BevyError,
    prelude::{not, resource_exists, IntoScheduleConfigs},
    resource::Resource,
    system::{Commands, Local},
};
use bevy_platform::hash::FixedHasher;
use bevy_render::{render_resource::PipelineCache, renderer::RenderDevice, Extract, Render};
use core::hash::BuildHasher;
use std::{
    fs,
    fs::OpenOptions,
    io,
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
    thread::JoinHandle,
};
use thiserror::Error;
use tracing::{debug, warn};
use wgpu::{hal::PipelineCacheError, Backend, PipelineCacheDescriptor};

/// Plugin for managing [`wgpu::PipelineCache`] resources across application runs.
///
/// When pipelines are compiled by [`crate::PipelineCache`], if this plugin is enabled, it will
/// persist the pipeline cache to disk, allowing for faster startup times in subsequent runs.
///
/// Note: This plugin is currently only supported on the Vulkan backend.
pub struct PersistentPipelineCachePlugin {
    /// A unique key for the application, used to identify the cache directory. Should change
    /// if the application is updated or if the cache should be invalidated.
    pub application_key: &'static str,
    /// The directory where the pipeline cache will be stored.
    pub data_dir: PathBuf,
    /// The eviction policy for the cache.
    pub eviction_policy: EvictionPolicy,
}

impl PersistentPipelineCachePlugin {
    /// Creates a new instance of the `PersistentPipelineCachePlugin` with the specified
    /// application key.
    pub fn new(application_key: &'static str, data_dir: PathBuf) -> Self {
        Self {
            application_key,
            data_dir,
            eviction_policy: EvictionPolicy::Stale,
        }
    }
}

impl Plugin for PersistentPipelineCachePlugin {
    fn build(&self, _app: &mut App) {}

    fn finish(&self, app: &mut App) {
        if !self.data_dir.exists() || !self.data_dir.is_dir() {
            warn!(
                "PersistentPipelineCachePlugin data directory does not exist or is not a directory: {:?}",
                self.data_dir
            );
            return;
        }

        if let Some(render_app) = app.get_sub_app_mut(bevy_render::RenderApp) {
            let adapter_debug = render_app.world().resource::<RenderAdapterInfo>();
            if adapter_debug.backend != Backend::Vulkan {
                warn!("PersistentPipelineCachePlugin is only supported on Vulkan backend..");
                return;
            }
            render_app
                .add_systems(
                    ExtractSchedule,
                    extract_persistent_pipeline_cache
                        .run_if(not(resource_exists::<PersistentPipelineCache>)),
                )
                .add_systems(
                    Render,
                    write_persistent_pipeline_cache
                        .run_if(resource_exists::<PersistentPipelineCache>)
                        .in_set(RenderSystems::Cleanup),
                );
        };

        app.insert_resource(PersistentPipelineCacheConfig {
            application_key: self.application_key,
            data_dir: self.data_dir.clone(),
            eviction_policy: self.eviction_policy,
        });
    }
}

pub fn extract_persistent_pipeline_cache(
    mut commands: Commands,
    persistent_pipeline_cache_config: Extract<Option<Res<PersistentPipelineCacheConfig>>>,
    adapter_debug: Res<RenderAdapterInfo>,
    render_device: Res<RenderDevice>,
) -> Result<(), BevyError> {
    let Some(persistent_pipeline_cache_config) = &*persistent_pipeline_cache_config else {
        return Ok(());
    };

    debug!(
        "Extracting persistent pipeline cache with application key: {}",
        persistent_pipeline_cache_config.application_key
    );
    let cache_path = persistent_pipeline_cache_config
        .data_dir
        .join(persistent_pipeline_cache_config.application_key);

    match persistent_pipeline_cache_config.eviction_policy {
        EvictionPolicy::Always => {
            // Evict all existing data
            if cache_path.exists() {
                fs::remove_dir_all(&cache_path).map_err(PersistentPipelineCacheError::Io)?;
            }
        }
        EvictionPolicy::Stale => {
            // Evict all but matching our application key
            if cache_path.exists() {
                for entry in fs::read_dir(&cache_path).map_err(PersistentPipelineCacheError::Io)? {
                    // Check if the entry is a directory and doesn't match the cache path
                    let entry = entry.map_err(PersistentPipelineCacheError::Io)?;
                    if entry
                        .file_type()
                        .map_err(PersistentPipelineCacheError::Io)?
                        .is_dir()
                        && entry.file_name() != cache_path
                    {
                        fs::remove_dir_all(entry.path())
                            .map_err(PersistentPipelineCacheError::Io)?;
                        debug!("Evicted stale pipeline cache at {:?}", entry.path());
                    }
                }
            }
        }
        EvictionPolicy::Never => {}
    }

    let cache_key = wgpu::util::pipeline_cache_key(&adapter_debug)
        .ok_or(PersistentPipelineCacheError::InvalidAdapterKey)?;
    let cache_path = cache_path.join(cache_key);

    // Ensure the cache directory exists
    if let Some(parent) = cache_path.parent() {
        if !parent.exists() {
            debug!(
                "Creating persistent pipeline cache directory at {:?}",
                parent
            );
            fs::create_dir_all(parent).map_err(PersistentPipelineCacheError::Io)?;
        }
    }

    let persistent_pipeline_cache = PersistentPipelineCache::new(
        &render_device,
        persistent_pipeline_cache_config.application_key,
        &cache_path,
    )?;

    commands.insert_resource(persistent_pipeline_cache);
    Ok(())
}

pub fn write_persistent_pipeline_cache(
    mut persistent_pipeline_cache: ResMut<PersistentPipelineCache>,
    pipeline_cache: Res<PipelineCache>,
    mut pipeline_cache_size: Local<usize>,
) -> Result<(), BevyError> {
    let cache_size = pipeline_cache.size();
    if cache_size > *pipeline_cache_size {
        persistent_pipeline_cache.write()?;
        *pipeline_cache_size = cache_size;
    }

    Ok(())
}

/// Configuration for the persistent pipeline cache.
#[derive(Resource)]
pub struct PersistentPipelineCacheConfig {
    /// A unique key for the application, used to identify the cache directory.
    pub application_key: &'static str,
    /// The directory where the pipeline cache will be stored.
    pub data_dir: PathBuf,
    /// The eviction policy for the cache.
    pub eviction_policy: EvictionPolicy,
}

/// Resource for managing [`wgpu::PipelineCache`].
#[derive(Resource)]
pub struct PersistentPipelineCache {
    cache: Arc<wgpu::PipelineCache>,
    write_lock: Arc<Mutex<()>>,
    write_tasks: Vec<JoinHandle<Result<(), PersistentPipelineCacheError>>>,
    cache_path: PathBuf,
    data_key: u64,
}

impl PersistentPipelineCache {
    /// Create a new instance of the persistent pipeline cache with the given application key and
    /// cache path.
    pub fn new(
        render_device: &RenderDevice,
        app_key: &'static str,
        cache_path: &Path,
    ) -> Result<Self, PersistentPipelineCacheError> {
        // Get data if the cache file exists
        let cache_data = if cache_path.exists() {
            let data = fs::read(cache_path).map_err(PersistentPipelineCacheError::Io)?;
            debug!(
                "Loaded persistent pipeline cache from {:?}, size: {}",
                cache_path,
                data.len()
            );
            Some(data)
        } else {
            debug!("Creating new persistent pipeline cache at {:?}", cache_path);
            None
        };
        // SAFETY: Data was created with a cache key that matches the adapter.
        let cache = unsafe {
            render_device
                .wgpu_device()
                .create_pipeline_cache(&PipelineCacheDescriptor {
                    data: cache_data.as_deref(),
                    label: app_key.into(),
                    fallback: true,
                })
        };

        let data_key = {
            let hasher = FixedHasher;
            hasher.hash_one(&cache_data)
        };

        Ok(PersistentPipelineCache {
            cache: Arc::new(cache),
            write_lock: Arc::new(Mutex::new(())),
            write_tasks: vec![],
            cache_path: cache_path.to_path_buf(),
            data_key,
        })
    }

    /// Get the cached data if it has changed since the last call.
    pub fn get_data(&mut self) -> Option<Vec<u8>> {
        let data = self.cache.get_data();
        let hasher = FixedHasher;
        let data_key = hasher.hash_one(&data);
        if self.data_key != data_key {
            self.data_key = data_key;
            return data;
        }

        None
    }

    /// Write the cached data to disk, if it has changed since the last write.
    pub fn write(&mut self) -> Result<(), PersistentPipelineCacheError> {
        // Process existing tasks
        let mut pending_tasks = vec![];
        let mut error = None;
        for task in self.write_tasks.drain(..) {
            if task.is_finished() {
                match task.join() {
                    Ok(Ok(())) => {
                        debug!("Persistent pipeline cache write task completed successfully.");
                    }
                    Ok(Err(err)) => {
                        warn!("Persistent pipeline cache write task failed: {}", err);
                        error = Some(err);
                    }
                    Err(err) => {
                        warn!("Persistent pipeline cache write task panicked: {:?}", err);
                        error = Some(PersistentPipelineCacheError::Io(io::Error::other(
                            "Persistent pipeline cache write task panicked",
                        )));
                    }
                }
            } else {
                pending_tasks.push(task);
            }
        }

        if let Some(err) = error {
            return Err(err);
        }

        if let Some(data) = self.get_data() {
            let temp = self.cache_path.with_extension("tmp");
            let cache_path = self.cache_path.clone();
            let lock = self.write_lock.clone();
            let join_handle = std::thread::spawn(move || {
                let _guard = lock
                    .lock()
                    .or(Err(PersistentPipelineCacheError::LockError))?;
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&temp)
                    .map_err(PersistentPipelineCacheError::Io)?;
                file.write_all(&data)
                    .map_err(PersistentPipelineCacheError::Io)?;
                fs::rename(&temp, &cache_path).map_err(PersistentPipelineCacheError::Io)?;
                Ok(())
            });
            self.write_tasks.push(join_handle);
        }

        Ok(())
    }

    /// Get the underlying wgpu pipeline cache.
    pub fn get_cache(&self) -> Arc<wgpu::PipelineCache> {
        self.cache.clone()
    }
}

/// Describes the eviction policy for the persistent pipeline cache.
#[derive(Debug, Copy, Clone)]
pub enum EvictionPolicy {
    /// Evict all existing data on startup.
    Always,
    /// Evict all but the data matching the application key on startup.
    Stale,
    /// Never evict any data.
    Never,
}

/// Error type for persistent pipeline cache operations.
#[derive(Debug, Error)]
#[error("Error while handling persistent pipeline cache")]
pub enum PersistentPipelineCacheError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("Failed to create pipeline cache: {0}")]
    DeviceError(#[from] PipelineCacheError),
    #[error("Could not create cache key from adapter")]
    InvalidAdapterKey,
    #[error("Failed to acquire write lock for persistent pipeline cache")]
    LockError,
}
