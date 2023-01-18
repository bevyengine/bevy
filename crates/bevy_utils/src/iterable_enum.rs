use std::marker::PhantomData;

/// A trait for enums to get a `Unit`-enum-field by a `usize`
pub trait IterableEnum: Sized {
    /// Gets an `Unit`-enum-field by the given `usize` index
    fn get_at(index: usize) -> Option<Self>;

    /// Creates a new [`EnumIterator`] which will numerically return every `Unit` of this enum
    #[inline]
    fn enum_iter() -> EnumIterator<Self> {
        EnumIterator {
            accelerator: 0,
            phantom: PhantomData,
        }
    }
}

/// An iterator over `IterableEnum`s
/// 
/// Iterates all `Unit` fields in numeric order
pub struct EnumIterator<E: IterableEnum> {
    accelerator: usize,
    phantom: PhantomData<E>,
}

impl<E: IterableEnum> Iterator for EnumIterator<E> {
    type Item = E;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(unit) = E::get_at(self.accelerator) {
            self.accelerator += 1;
            Some(unit)
        } else {
            None
        }
    }
}
