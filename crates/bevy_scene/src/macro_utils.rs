use bevy_ecs::name::Name;
use bevy_platform::{
    collections::{hash_map::RawEntryMut, HashMap},
    hash::PassHash,
};
/// This is used by the [`bsn!`](crate::bsn) macro to generate compile-time only references to symbols. Currently this is used
/// to add IDE support for nested type names, as it allows us to pass the input Ident from the input to the output code.
pub const fn touch_type<T>() {}

/// This is used by the [`bsn!`](crate::bsn) macro to generate a per-name unique ID of an expression-based name at runtime.
pub struct NameIds {
    refs: HashMap<Name, usize, PassHash>,
    next: usize,
}
impl NameIds {
    /// Create a new [`NameIds`] mapping starting at a given index
    /// The [`bsn!`](crate::bsn) should set `next` to be the next free fixed id from the macro-time `EntityRefs`
    pub fn new(next: usize) -> Self {
        Self {
            refs: HashMap::default(),
            next,
        }
    }
    /// Retrieves the index for a given entity name.
    /// Creates a new one if it hasn't been seen yet.
    pub fn get(&mut self, name: impl Into<Name>) -> (&Name, usize) {
        let name: Name = name.into();
        match self
            .refs
            .raw_entry_mut()
            .from_key_hashed_nocheck(name.pre_hash(), &name)
        {
            RawEntryMut::Occupied(entry) => {
                let kv = entry.into_key_value();
                (kv.0, *kv.1)
            }
            RawEntryMut::Vacant(entry) => {
                let index = self.next;
                let kv = entry.insert(name, index);
                self.next += 1;
                (kv.0, *kv.1)
            }
        }
    }
}

/// Creates a tuple that will be nested after it passes 11 items.
/// When there is a single item, it is _not_ wrapped in a tuple.
/// This is implemented in a way that creates the smallest number of trait impls possible.
#[macro_export]
#[doc(hidden)]
macro_rules! auto_nest_tuple {
    // direct expansion
    () => { () };
    ($a:expr) => {
        $a
    };
    ($a:expr, $b:expr) => {
        (
            $a,
            $b,
        )
    };
    ($a:expr, $b:expr, $c:expr) => {
        (
            $a,
            $b,
            $c,
        )
    };
    ($a:expr, $b:expr, $c:expr, $d:expr) => {
        (
            $a,
            $b,
            $c,
            $d,
        )
    };
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr) => {
        (
            $a,
            $b,
            $c,
            $d,
            $e,
        )
    };
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr) => {
        (
            $a,
            $b,
            $c,
            $d,
            $e,
            $f,
        )
    };
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr, $g:expr) => {
        (
            $a,
            $b,
            $c,
            $d,
            $e,
            $f,
            $g,
        )
    };
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr, $g:expr, $h:expr) => {
        (
            $a,
            $b,
            $c,
            $d,
            $e,
            $f,
            $g,
            $h,
        )
    };
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr, $g:expr, $h:expr, $i:expr) => {
        (
            $a,
            $b,
            $c,
            $d,
            $e,
            $f,
            $g,
            $h,
            $i,
        )
    };
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr, $g:expr, $h:expr, $i:expr, $j:expr) => {
        (
            $a,
            $b,
            $c,
            $d,
            $e,
            $f,
            $g,
            $h,
            $i,
            $j,
        )
    };
    ($a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr, $g:expr, $h:expr, $i:expr, $j:expr, $k:expr) => {
        (
            $a,
            $b,
            $c,
            $d,
            $e,
            $f,
            $g,
            $h,
            $i,
            $j,
            $k,
        )
    };

    // recursive expansion
    (
        $a:expr, $b:expr, $c:expr, $d:expr, $e:expr, $f:expr,
        $g:expr, $h:expr, $i:expr, $j:expr, $k:expr, $($rest:expr),*
    ) => {
        (
            $a,
            $b,
            $c,
            $d,
            $e,
            $f,
            $g,
            $h,
            $i,
            $j,
            $k,
            $crate::auto_nest_tuple!($($rest),*)
        )
    };
}

/// This is used by the [`bsn!`](crate::bsn) derive to work around [this Rust limitation](https://github.com/rust-lang/rust/issues/86935).
/// A fix is implemented and on track for stabilization. If it is ever implemented, we can remove this.
pub type PathResolveHelper<T> = T;
#[cfg(test)]
mod tests {

    use crate::macro_utils::NameIds;

    #[test]
    fn test_name_ids() {
        let mut names = NameIds::new(1);
        let (_name_0, _id_0) = names.get("foo");
        let (_name_1, id_1) = names.get("bar");
        assert_eq!(id_1, 2);
    }
}
