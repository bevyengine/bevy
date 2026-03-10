//! Tests for [`bevy_platform::dirs::preferences_dir`] with platform-specific behavior.

#![allow(unsafe_code, reason = "Tests manipulate environment variables.")]

#[cfg(target_os = "linux")]
#[test]
fn preferences_dir_follows_xdg() {
    use bevy_platform::dirs::preferences_dir;
    use std::env;

    // a default path should be returned when XDG_CONFIG_HOME is not set
    // SAFETY: no multi-threaded access to the environment
    // 1. integration tests are a standalone process
    // 2. we have a single #[test], so no parallel execution
    unsafe { env::remove_var("XDG_CONFIG_HOME") }
    let default = preferences_dir().unwrap();

    // the default path should also be returned when XDG_CONFIG_HOME is set but empty
    // SAFETY: no multi-threaded access to the environment
    unsafe { env::set_var("XDG_CONFIG_HOME", "") }
    assert_eq!(preferences_dir(), Some(default.clone()));

    // when set, the path should be returned if it's absolute
    // SAFETY: no multi-threaded access to the environment
    unsafe { env::set_var("XDG_CONFIG_HOME", "/tmp") }
    assert_eq!(preferences_dir(), Some("/tmp".into()));

    // when set to a relative path, it should be ignored and the default path should be returned
    // SAFETY: no multi-threaded access to the environment
    unsafe { env::set_var("XDG_CONFIG_HOME", "relative/path") }
    assert_eq!(preferences_dir(), Some(default));
}
