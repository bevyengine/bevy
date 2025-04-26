use bevy_reflect_derive::impl_reflect_opaque;

impl_reflect_opaque!(::alloc::collections::BTreeSet<T: Ord + Eq + Clone + Send + Sync>(Clone));
