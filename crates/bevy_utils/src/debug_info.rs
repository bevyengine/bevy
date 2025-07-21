use crate::cfg;
cfg::alloc! {
    use alloc::{borrow::Cow, fmt, string::String};
}
#[cfg(feature = "debug")]
use core::any::type_name;
use disqualified::ShortName;

#[cfg(not(feature = "debug"))]
const FEATURE_DISABLED: &str = "Enable the debug feature to see the name";

/// Wrapper to help debugging ECS issues. This is used to display the names of systems, components, ...
///
/// * If the `debug` feature is enabled, the actual name will be used
/// * If it is disabled, a string mentioning the disabled feature will be used
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugName {
    #[cfg(feature = "debug")]
    name: Cow<'static, str>,
}

cfg::alloc! {
    impl fmt::Display for DebugName {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            #[cfg(feature = "debug")]
            f.write_str(self.name.as_ref())?;
            #[cfg(not(feature = "debug"))]
            f.write_str(FEATURE_DISABLED)?;

            Ok(())
        }
    }
}

impl DebugName {
    /// Create a new `DebugName` from a `&str`
    ///
    /// The value will be ignored if the `debug` feature is not enabled
    #[cfg_attr(
        not(feature = "debug"),
        expect(
            unused_variables,
            reason = "The value will be ignored if the `debug` feature is not enabled"
        )
    )]
    pub const fn borrowed(value: &'static str) -> Self {
        DebugName {
            #[cfg(feature = "debug")]
            name: Cow::Borrowed(value),
        }
    }

    cfg::alloc! {
        /// Create a new `DebugName` from a `String`
        ///
        /// The value will be ignored if the `debug` feature is not enabled
        #[cfg_attr(
            not(feature = "debug"),
            expect(
                unused_variables,
                reason = "The value will be ignored if the `debug` feature is not enabled"
            )
        )]
        pub fn owned(value: String) -> Self {
            DebugName {
                #[cfg(feature = "debug")]
                name: Cow::Owned(value),
            }
        }
    }

    /// Create a new `DebugName` from a type by using its [`core::any::type_name`]
    ///
    /// The value will be ignored if the `debug` feature is not enabled
    pub fn type_name<T>() -> Self {
        DebugName {
            #[cfg(feature = "debug")]
            name: Cow::Borrowed(type_name::<T>()),
        }
    }

    /// Get the [`ShortName`] corresponding to this debug name
    ///
    /// The value will be a static string if the `debug` feature is not enabled
    pub fn shortname(&self) -> ShortName {
        #[cfg(feature = "debug")]
        return ShortName(self.name.as_ref());
        #[cfg(not(feature = "debug"))]
        return ShortName(FEATURE_DISABLED);
    }

    /// Return the string hold by this `DebugName`
    ///
    /// This is intended for debugging purpose, and only available if the `debug` feature is enabled
    #[cfg(feature = "debug")]
    pub fn as_string(&self) -> String {
        self.name.clone().into_owned()
    }
}

cfg::alloc! {
    impl From<Cow<'static, str>> for DebugName {
        #[cfg_attr(
            not(feature = "debug"),
            expect(
                unused_variables,
                reason = "The value will be ignored if the `debug` feature is not enabled"
            )
        )]
        fn from(value: Cow<'static, str>) -> Self {
            Self {
                #[cfg(feature = "debug")]
                name: value,
            }
        }
    }

    impl From<String> for DebugName {
        fn from(value: String) -> Self {
            Self::owned(value)
        }
    }
}

impl From<&'static str> for DebugName {
    fn from(value: &'static str) -> Self {
        Self::borrowed(value)
    }
}
