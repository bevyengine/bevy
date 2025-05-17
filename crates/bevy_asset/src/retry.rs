use crate::{
    io::{AssetReaderError, AssetSourceId},
    Asset, AssetLoadError, AssetPath, AssetServer, LoadState, UntypedAssetLoadFailedEvent,
};
use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::{
    event::EventReader,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Res, ResMut},
};
use bevy_platform::collections::HashMap;
use core::time::Duration;
use serde::{Deserialize, Serialize};
use std::{any::TypeId, boxed::Box, time::Instant};
use std::{borrow::ToOwned, io::ErrorKind};
use tracing::{error, info};

#[allow(unused_imports)]
use crate::io::AssetReader;

/// Returns settings appropriate for a particular asset load failure.
pub trait ProvideAssetLoadRetrySettings: Send + Sync + 'static {
    fn get_retry_settings(
        &self,
        default_settings: AssetLoadRetrySettings,
        event: &UntypedAssetLoadFailedEvent,
    ) -> AssetLoadRetrySettings;
}

/// An [`AssetLoadRetrySettings`] provider that uses the contained settings for the following errors:
//
/// - I/O errors (e.g. timeouts, interrupted connections, etc)
/// - Remote server errors (e.g. `500 Internal Server Error`),
/// - Rate limit errors (e.g. `429 Too Many Requests`)
///
/// For failures that do not match these conditions, it will leave the incoming defaults untouched.
/// If you would like to override those cases as well, set `retry_settings_for_unmatched` to anything
/// other than `None` (e.g. `Some(AssetLoadRetrySettings::no_retries())`)
pub struct IoErrorRetrySettingsProvider {
    pub retry_settings: AssetLoadRetrySettings,
    /// Settings to use for failures *not* matched by this provider as io failures that should be retried.
    /// If `None`, default settings from the [`AssetReader`] will be used.
    pub retry_settings_for_unmatched: Option<AssetLoadRetrySettings>,
}

impl Default for IoErrorRetrySettingsProvider {
    fn default() -> Self {
        Self {
            retry_settings: AssetLoadRetrySettings::network_default(),
            retry_settings_for_unmatched: None, // pass thru
        }
    }
}

impl From<AssetLoadRetrySettings> for IoErrorRetrySettingsProvider {
    fn from(value: AssetLoadRetrySettings) -> Self {
        Self::new(value)
    }
}

impl IoErrorRetrySettingsProvider {
    pub fn new(retry_settings: AssetLoadRetrySettings) -> Self {
        Self {
            retry_settings,
            ..Default::default()
        }
    }
    pub fn with_retry_settings_for_unmatched(
        mut self,
        retry_settings: Option<AssetLoadRetrySettings>,
    ) -> Self {
        self.retry_settings_for_unmatched = retry_settings;
        self
    }
}

