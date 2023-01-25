/// Iterator which iterates the `FIRST` or second entry in a tuple
pub struct TupleExtract<F, S, I: Iterator<Item = (F, S)>, const FIRST: bool> {
    inner: I,
}

impl<F, S, I: Iterator<Item = (F, S)>> TupleExtract<F, S, I, true> {
    /// Creates a `TupleExtract` iterator which extracts the *first* entry
    pub fn new_first(inner: I) -> Self {
        Self { inner }
    }
}

impl<F, S, I: Iterator<Item = (F, S)>> TupleExtract<F, S, I, false> {
    /// Creates a `TupleExtract` iterator which extracts the *second* entry
    pub fn new_second(inner: I) -> Self {
        Self { inner }
    }
}

impl<F, S, I: Iterator<Item = (F, S)>> Iterator for TupleExtract<F, S, I, true> {
    type Item = F;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(f, _s)| f)
    }
}

impl<F, S, I: Iterator<Item = (F, S)>> Iterator for TupleExtract<F, S, I, false> {
    type Item = S;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_f, s)| s)
    }
}
