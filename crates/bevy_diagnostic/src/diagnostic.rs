use bevy_log::warn;
use bevy_utils::{Duration, Instant, StableHashMap, Uuid};
use std::{borrow::Cow, collections::VecDeque};

use crate::MAX_DIAGNOSTIC_NAME_WIDTH;

/// Unique identifier for a [Diagnostic]
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

/// A single measurement of a [Diagnostic]
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
    max_history_length: usize,
    pub is_enabled: bool,
}

impl Diagnostic {
    /// Add a new value as a [`DiagnosticMeasurement`]. Its timestamp will be [`Instant::now`].
    pub fn add_measurement(&mut self, value: f64) {
        let time = Instant::now();
        if self.max_history_length > 1 {
            if self.history.len() == self.max_history_length {
                if let Some(removed_diagnostic) = self.history.pop_front() {
                    self.sum -= removed_diagnostic.value;
                }
            }

            self.sum += value;
        } else {
            self.history.clear();
            self.sum = value;
        }
        self.history
            .push_back(DiagnosticMeasurement { time, value });
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
            is_enabled: true,
        }
    }

    /// Add a suffix to use when logging the value, can be used to show a unit.
    #[must_use]
    pub fn with_suffix(mut self, suffix: impl Into<Cow<'static, str>>) -> Self {
        self.suffix = suffix.into();
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

    /// Return the mean (average) of this diagnostic's values.
    /// N.B. this a cheap operation as the sum is cached.
    pub fn average(&self) -> Option<f64> {
        if !self.history.is_empty() {
            Some(self.sum / self.history.len() as f64)
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

/// A collection of [Diagnostic]s
#[derive(Debug, Default)]
pub struct Diagnostics {
    // This uses a [`StableHashMap`] to ensure that the iteration order is deterministic between
    // runs when all diagnostics are inserted in the same order.
    diagnostics: StableHashMap<DiagnosticId, Diagnostic>,
}

impl Diagnostics {
    /// Add a new [`Diagnostic`].
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

    /// Add a measurement to an enabled [`Diagnostic`]. The measurement is passed as a function so that
    /// it will be evaluated only if the [`Diagnostic`] is enabled. This can be useful if the value is
    /// costly to calculate.
    pub fn add_measurement<F>(&mut self, id: DiagnosticId, value: F)
    where
        F: FnOnce() -> f64,
    {
        if let Some(diagnostic) = self
            .diagnostics
            .get_mut(&id)
            .filter(|diagnostic| diagnostic.is_enabled)
        {
            diagnostic.add_measurement(value());
        }
    }

    /// Return an iterator over all [`Diagnostic`].
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.values()
    }
}
