use bevy_app::App;
use bevy_ecs::system::{Deferred, Res, Resource, SystemBuffer, SystemParam};
use bevy_log::warn;
use bevy_utils::{Duration, Instant, StableHashMap, Uuid};
use std::{borrow::Cow, collections::VecDeque};

use crate::MAX_DIAGNOSTIC_NAME_WIDTH;

/// Unique identifier for a [`Diagnostic`].
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct DiagnosticId(pub Uuid);

impl DiagnosticId {
    pub const fn from_u128(value: u128) -> Self {
        DiagnosticId(Uuid::from_u128(value))
    }
}

impl Default for DiagnosticId {
    fn default() -> Self {
        DiagnosticId(Uuid::new_v4())
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
    pub id: DiagnosticId,
    pub name: Cow<'static, str>,
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
        if let Some(previous) = self.measurement() {
            let delta = (measurement.time - previous.time).as_secs_f64();
            let alpha = (delta / self.ema_smoothing_factor).clamp(0.0, 1.0);
            self.ema += alpha * (measurement.value - self.ema);
        } else {
            self.ema = measurement.value;
        }

        if self.max_history_length > 1 {
            if self.history.len() == self.max_history_length {
                if let Some(removed_diagnostic) = self.history.pop_front() {
                    self.sum -= removed_diagnostic.value;
                }
            }

            self.sum += measurement.value;
        } else {
            self.history.clear();
            self.sum = measurement.value;
        }

        self.history.push_back(measurement);
    }

    /// Create a new diagnostic with the given ID, name and maximum history.
    pub fn new(
        id: DiagnosticId,
        name: impl Into<Cow<'static, str>>,
        max_history_length: usize,
    ) -> Diagnostic {
        let name = name.into();
        if name.chars().count() > MAX_DIAGNOSTIC_NAME_WIDTH {
            // This could be a false positive due to a unicode width being shorter
            warn!(
                "Diagnostic {:?} has name longer than {} characters, and so might overflow in the LogDiagnosticsPlugin\
                Consider using a shorter name.",
                name, MAX_DIAGNOSTIC_NAME_WIDTH
            );
        }
        Diagnostic {
            id,
            name,
            suffix: Cow::Borrowed(""),
            history: VecDeque::with_capacity(max_history_length),
            max_history_length,
            sum: 0.0,
            ema: 0.0,
            ema_smoothing_factor: 2.0 / 21.0,
            is_enabled: true,
        }
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
    // This uses a [`StableHashMap`] to ensure that the iteration order is deterministic between
    // runs when all diagnostics are inserted in the same order.
    diagnostics: StableHashMap<DiagnosticId, Diagnostic>,
}

impl DiagnosticsStore {
    /// Add a new [`Diagnostic`].
    ///
    /// If possible, prefer calling [`App::register_diagnostic`].
    pub fn add(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.insert(diagnostic.id, diagnostic);
    }

    pub fn get(&self, id: DiagnosticId) -> Option<&Diagnostic> {
        self.diagnostics.get(&id)
    }

    pub fn get_mut(&mut self, id: DiagnosticId) -> Option<&mut Diagnostic> {
        self.diagnostics.get_mut(&id)
    }

    /// Get the latest [`DiagnosticMeasurement`] from an enabled [`Diagnostic`].
    pub fn get_measurement(&self, id: DiagnosticId) -> Option<&DiagnosticMeasurement> {
        self.diagnostics
            .get(&id)
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
    pub fn add_measurement<F>(&mut self, id: DiagnosticId, value: F)
    where
        F: FnOnce() -> f64,
    {
        if self
            .store
            .get(id)
            .filter(|diagnostic| diagnostic.is_enabled)
            .is_some()
        {
            let measurement = DiagnosticMeasurement {
                time: Instant::now(),
                value: value(),
            };
            self.queue.0.insert(id, measurement);
        }
    }
}

#[derive(Default)]
struct DiagnosticsBuffer(StableHashMap<DiagnosticId, DiagnosticMeasurement>);

impl SystemBuffer for DiagnosticsBuffer {
    fn apply(
        &mut self,
        _system_meta: &bevy_ecs::system::SystemMeta,
        world: &mut bevy_ecs::world::World,
    ) {
        let mut diagnostics = world.resource_mut::<DiagnosticsStore>();
        for (id, measurement) in self.0.drain() {
            if let Some(diagnostic) = diagnostics.get_mut(id) {
                diagnostic.add_measurement(measurement);
            }
        }
    }
}

/// Extend [`App`] with new `register_diagnostic` function.
pub trait RegisterDiagnostic {
    fn register_diagnostic(&mut self, diagnostic: Diagnostic) -> &mut Self;
}

impl RegisterDiagnostic for App {
    /// Register a new [`Diagnostic`] with an [`App`].
    ///
    /// Will initialize a [`DiagnosticsStore`] if it doesn't exist.
    ///
    /// ```rust
    /// use bevy_app::App;
    /// use bevy_diagnostic::{Diagnostic, DiagnosticsPlugin, DiagnosticId, RegisterDiagnostic};
    ///
    /// const UNIQUE_DIAG_ID: DiagnosticId = DiagnosticId::from_u128(42);
    ///
    /// App::new()
    ///     .register_diagnostic(Diagnostic::new(UNIQUE_DIAG_ID, "example", 10))
    ///     .add_plugins(DiagnosticsPlugin)
    ///     .run();
    /// ```
    fn register_diagnostic(&mut self, diagnostic: Diagnostic) -> &mut Self {
        self.init_resource::<DiagnosticsStore>();
        let mut diagnostics = self.world.resource_mut::<DiagnosticsStore>();
        diagnostics.add(diagnostic);

        self
    }
}
