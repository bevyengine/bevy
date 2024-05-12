use std::hash::{Hash, Hasher};
use std::{borrow::Cow, collections::VecDeque};

use bevy_app::{App, SubApp};
use bevy_ecs::system::{Deferred, Res, Resource, SystemBuffer, SystemParam};
use bevy_utils::{hashbrown::HashMap, Duration, Instant, PassHash};
use const_fnv1a_hash::fnv1a_hash_str_64;

use crate::DEFAULT_MAX_HISTORY_LENGTH;

/// Unique diagnostic path, separated by `/`.
///
/// Requirements:
/// - Can't be empty
/// - Can't have leading or trailing `/`
/// - Can't have empty components.
#[derive(Debug, Clone)]
pub struct DiagnosticPath {
    path: Cow<'static, str>,
    hash: u64,
}

impl DiagnosticPath {
    /// Create a new `DiagnosticPath`. Usable in const contexts.
    ///
    /// **Note**: path is not validated, so make sure it follows all the requirements.
    pub const fn const_new(path: &'static str) -> DiagnosticPath {
        DiagnosticPath {
            path: Cow::Borrowed(path),
            hash: fnv1a_hash_str_64(path),
        }
    }

    /// Create a new `DiagnosticPath` from the specified string.
    pub fn new(path: impl Into<Cow<'static, str>>) -> DiagnosticPath {
        let path = path.into();

        debug_assert!(!path.is_empty(), "diagnostic path can't be empty");
        debug_assert!(
            !path.starts_with('/'),
            "diagnostic path can't be start with `/`"
        );
        debug_assert!(
            !path.ends_with('/'),
            "diagnostic path can't be end with `/`"
        );
        debug_assert!(
            !path.contains("//"),
            "diagnostic path can't contain empty components"
        );

        DiagnosticPath {
            hash: fnv1a_hash_str_64(&path),
            path,
        }
    }

    /// Create a new `DiagnosticPath` from an iterator over components.
    pub fn from_components<'a>(components: impl IntoIterator<Item = &'a str>) -> DiagnosticPath {
        let mut buf = String::new();

        for (i, component) in components.into_iter().enumerate() {
            if i > 0 {
                buf.push('/');
            }
            buf.push_str(component);
        }

        DiagnosticPath::new(buf)
    }

    /// Returns full path, joined by `/`
    pub fn as_str(&self) -> &str {
        &self.path
    }

    /// Returns an iterator over path components.
    pub fn components(&self) -> impl Iterator<Item = &str> + '_ {
        self.path.split('/')
    }
}

impl From<DiagnosticPath> for String {
    fn from(path: DiagnosticPath) -> Self {
        path.path.into()
    }
}

impl Eq for DiagnosticPath {}

impl PartialEq for DiagnosticPath {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.path == other.path
    }
}

impl Hash for DiagnosticPath {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.hash);
    }
}

impl std::fmt::Display for DiagnosticPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.path.fmt(f)
    }
}

/// A single measurement of a [`Diagnostic`].
#[derive(Debug)]
pub struct DiagnosticMeasurement {
    pub time: Instant,
    pub value: f64,
}

/// A timeline of [`DiagnosticMeasurement`]s of a specific type.
/// Diagnostic examples: frames per second, CPU usage, network latency
#[derive(Debug)]
pub struct Diagnostic {
    path: DiagnosticPath,
    pub suffix: Cow<'static, str>,
    history: VecDeque<DiagnosticMeasurement>,
    sum: f64,
    ema: f64,
    ema_smoothing_factor: f64,
    max_history_length: usize,
    pub is_enabled: bool,
}

impl Diagnostic {
    /// Add a new value as a [`DiagnosticMeasurement`].
    pub fn add_measurement(&mut self, measurement: DiagnosticMeasurement) {
        if measurement.value.is_nan() {
            // Skip calculating the moving average.
        } else if let Some(previous) = self.measurement() {
            let delta = (measurement.time - previous.time).as_secs_f64();
            let alpha = (delta / self.ema_smoothing_factor).clamp(0.0, 1.0);
            self.ema += alpha * (measurement.value - self.ema);
        } else {
            self.ema = measurement.value;
        }

        if self.max_history_length > 1 {
            if self.history.len() >= self.max_history_length {
                if let Some(removed_diagnostic) = self.history.pop_front() {
                    if !removed_diagnostic.value.is_nan() {
                        self.sum -= removed_diagnostic.value;
                    }
                }
            }

            if measurement.value.is_finite() {
                self.sum += measurement.value;
            }
        } else {
            self.history.clear();
            if measurement.value.is_nan() {
                self.sum = 0.0;
            } else {
                self.sum = measurement.value;
            }
        }

        self.history.push_back(measurement);
    }

