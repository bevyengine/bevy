use bevy_utils::{Duration, HashMap, Instant, Uuid};
use std::collections::VecDeque;

/// Unique identifier for a [Diagnostic]
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
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

/// A timeline of [DiagnosticMeasurement]s of a specific type.
/// Diagnostic examples: frames per second, CPU usage, network latency
#[derive(Debug)]
pub struct Diagnostic {
    pub id: DiagnosticId,
    pub name: String,
    history: VecDeque<DiagnosticMeasurement>,
    sum: f64,
    max_history_length: usize,
}

impl Diagnostic {
    pub fn add_measurement(&mut self, value: f64) {
        let time = Instant::now();
        if self.history.len() == self.max_history_length {
            if let Some(removed_diagnostic) = self.history.pop_back() {
                self.sum -= removed_diagnostic.value;
            }
        }

        self.sum += value;
        self.history
            .push_front(DiagnosticMeasurement { time, value });
    }

    pub fn new(id: DiagnosticId, name: &str, max_history_length: usize) -> Diagnostic {
        Diagnostic {
            id,
            name: name.to_string(),
            history: VecDeque::with_capacity(max_history_length),
            max_history_length,
            sum: 0.0,
        }
    }

    pub fn value(&self) -> Option<f64> {
        self.history.back().map(|measurement| measurement.value)
    }

    pub fn sum(&self) -> f64 {
        self.sum
    }

    pub fn average(&self) -> Option<f64> {
        if !self.history.is_empty() {
            Some(self.sum / self.history.len() as f64)
        } else {
            None
        }
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    pub fn duration(&self) -> Option<Duration> {
        if self.history.len() < 2 {
            return None;
        }

        if let Some(oldest) = self.history.back() {
            if let Some(newest) = self.history.front() {
                return Some(newest.time.duration_since(oldest.time));
            }
        }

        None
    }

    pub fn get_max_history_length(&self) -> usize {
        self.max_history_length
    }
}

/// A collection of [Diagnostic]s
#[derive(Debug, Default)]
pub struct Diagnostics {
    diagnostics: HashMap<DiagnosticId, Diagnostic>,
}

impl Diagnostics {
    pub fn add(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.insert(diagnostic.id, diagnostic);
    }

    pub fn get(&self, id: DiagnosticId) -> Option<&Diagnostic> {
        self.diagnostics.get(&id)
    }

    pub fn get_mut(&mut self, id: DiagnosticId) -> Option<&mut Diagnostic> {
        self.diagnostics.get_mut(&id)
    }

    pub fn get_measurement(&self, id: DiagnosticId) -> Option<&DiagnosticMeasurement> {
        self.diagnostics
            .get(&id)
            .and_then(|diagnostic| diagnostic.history.front())
    }

    pub fn add_measurement(&mut self, id: DiagnosticId, value: f64) {
        if let Some(diagnostic) = self.diagnostics.get_mut(&id) {
            diagnostic.add_measurement(value);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.values()
    }
}
