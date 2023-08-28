/// Represents a volume level value.
#[derive(Clone, Copy, Debug)]
pub enum VolumeLevel {
    /// Volume Amplitude Ratio (unitless)
    /// 
    /// # Notes
    /// 
    /// `Self::Amplitude(a)` is only defined for `a` between `f32::MIN_POSITIVE` and `f32::MAX`.
    /// If `a` is any other value, `VolumeLevel` will be in an invalid state.
    Amplitude(f32),
    /// Decibels of Amplitude Ratio (dB)
    /// 
    /// # Notes
    /// 
    /// `Self::Decibels(a)` is defined for all non-`NaN` values of `a`. However, values with a
    /// magnitude larger than `1,000` are largely meaningless due to the logarithmic nature of
    /// the decibel scale.
    Decibels(f32),
}

impl VolumeLevel {
    /// The amplitude ratio this represents. This value is bounded between
    /// `f32::MIN_POSITIVE` and `f32::MAX`, with `1.` representing a neutral level.
    pub fn amplitude(&self) -> f32 {
        let amplitude = match self {
            Self::Amplitude(amplitude) => *amplitude,
            Self::Decibels(db) => db_to_a(*db),
        };

        debug_assert!(
            (f32::MIN_POSITIVE..=f32::MAX).contains(&amplitude),
            "Amplitude value outside domain"
        );

        amplitude
    }

    /// The decibels of amplitude ratio this represents. This value is bounded between
    /// (approximately) `-800` and `800`, with `0.` representing a neutral level.
    pub fn decibels(&self) -> f32 {
        let db = match self {
            Self::Amplitude(amplitude) => a_to_db(*amplitude),
            Self::Decibels(db) => *db,
        };

        debug_assert!(
            (a_to_db(f32::MIN_POSITIVE)..=a_to_db(f32::MAX)).contains(&db),
            "dB value outside domain"
        );

        db
    }
}

impl Default for VolumeLevel {
    fn default() -> Self {
        Self::Decibels(0.)
    }
}

impl PartialEq for VolumeLevel {
    fn eq(&self, other: &Self) -> bool {
        use VolumeLevel::{Amplitude, Decibels};

        match (self, other) {
            (Amplitude(a), Amplitude(b)) | (Decibels(a), Decibels(b)) => a == b,
            (a, b) => a.decibels() == b.decibels(),
        }
    }
}

impl Eq for VolumeLevel {}

impl PartialOrd for VolumeLevel {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use VolumeLevel::{Amplitude, Decibels};

        Some(match (self, other) {
            (Amplitude(a), Amplitude(b)) | (Decibels(a), Decibels(b)) => a.total_cmp(b),
            (a, b) => a.decibels().total_cmp(&b.decibels()),
        })
    }
}

impl Ord for VolumeLevel {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[inline(always)]
fn db_to_a(db: f32) -> f32 {
    10f32.powf(db / 20.)
}

#[inline(always)]
fn a_to_db(a: f32) -> f32 {
    a.log10() * 20.
}

#[cfg(test)]
mod tests {
    use super::VolumeLevel;

    /// Based on Wikipedia's [Decibel](https://web.archive.org/web/20230810185300/https://en.wikipedia.org/wiki/Decibel) article.
    const DB_AMPLITUDE_TABLE: [(f32, f32); 27] = [
        (100., 100000.),
        (90., 31623.),
        (80., 10000.),
        (70., 3162.),
        (60., 1000.),
        (50., 316.2),
        (40., 100.),
        (30., 31.62),
        (20., 10.),
        (10., 3.162),
        (5.998, 1.995),
        (3.003, 1.413),
        (1.002, 1.122),
        (0., 1.),
        (-1.002, 0.891),
        (-3.003, 0.708),
        (-5.998, 0.501),
        (-10., 0.3162),
        (-20., 0.1),
        (-30., 0.03162),
        (-40., 0.01),
        (-50., 0.003162),
        (-60., 0.001),
        (-70., 0.0003162),
        (-80., 0.0001),
        (-90., 0.00003162),
        (-100., 0.00001),
    ];

    #[test]
    fn volume_conversion() {
        use VolumeLevel::{Amplitude, Decibels};

        for (db, amp) in DB_AMPLITUDE_TABLE {
            for volume in [Amplitude(amp), Decibels(db)] {
                let db_test = volume.decibels();
                let amp_test = volume.amplitude();

                let db_delta = db_test - db;
                let amp_relative_delta = (amp_test - amp) / amp;

                assert!(
                    db_delta.abs() < 1e-2,
                    "Expected ~{}dB; Got {}dB (Delta {})",
                    db,
                    db_test,
                    db_delta
                );
                assert!(
                    amp_relative_delta.abs() < 1e-3,
                    "Expected ~{}; Got {} (Relative Delta {})",
                    amp,
                    amp_test,
                    amp_relative_delta
                );
            }
        }
    }
}