    /// Create a new diagnostic with the given path.
    pub fn new(path: DiagnosticPath) -> Diagnostic {
        Diagnostic {
            path,
            suffix: Cow::Borrowed(""),
            history: VecDeque::with_capacity(DEFAULT_MAX_HISTORY_LENGTH),
            max_history_length: DEFAULT_MAX_HISTORY_LENGTH,
            sum: 0.0,
            ema: 0.0,
            ema_smoothing_factor: 2.0 / 21.0,
            is_enabled: true,
        }
    }

    /// Set the maximum history length.
    #[must_use]
    pub fn with_max_history_length(mut self, max_history_length: usize) -> Self {
        self.max_history_length = max_history_length;

        // reserve/reserve_exact reserve space for n *additional* elements.
        let expected_capacity = self
            .max_history_length
            .saturating_sub(self.history.capacity());
        self.history.reserve_exact(expected_capacity);
        self.history.shrink_to(expected_capacity);
        self
    }

    /// Add a suffix to use when logging the value, can be used to show a unit.
    #[must_use]
    pub fn with_suffix(mut self, suffix: impl Into<Cow<'static, str>>) -> Self {
        self.suffix = suffix.into();
        self
    }

    /// The smoothing factor used for the exponential smoothing used for
    /// [`smoothed`](Self::smoothed).
    ///
    /// If measurements come in less frequently than `smoothing_factor` seconds
    /// apart, no smoothing will be applied. As measurements come in more
    /// frequently, the smoothing takes a greater effect such that it takes
    /// approximately `smoothing_factor` seconds for 83% of an instantaneous
    /// change in measurement to e reflected in the smoothed value.
    ///
    /// A smoothing factor of 0.0 will effectively disable smoothing.
    #[must_use]
    pub fn with_smoothing_factor(mut self, smoothing_factor: f64) -> Self {
        self.ema_smoothing_factor = smoothing_factor;
        self
    }

    pub fn path(&self) -> &DiagnosticPath {
        &self.path
    }

    /// Get the latest measurement from this diagnostic.
    #[inline]
    pub fn measurement(&self) -> Option<&DiagnosticMeasurement> {
        self.history.back()
    }

    /// Get the latest value from this diagnostic.
    pub fn value(&self) -> Option<f64> {
        self.measurement().map(|measurement| measurement.value)
    }

    /// Return the simple moving average of this diagnostic's recent values.
    /// N.B. this a cheap operation as the sum is cached.
    pub fn average(&self) -> Option<f64> {
        if !self.history.is_empty() {
            Some(self.sum / self.history.len() as f64)
        } else {
            None
        }
    }

    /// Return the exponential moving average of this diagnostic.
    ///
    /// This is by default tuned to behave reasonably well for a typical
    /// measurement that changes every frame such as frametime. This can be
    /// adjusted using [`with_smoothing_factor`](Self::with_smoothing_factor).
    pub fn smoothed(&self) -> Option<f64> {
        if !self.history.is_empty() {
            Some(self.ema)
        } else {
            None
        }
    }

    /// Return the number of elements for this diagnostic.
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// Return the duration between the oldest and most recent values for this diagnostic.
    pub fn duration(&self) -> Option<Duration> {
        if self.history.len() < 2 {
            return None;
        }

        if let Some(newest) = self.history.back() {
            if let Some(oldest) = self.history.front() {
                return Some(newest.time.duration_since(oldest.time));
            }
        }

        None
    }

    /// Return the maximum number of elements for this diagnostic.
    pub fn get_max_history_length(&self) -> usize {
        self.max_history_length
    }

    pub fn values(&self) -> impl Iterator<Item = &f64> {
        self.history.iter().map(|x| &x.value)
    }

    pub fn measurements(&self) -> impl Iterator<Item = &DiagnosticMeasurement> {
        self.history.iter()
    }

