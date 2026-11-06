//! Dummy values for static analysis tools.
//! 
//! crates/bevy_shader/src/shader_cache.rs resolves a magic "constants"
//! namespace to a virtual module where constants can be injected, similar to
//! pipeline override constants. Until this is refactored into using standard
//! WESL features, this package serves to have something on the filesystem
//! that corresponds to the module that would otherwise be "not found" during
//! design-time/static analysis.

const SORTED_FRAGMENT_MAX_COUNT: u32 = 128;
const SCREEN_SPACE_SPECULAR_TRANSMISSION_BLUR_TAPS: u32 = 128;
const MAX_DIRECTIONAL_LIGHTS: u32 = 128;
const MAX_CASCADES_PER_LIGHT: u32 = 128;
const WORLD_CACHE_SIZE: u32 = 128;
const SHADOW_SAMPLES: u32 = 128;