impl ProvideAssetLoadRetrySettings for IoErrorRetrySettingsProvider {
    fn get_retry_settings(
        &self,
        _default_settings: AssetLoadRetrySettings,
        event: &UntypedAssetLoadFailedEvent,
    ) -> AssetLoadRetrySettings {
        if let AssetLoadError::AssetReaderError(read_error) = &event.error {
            match read_error {
                AssetReaderError::NotFound(_) => {}
                AssetReaderError::Io(io_error) => match io_error.kind() {
                    ErrorKind::InvalidInput
                    | ErrorKind::InvalidData
                    | ErrorKind::NotFound
                    | ErrorKind::PermissionDenied
                    | ErrorKind::Unsupported => {}
                    _ => return self.retry_settings,
                },
                AssetReaderError::HttpError(status_code) => {
                    match status_code {
                        // Retry after server errors and rate limits
                        500..=599 | 429 => return self.retry_settings,
                        _ => {}
                    }
                }
            }
        }

        self.retry_settings_for_unmatched
            .unwrap_or_else(AssetLoadRetrySettings::no_retries)
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct AssetLoadRetrySettings {
    /// The maximum number of retries to attempt.
    pub max_attempts: usize,
    /// The maximum duration between retry attempts.
    pub max_delay: Duration,
    /// The initial delay between the asset failing and the first retry attempt.
    pub starting_delay: Duration,
    /// Each attempt, the delay is multiplied by this factor.
    pub time_multiple: f32,
}

impl AssetLoadRetrySettings {
    pub fn with_max_attempts(mut self, max_attempts: usize) -> Self {
        self.max_attempts = max_attempts;
        self
    }
    pub fn with_max_delay(mut self, max_delay: Duration) -> Self {
        self.max_delay = max_delay;
        self
    }
    pub fn with_starting_delay(mut self, starting_delay: Duration) -> Self {
        self.starting_delay = starting_delay;
        self
    }
    pub fn with_time_multiple(mut self, time_multiple: f32) -> Self {
        self.time_multiple = time_multiple;
        self
    }
    /// Returns `false` if no retries will be attempted.
    pub fn will_retry(&self) -> bool {
        self.max_attempts > 0
    }
    /// Returns a configuration that does not allow any retries.
    pub fn no_retries() -> Self {
        AssetLoadRetrySettings {
            max_attempts: 0,
            max_delay: Duration::default(),
            starting_delay: Duration::default(),
            time_multiple: f32::INFINITY,
        }
    }
    /// Evenly-spaced retry attempts up to a particular limit.
    pub fn constant_interval(interval: Duration, max_attempts: usize) -> Self {
        AssetLoadRetrySettings {
            max_attempts,
            max_delay: interval,
            starting_delay: interval,
            time_multiple: 1.0,
        }
    }
    /// Returns sane retry defaults for when loading assets over a network.
    pub fn network_default() -> Self {
        AssetLoadRetrySettings {
            max_attempts: 5,
            max_delay: Duration::from_millis(10000),
            starting_delay: Duration::from_millis(100),
            time_multiple: 3.0,
        }
    }
    /// Computes the next time a retry should be attempted.
    fn get_next_attempt_time(&self, now: Instant, attempt_number: usize) -> Option<Instant> {
        if attempt_number > self.max_attempts {
            None
        } else {
            let delay = Duration::from_secs_f32(
                self.starting_delay.as_secs_f32()
                    * self.time_multiple.powf(attempt_number as f32 - 1.0),
            )
            .min(self.max_delay);

            Some(now + delay)
        }
    }
}

impl PartialEq for AssetLoadRetrySettings {
    fn eq(&self, other: &Self) -> bool {
        (self.max_attempts == 0 && other.max_attempts == 0)
            || (self.max_attempts == other.max_attempts
                && self.max_delay == other.max_delay
                && self.starting_delay == other.starting_delay
                && self.time_multiple == other.time_multiple)
    }
}

impl ProvideAssetLoadRetrySettings for AssetLoadRetrySettings {
    fn get_retry_settings(
        &self,
        _default_settings: AssetLoadRetrySettings,
        _event: &UntypedAssetLoadFailedEvent,
    ) -> AssetLoadRetrySettings {
        *self
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum AssetLoadRetryStatus {
    /// We're currently waiting until the scheduled retry time.
    Scheduled,
    /// The asset is currently being retried.
    Loading,
    /// We're done retrying and the state will be removed shortly.
    Finished,
}

#[derive(Debug)]
pub struct AssetLoadRetryQueueItem {
    status: AssetLoadRetryStatus,
    retry_settings: AssetLoadRetrySettings,
    current_attempt: usize,
    scheduled_at: Instant,
    path: AssetPath<'static>,
}

impl AssetLoadRetryQueueItem {
    pub fn is_final_attempt(&self) -> bool {
        self.current_attempt >= self.retry_settings.max_attempts
    }
}

/// This resource holds retry settings and state for pending and current retry attempts.
///
/// ### Retry Settings
///
/// For a particular asset load error, final retry settings are resolved by passing retry defaults from [`AssetReader::get_default_retry_settings`]
/// through the following chain:
/// 1. Source overrides from [`AssetLoadRetrier::set_source_settings`] (if any)
/// 2. Asset overrides from [`AssetLoadRetrier::set_asset_settings`] (if any)
#[derive(Resource)]
pub struct AssetLoadRetrier {
    /// Asset-specific retry settings (takes priority over source settings)
    asset_settings_overrides: HashMap<TypeId, Box<dyn ProvideAssetLoadRetrySettings>>,
    /// Source-specific retry settings.
    source_settings_overrides:
        HashMap<AssetSourceId<'static>, Box<dyn ProvideAssetLoadRetrySettings>>,

    /// State for retries that are queued or busy retrying.
    pending: HashMap<AssetPath<'static>, AssetLoadRetryQueueItem>,
    /// A really quick way to tell if there's anything needing loaded right now.
    pub next_scheduled: Option<Instant>,
    /// If set to `true`, load failures will not be retried.
    disabled: bool,
    /// When was the last time we cleared completed items from the retry state?
    last_cleanup: Option<Instant>,
    /// How often to run [`asset_load_retry_cleanup`], which checks for completed items and clears them out of the retry queue.
    pub cleanup_interval: Duration,
    /// Minimum duration to keep state around to prevent future failures from triggering an all-new set of retries.
    pub retain_duration: Duration,
}

impl Default for AssetLoadRetrier {
    fn default() -> Self {
        Self {
            disabled: false,
            pending: Default::default(),
            next_scheduled: None,
            asset_settings_overrides: Default::default(),
            source_settings_overrides: Default::default(),
            last_cleanup: None,
            cleanup_interval: Duration::from_millis(1000),
            retain_duration: Duration::from_millis(100),
        }
    }
}

impl AssetLoadRetrier {
    /// Returns an unordered iterator over current pending and retrying items.
    pub fn iter(
        &self,
    ) -> bevy_platform::collections::hash_map::Iter<'_, AssetPath<'static>, AssetLoadRetryQueueItem>
    {
        self.pending.iter()
    }

    /// Sets a source-specific retry settings provider. The default settings passed to the provider will be
    /// the defaults returned by the asset source.
    pub fn set_source_settings<S: ProvideAssetLoadRetrySettings>(
        &mut self,
        source_id: AssetSourceId<'static>,
        settings: S,
    ) {
        self.source_settings_overrides
            .insert(source_id, Box::new(settings));
    }

    /// Sets an asset-specific retry settings provider. The default settings passed to the provider will be
    /// what was returned by the asset source (including any overrides from `set_source_settings`).
    pub fn set_asset_settings<A: Asset, S: ProvideAssetLoadRetrySettings>(&mut self, settings: S) {
        let type_id = TypeId::of::<A>();
        self.asset_settings_overrides
            .insert(type_id, Box::new(settings));
    }

    /// Resolves final retry settings to use for a failed asset.
    pub fn get_retry_settings(
        &self,
        asset_server: &Res<'_, AssetServer>,
        event: &UntypedAssetLoadFailedEvent,
    ) -> AssetLoadRetrySettings {
        let source_id = event.path.source();
        let Ok(source) = asset_server.get_source(source_id) else {
            error!("Failed to look up source! {:?}", source_id);
            return AssetLoadRetrySettings::no_retries();
        };
        let mut settings = source.reader().get_default_retry_settings(event);
        if let Some(source_override) = self.source_settings_overrides.get(source_id) {
            settings = source_override.get_retry_settings(settings, event);
        }
        if let Some(asset_override) = self.asset_settings_overrides.get(&event.id.type_id()) {
            settings = asset_override.get_retry_settings(settings, event);
        }
        settings
    }

    /// Re-enables the retry system after being disabled by `disable()`
    pub fn enable(&mut self) {
        self.disabled = false;
    }

    /// Clears all pending retries and prevents any future retry queueing.
    pub fn disable(&mut self) {
        self.pending.clear();
        self.next_scheduled = None;
        self.disabled = true;
    }

    /// Returns `true` if there are no retries pending or loading.
    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    fn push_retry(
        &mut self,
        event: &UntypedAssetLoadFailedEvent,
        scheduled_at: Instant,
        retry_settings: AssetLoadRetrySettings,
    ) {
        self.next_scheduled = Some(
            self.next_scheduled
                .map_or(scheduled_at, |v| v.min(scheduled_at)),
        );
        self.pending.insert(
            event.path.to_owned(),
            AssetLoadRetryQueueItem {
                status: AssetLoadRetryStatus::Scheduled,
                current_attempt: 1,
                scheduled_at,
                path: event.path.clone(),
                retry_settings,
            },
        );
    }

    fn recompute_next_scheduled(&mut self) {
        self.next_scheduled = self
            .pending
            .iter()
            .filter_map(|(_, item)| {
                if item.status == AssetLoadRetryStatus::Scheduled {
                    Some(&item.scheduled_at)
                } else {
                    None
                }
            })
            .min()
            .copied();
    }
}

/// Clears completed assets out of the queue.
pub fn asset_load_retry_cleanup(
    asset_server: Res<AssetServer>,
    mut retrier: ResMut<AssetLoadRetrier>,
) {
    // Throttle cleanup attempts
    let now: Instant = Instant::now();
    let should_cleanup = retrier.last_cleanup.map_or(true, |last_cleanup| {
        (now - last_cleanup) > retrier.cleanup_interval
    });
    if !should_cleanup {
        return;
    }
    retrier.last_cleanup = Some(now);

    let retain_duration = retrier.retain_duration;
    retrier.pending.retain(|_, item| {
        if item.status == AssetLoadRetryStatus::Finished {
            return now < item.scheduled_at;
        }
        let load_state = asset_server.get_load_state_for_path(&item.path);
        let should_drop = load_state.as_ref().map_or(true, |load_state| {
            if item.is_final_attempt() && item.status == AssetLoadRetryStatus::Loading {
                // If we're on the final retry attempt, drop once the status is anything but loading
                !load_state.is_loading()
            } else {
                // If there are still future retries that will be attempted, only drop if the asset's been dropped or succeeded.
                load_state.is_loaded()
            }
        });
        if should_drop {
            if let Some(load_state) = load_state {
                if load_state.is_loaded() {
                    info!(
                        "Successfully loaded {:?} after {} retry attempt(s)",
                        item.path, item.current_attempt
                    );
                } else {
                    error!(
                        "All {} retry attempt(s) failed for {:?}",
                        item.current_attempt, item.path,
                    );
                }
            }
            item.status = AssetLoadRetryStatus::Finished;
            item.scheduled_at = now + retain_duration;
        }
        true
    });
}

/// Watches for asset load errors and queues retries according to the [`AssetLoadRetrySettings`] returned by the particular [`AssetReader`].
pub fn asset_load_retry(
    asset_server: Res<AssetServer>,
    mut error_events: EventReader<UntypedAssetLoadFailedEvent>,
    mut retrier: ResMut<AssetLoadRetrier>,
) {
    if retrier.disabled {
        return;
    }

    let retain_duration = retrier.retain_duration;
    let mut schedule_needs_refresh = false;

    // Start loading queued items
    let now: Instant = Instant::now();
    let attempt_load = retrier
        .next_scheduled
        .map_or(false, |next_scheduled| now > next_scheduled);

    if attempt_load {
        for (_, item) in retrier.pending.iter_mut() {
            if item.status == AssetLoadRetryStatus::Scheduled && now > item.scheduled_at {
                // Check that the asset hasn't been loaded or dropped while we've been waiting.
                let load_state = asset_server.get_load_state_for_path(&item.path);
                let should_load = load_state.map_or(false, |load_state| {
                    matches!(load_state, LoadState::NotLoaded | LoadState::Failed(_))
                });

                if should_load {
                    item.status = AssetLoadRetryStatus::Loading;
                    info!(
                        "Attempting reload {:?} (try {}/{})",
                        item.path, item.current_attempt, item.retry_settings.max_attempts
                    );
                    let path = item.path.clone();
                    let _ = asset_server.load_untyped(path);
                } else {
                    // The handle died while we were waiting to retry, or it was resolved by some other method.
                    // Remove from the queue after the retain_duration and don't retry.
                    item.status = AssetLoadRetryStatus::Finished;
                    item.scheduled_at = now + retain_duration;
                }
            }
        }
        schedule_needs_refresh = true;
    }

    // Queue retries for failed assets
    for event in error_events.read() {
        match event.error {
            AssetLoadError::AssetLoaderError { .. } | AssetLoadError::AssetReaderError(_) => {
                let new_initial_settings = retrier.get_retry_settings(&asset_server, event);

                let existing_item = retrier.pending.get_mut(&event.path);
                if let Some(existing_item) = existing_item {
                    // Check that the error that just happened is worthy of retrying. For instance,
                    // if we hit a 404 Not Found after an initial 500 Server Error, we want to stop.
                    if !new_initial_settings.will_retry() {
                        schedule_needs_refresh = true;
                        existing_item.scheduled_at = now + retain_duration;
                        existing_item.status = AssetLoadRetryStatus::Finished;
                        continue;
                    }

                    let attempt_number = existing_item.current_attempt + 1;
                    let Some(next_attempt) = existing_item
                        .retry_settings
                        .get_next_attempt_time(now, attempt_number)
                    else {
                        schedule_needs_refresh = true;
                        existing_item.scheduled_at = now + retain_duration;
                        existing_item.status = AssetLoadRetryStatus::Finished;
                        continue;
                    };

                    existing_item.scheduled_at = next_attempt;
                    existing_item.current_attempt = attempt_number;
                    existing_item.status = AssetLoadRetryStatus::Scheduled;
                    let wait_duration = (next_attempt - now).as_secs_f32();
                    info!(
                        "Queued retry {:?} (try {}/{} in {:.2} sec)",
                        event.path,
                        attempt_number,
                        existing_item.retry_settings.max_attempts,
                        wait_duration
                    );
                    schedule_needs_refresh = true;
                } else {
                    let retry_settings = new_initial_settings;
                    let Some(scheduled_at) = retry_settings.get_next_attempt_time(now, 1) else {
                        continue;
                    };
                    let wait_duration = (scheduled_at - now).as_secs_f32();
                    info!(
                        "Queued retry {:?} (try 1/{} in {:.2} sec)",
                        event.path, retry_settings.max_attempts, wait_duration
                    );
                    retrier.push_retry(event, scheduled_at, retry_settings);
                }
            }
            _ => {}
        }
    }

    if schedule_needs_refresh {
        retrier.recompute_next_scheduled();
    }
}

/// Provides retrying of assets that failed to load. Retry settings are provided by an asset
/// source's [`AssetReader`] via its [`AssetReader::get_default_retry_settings`] method. This
/// can be be overridden by setting custom providers on the [`AssetLoadRetrier`] resource for
/// particular source and asset types.
#[derive(Default)]
pub struct AssetLoadRetryPlugin;

impl Plugin for AssetLoadRetryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetLoadRetrier>().add_systems(
            PreUpdate,
            (asset_load_retry, asset_load_retry_cleanup).chain(),
        );
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        str::FromStr,
        sync::{Arc, Mutex},
    };

    use bevy_app::App;
    use bevy_core::TaskPoolPlugin;
    use bevy_utils::{Duration, HashMap};

    use crate::{
        io::{memory::Dir, AssetSource, AssetSourceId},
        retry::{
            AssetLoadRetrier, AssetLoadRetryPlugin, AssetLoadRetrySettings, AssetLoadRetryStatus,
            IoErrorRetrySettingsProvider,
        },
        tests::{run_app_until, CoolText, CoolTextLoader, SubText, UnstableMemoryAssetReader},
        AssetApp, AssetPath, AssetPlugin, AssetServer, Assets, LoadState,
    };

    fn retry_app(
        expected_failure_count: usize,
        load_delay: Option<Duration>,
    ) -> (App, Arc<Mutex<HashMap<PathBuf, usize>>>) {
        let mut app = App::new();

        let a_path = "a.cool.ron";
        let a_ron = r#"
(
    text: "a",
    dependencies: [
        "foo/b.cool.ron",
        "c.cool.ron",
    ],
    embedded_dependencies: [],
    sub_texts: [],
)"#;
        let b_path = "foo/b.cool.ron";
        let b_ron = r#"
(
    text: "b",
    dependencies: [],
    embedded_dependencies: [],
    sub_texts: [],
)"#;

        let dir = Dir::default();
        dir.insert_asset_text(Path::new(a_path), a_ron);
        dir.insert_asset_text(Path::new(b_path), b_ron);

        let mut unstable_reader = UnstableMemoryAssetReader::new(dir, expected_failure_count);
        if let Some(load_delay) = load_delay {
            unstable_reader.load_delay = load_delay;
        }
        let attempt_counters = unstable_reader.attempt_counters.clone();
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSource::build().with_reader(move || Box::new(unstable_reader.clone())),
        )
        .add_plugins(TaskPoolPlugin::default())
        .add_plugins(AssetPlugin::default())
        .add_plugins(AssetLoadRetryPlugin)
        .init_asset::<CoolText>()
        .init_asset::<SubText>()
        .register_asset_loader(CoolTextLoader);

        app.update();
        (app, attempt_counters)
    }

    #[test]
    fn retry_failed_loads() {
        #[cfg(not(feature = "multi-threaded"))]
        panic!("This test requires the \"multi-threaded\" feature\ncargo test --package bevy_asset --features multi-threaded");

        let (mut app, _) = retry_app(3, None);

        {
            let mut asset_retrier = app.world.resource_mut::<AssetLoadRetrier>();
            asset_retrier.cleanup_interval = Duration::from_millis(1);
            asset_retrier.set_source_settings(
                AssetSourceId::Default,
                // Zero-second delays will cause retries to happen each tick
                IoErrorRetrySettingsProvider::from(AssetLoadRetrySettings {
                    max_attempts: 5,
                    max_delay: Duration::from_millis(0),
                    starting_delay: Duration::from_millis(0),
                    time_multiple: 1.0,
                }),
            );
        }

        // Start loading an asset
        let (a_handle, a_id) = {
            let asset_server = app.world.resource::<AssetServer>();
            let a_handle = asset_server.load::<CoolText>("a.cool.ron");
            let a_id = a_handle.id();
            (a_handle, a_id)
        };
        app.world.spawn(a_handle);
        {
            let asset_server = app.world.resource::<AssetServer>();
            assert_eq!(
                asset_server.get_load_state(a_id).unwrap(),
                LoadState::Loading
            );
        }

        // Wait for it to fail
        run_app_until(&mut app, |world| {
            let asset_server = world.resource::<AssetServer>();
            if asset_server.get_load_state(a_id).unwrap() == LoadState::Failed {
                return Some(());
            }
            None
        });
        {
            let assets = app.world.resource::<Assets<CoolText>>();
            let asset_retrier = app.world.resource::<AssetLoadRetrier>();
            assert!(assets.get(a_id).is_none());
            assert_eq!(
                asset_retrier.pending.len(),
                1,
                "Failed asset should be queued for a retry"
            );
            let (_, queue_item) = asset_retrier.pending.iter().next().unwrap();
            assert_eq!(queue_item.status, AssetLoadRetryStatus::Scheduled);
            assert_eq!(
                queue_item.retry_settings,
                AssetLoadRetrySettings {
                    max_attempts: 5,
                    max_delay: Duration::from_millis(0),
                    starting_delay: Duration::from_millis(0),
                    time_multiple: 1.0,
                }
            );
        }

        // Wait for the retry system to kick in
        run_app_until(&mut app, |world| {
            let asset_server = world.resource::<AssetServer>();
            if asset_server.get_load_state(a_id).unwrap() == LoadState::Loading {
                return Some(());
            }
            None
        });

        {
            let asset_retrier = app.world.resource::<AssetLoadRetrier>();
            let (_, queue_item) = asset_retrier.pending.iter().next().unwrap();
            assert_eq!(queue_item.status, AssetLoadRetryStatus::Loading);
            assert_eq!(asset_retrier.next_scheduled, None);
        }

        // Wait for the load to complete
        run_app_until(&mut app, |world| {
            let asset_server = world.resource::<AssetServer>();
            if asset_server.get_load_state(a_id).unwrap() == LoadState::Loaded {
                let asset_retrier = world.resource::<AssetLoadRetrier>();
                if asset_retrier.is_empty() {
                    assert!(asset_retrier.pending.is_empty());
                    assert_eq!(asset_retrier.next_scheduled, None);
                    return Some(());
                }
            }
            None
        });

        {
            let assets = app.world.resource::<Assets<CoolText>>();
            assert_eq!(assets.get(a_id).unwrap().text, "a");
        }
    }

    #[test]
    fn handle_drop_before_success() {
        #[cfg(not(feature = "multi-threaded"))]
        panic!("This test requires the \"multi-threaded\" feature\ncargo test --package bevy_asset --features multi-threaded");

        let (mut app, attempt_counters) = retry_app(2, Some(Duration::from_millis(10)));
        {
            let mut asset_retrier = app.world.resource_mut::<AssetLoadRetrier>();
            asset_retrier.cleanup_interval = Duration::from_millis(1);
            asset_retrier.set_source_settings(
                AssetSourceId::Default,
                IoErrorRetrySettingsProvider::from(AssetLoadRetrySettings {
                    max_attempts: 1,
                    max_delay: Duration::from_millis(1000),
                    starting_delay: Duration::from_millis(1000),
                    time_multiple: 1.0,
                }),
            );
        }

        // Start loading an asset
        let a_path = "a.cool.ron";
        let a_pathbuf = PathBuf::from_str(a_path).unwrap();
        let a_assetpath = AssetPath::parse(a_path);
        let (a_handle, a_id) = {
            let asset_server = app.world.resource::<AssetServer>();
            let a_handle = asset_server.load::<CoolText>("a.cool.ron");
            let a_id = a_handle.id();
            (a_handle, a_id)
        };

        let handle_entity = app.world.spawn(a_handle).id();

        // Wait for it to fail and then schedule
        run_app_until(&mut app, |world| {
            let asset_server = world.resource::<AssetServer>();
            if asset_server.get_load_state(a_id).unwrap() == LoadState::Failed {
                let retrier = world.resource::<AssetLoadRetrier>();
                if let Some(item) = retrier.pending.get(&a_assetpath) {
                    if item.status == AssetLoadRetryStatus::Scheduled {
                        return Some(());
                    }
                }
            }
            None
        });

        // Drop the handle
        app.world.despawn(handle_entity);

        // Wait and check that no retry loads are attempted
        run_app_until(&mut app, |world| {
            let asset_server = world.resource::<AssetServer>();
            let asset_retrier = world.resource::<AssetLoadRetrier>();
            if asset_server.get_load_state(a_id).is_none() && asset_retrier.is_empty() {
                return Some(());
            }
            None
        });

        let a_attempts = *attempt_counters.lock().unwrap().get(&a_pathbuf).unwrap();
        assert_eq!(a_attempts, 1, "Expected no retry attempts");
    }
}