    /// Clear the history of this diagnostic.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

/// A collection of [`Diagnostic`]s.
#[derive(Debug, Default, Resource)]
pub struct DiagnosticsStore {
    diagnostics: HashMap<DiagnosticPath, Diagnostic, PassHash>,
}

impl DiagnosticsStore {
    /// Add a new [`Diagnostic`].
    ///
    /// If possible, prefer calling [`App::register_diagnostic`].
    pub fn add(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.insert(diagnostic.path.clone(), diagnostic);
    }

    pub fn get(&self, path: &DiagnosticPath) -> Option<&Diagnostic> {
        self.diagnostics.get(path)
    }

    pub fn get_mut(&mut self, path: &DiagnosticPath) -> Option<&mut Diagnostic> {
        self.diagnostics.get_mut(path)
    }

    /// Get the latest [`DiagnosticMeasurement`] from an enabled [`Diagnostic`].
    pub fn get_measurement(&self, path: &DiagnosticPath) -> Option<&DiagnosticMeasurement> {
        self.diagnostics
            .get(path)
            .filter(|diagnostic| diagnostic.is_enabled)
            .and_then(|diagnostic| diagnostic.measurement())
    }

    /// Return an iterator over all [`Diagnostic`]s.
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.values()
    }

    /// Return an iterator over all [`Diagnostic`]s, by mutable reference.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Diagnostic> {
        self.diagnostics.values_mut()
    }
}

/// Record new [`DiagnosticMeasurement`]'s.
#[derive(SystemParam)]
pub struct Diagnostics<'w, 's> {
    store: Res<'w, DiagnosticsStore>,
    queue: Deferred<'s, DiagnosticsBuffer>,
}

impl<'w, 's> Diagnostics<'w, 's> {
    /// Add a measurement to an enabled [`Diagnostic`]. The measurement is passed as a function so that
    /// it will be evaluated only if the [`Diagnostic`] is enabled. This can be useful if the value is
    /// costly to calculate.
    pub fn add_measurement<F>(&mut self, path: &DiagnosticPath, value: F)
    where
        F: FnOnce() -> f64,
    {
        if self
            .store
            .get(path)
            .filter(|diagnostic| diagnostic.is_enabled)
            .is_some()
        {
            let measurement = DiagnosticMeasurement {
                time: Instant::now(),
                value: value(),
            };
            self.queue.0.insert(path.clone(), measurement);
        }
    }
}

#[derive(Default)]
struct DiagnosticsBuffer(HashMap<DiagnosticPath, DiagnosticMeasurement, PassHash>);

impl SystemBuffer for DiagnosticsBuffer {
    fn apply(
        &mut self,
        _system_meta: &bevy_ecs::system::SystemMeta,
        world: &mut bevy_ecs::world::World,
    ) {
        let mut diagnostics = world.resource_mut::<DiagnosticsStore>();
        for (path, measurement) in self.0.drain() {
            if let Some(diagnostic) = diagnostics.get_mut(&path) {
                diagnostic.add_measurement(measurement);
            }
        }
    }
}

/// Extend [`App`] with new `register_diagnostic` function.
pub trait RegisterDiagnostic {
    /// Register a new [`Diagnostic`] with an [`App`].
    ///
    /// Will initialize a [`DiagnosticsStore`] if it doesn't exist.
    ///
    /// ```
    /// use bevy_app::App;
    /// use bevy_diagnostic::{Diagnostic, DiagnosticsPlugin, DiagnosticPath, RegisterDiagnostic};
    ///
    /// const UNIQUE_DIAG_PATH: DiagnosticPath = DiagnosticPath::const_new("foo/bar");
    ///
    /// App::new()
    ///     .register_diagnostic(Diagnostic::new(UNIQUE_DIAG_PATH))
    ///     .add_plugins(DiagnosticsPlugin)
    ///     .run();
    /// ```
    fn register_diagnostic(&mut self, diagnostic: Diagnostic) -> &mut Self;
}

impl RegisterDiagnostic for SubApp {
    fn register_diagnostic(&mut self, diagnostic: Diagnostic) -> &mut Self {
        self.init_resource::<DiagnosticsStore>();
        let mut diagnostics = self.world_mut().resource_mut::<DiagnosticsStore>();
        diagnostics.add(diagnostic);

        self
    }
}

impl RegisterDiagnostic for App {
    fn register_diagnostic(&mut self, diagnostic: Diagnostic) -> &mut Self {
        SubApp::register_diagnostic(self.main_mut(), diagnostic);
        self
    }
}
