#[macro_export]
macro_rules! define_atomic_id {
    ($atomic_id_type:ident) => {
        #[derive(Copy, Clone, Hash, Eq, PartialEq, PartialOrd, Ord, Debug)]
        pub struct $atomic_id_type(core::num::NonZero<u32>);

        // We use new instead of default to indicate that each ID created will be unique.
        #[allow(clippy::new_without_default)]
        impl $atomic_id_type {
            pub fn new() -> Self {
                use core::sync::atomic::{AtomicU32, Ordering};

                static COUNTER: AtomicU32 = AtomicU32::new(1);

                let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
                Self(core::num::NonZero::<u32>::new(counter).unwrap_or_else(|| {
                    panic!(
                        "The system ran out of unique `{}`s.",
                        stringify!($atomic_id_type)
                    );
                }))
            }
        }

        impl From<$atomic_id_type> for core::num::NonZero<u32> {
            fn from(value: $atomic_id_type) -> Self {
                value.0
            }
        }

        impl From<core::num::NonZero<u32>> for $atomic_id_type {
            fn from(value: core::num::NonZero<u32>) -> Self {
                Self(value)
            }
        }
    };
}
