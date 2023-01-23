/// Iterator type holding chosen iterator
pub enum IterChoice<T, I: Iterator<Item = T>, J: Iterator<Item = T>> {
    /// The first iter possibility
    First(I),
    /// The second iter possibility
    Second(J),
}

impl<T, I: Iterator<Item = T>, J: Iterator<Item = T>> IterChoice<T, I, J> {
    /// Returns an [`IterChoice`] with the first possibility
    #[inline]
    pub fn new_first(iter: I) -> Self {
        Self::First(iter)
    }

    /// Returns an [`IterChoice`] with the second possibility
    #[inline]
    pub fn new_second(iter: J) -> Self {
        Self::Second(iter)
    }
}

impl<T, I: Iterator<Item = T>, J: Iterator<Item = T>> Iterator for IterChoice<T, I, J> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            IterChoice::First(iter) => iter.next(),
            IterChoice::Second(iter) => iter.next(),
        }
    }
}
