use crate::func::args::ArgCountOutOfBoundsError;
use core::fmt::{Debug, Formatter};

/// A container for zero or more argument counts for a function.
///
/// For most functions, this will contain a single count,
/// however, overloaded functions may contain more.
///
/// # Maximum Argument Count
///
/// The maximum number of arguments that can be represented by this struct is 63,
/// as given by [`ArgCount::MAX_COUNT`].
/// The reason for this is that all counts are stored internally as a single `u64`
/// with each bit representing a specific count based on its bit index.
///
/// This allows for a smaller memory footprint and faster lookups compared to a
/// `HashSet` or `Vec` of possible counts.
/// It's also more appropriate for representing the argument counts of a function
/// given that most functions will not have more than a few arguments.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ArgCount {
    /// The bits representing the argument counts.
    ///
    /// Each bit represents a specific count based on its bit index.
    bits: u64,
    /// The total number of argument counts.
    len: u8,
}

impl ArgCount {
    /// The maximum number of arguments that can be represented by this struct.
    pub const MAX_COUNT: usize = u64::BITS as usize - 1;

    /// Create a new [`ArgCount`] with the given count.
    ///
    /// # Errors
    ///
    /// Returns an error if the count is greater than [`Self::MAX_COUNT`].
    pub fn new(count: usize) -> Result<Self, ArgCountOutOfBoundsError> {
        Ok(Self {
            bits: 1 << Self::try_to_u8(count)?,
            len: 1,
        })
    }

    /// Adds the given count to this [`ArgCount`].
    ///
    /// # Panics
    ///
    /// Panics if the count is greater than [`Self::MAX_COUNT`].
    pub fn add(&mut self, count: usize) {
        self.try_add(count).unwrap();
    }

    /// Attempts to add the given count to this [`ArgCount`].
    ///
    /// # Errors
    ///
    /// Returns an error if the count is greater than [`Self::MAX_COUNT`].
    pub fn try_add(&mut self, count: usize) -> Result<(), ArgCountOutOfBoundsError> {
        let count = Self::try_to_u8(count)?;

        if !self.contains_unchecked(count) {
            self.len += 1;
            self.bits |= 1 << count;
        }

        Ok(())
    }

    /// Removes the given count from this [`ArgCount`].
    pub fn remove(&mut self, count: usize) {
        self.try_remove(count).unwrap();
    }

    /// Attempts to remove the given count from this [`ArgCount`].
    ///
    /// # Errors
    ///
    /// Returns an error if the count is greater than [`Self::MAX_COUNT`].
    pub fn try_remove(&mut self, count: usize) -> Result<(), ArgCountOutOfBoundsError> {
        let count = Self::try_to_u8(count)?;

        if self.contains_unchecked(count) {
            self.len -= 1;
            self.bits &= !(1 << count);
        }

        Ok(())
    }

    /// Checks if this [`ArgCount`] contains the given count.
    pub fn contains(&self, count: usize) -> bool {
        count < usize::BITS as usize && (self.bits >> count) & 1 == 1
    }

    /// Returns the total number of argument counts that this [`ArgCount`] contains.
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Returns true if this [`ArgCount`] contains no argument counts.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns an iterator over the argument counts in this [`ArgCount`].
    pub fn iter(&self) -> ArgCountIter {
        ArgCountIter {
            count: *self,
            index: 0,
            found: 0,
        }
    }

    /// Checks if this [`ArgCount`] contains the given count without any bounds checking.
    ///
    /// # Panics
    ///
    /// Panics if the count is greater than [`Self::MAX_COUNT`].
    fn contains_unchecked(&self, count: u8) -> bool {
        (self.bits >> count) & 1 == 1
    }

    /// Attempts to convert the given count to a `u8` within the bounds of the [maximum count].
    ///
    /// [maximum count]: Self::MAX_COUNT
    fn try_to_u8(count: usize) -> Result<u8, ArgCountOutOfBoundsError> {
        if count > Self::MAX_COUNT {
            Err(ArgCountOutOfBoundsError(count))
        } else {
            Ok(count as u8)
        }
    }
}

/// Defaults this [`ArgCount`] to empty.
///
/// This means that it contains no argument counts, including zero.
impl Default for ArgCount {
    fn default() -> Self {
        Self { bits: 0, len: 0 }
    }
}

impl Debug for ArgCount {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

/// An iterator for the argument counts in an [`ArgCount`].
pub struct ArgCountIter {
    count: ArgCount,
    index: u8,
    found: u8,
}

impl Iterator for ArgCountIter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.index as usize > ArgCount::MAX_COUNT {
                return None;
            }

            if self.found == self.count.len {
                // All counts have been found
                return None;
            }

            if self.count.contains_unchecked(self.index) {
                self.index += 1;
                self.found += 1;
                return Some(self.index as usize - 1);
            }

            self.index += 1;
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count.len(), Some(self.count.len()))
    }
}

impl ExactSizeIterator for ArgCountIter {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_default_to_empty() {
        let count = ArgCount::default();

        assert_eq!(count.len(), 0);
        assert!(count.is_empty());

        assert!(!count.contains(0));
    }

    #[test]
    fn should_construct_with_count() {
        let count = ArgCount::new(3).unwrap();

        assert_eq!(count.len(), 1);
        assert!(!count.is_empty());

        assert!(count.contains(3));
    }

    #[test]
    fn should_add_count() {
        let mut count = ArgCount::default();
        count.add(3);

        assert_eq!(count.len(), 1);

        assert!(count.contains(3));
    }

    #[test]
    fn should_add_multiple_counts() {
        let mut count = ArgCount::default();
        count.add(3);
        count.add(5);
        count.add(7);

        assert_eq!(count.len(), 3);

        assert!(!count.contains(0));
        assert!(!count.contains(1));
        assert!(!count.contains(2));

        assert!(count.contains(3));
        assert!(count.contains(5));
        assert!(count.contains(7));
    }

    #[test]
    fn should_add_idempotently() {
        let mut count = ArgCount::default();
        count.add(3);
        count.add(3);

        assert_eq!(count.len(), 1);
        assert!(count.contains(3));
    }

    #[test]
    fn should_remove_count() {
        let mut count = ArgCount::default();
        count.add(3);

        assert_eq!(count.len(), 1);
        assert!(count.contains(3));

        count.remove(3);

        assert_eq!(count.len(), 0);
        assert!(!count.contains(3));
    }

    #[test]
    fn should_allow_removing_nonexistent_count() {
        let mut count = ArgCount::default();

        assert_eq!(count.len(), 0);
        assert!(!count.contains(3));

        count.remove(3);

        assert_eq!(count.len(), 0);
        assert!(!count.contains(3));
    }

    #[test]
    fn should_iterate_over_counts() {
        let mut count = ArgCount::default();
        count.add(3);
        count.add(5);
        count.add(7);

        let mut iter = count.iter();

        assert_eq!(iter.len(), 3);

        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), Some(5));
        assert_eq!(iter.next(), Some(7));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn should_return_error_for_out_of_bounds_count() {
        let count = ArgCount::new(64);
        assert_eq!(count, Err(ArgCountOutOfBoundsError(64)));

        let mut count = ArgCount::default();
        assert_eq!(count.try_add(64), Err(ArgCountOutOfBoundsError(64)));
        assert_eq!(count.try_remove(64), Err(ArgCountOutOfBoundsError(64)));
    }

    #[test]
    fn should_return_false_for_out_of_bounds_contains() {
        let count = ArgCount::default();
        assert!(!count.contains(64));
    }
}
