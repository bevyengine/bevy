mod fixed_timestep;
mod stopwatch;
#[allow(clippy::module_inception)]
mod time;
mod timer;

pub use fixed_timestep::*;
pub use instant::{Duration, Instant};
pub use stopwatch::*;
pub use time::*;
pub use timer::*;

pub mod prelude {
    //! The Bevy Time Prelude.
    #[doc(hidden)]
    pub use crate::{Time, Timer};
}
