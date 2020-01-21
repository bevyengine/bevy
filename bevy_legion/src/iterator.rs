use std::iter::repeat;
use std::iter::ExactSizeIterator;
use std::iter::FusedIterator;
use std::iter::Repeat;
use std::iter::Take;
use std::slice::Iter;

/// An iterator over slices in a `SliceVec`.
#[derive(Clone)]
pub struct SliceVecIter<'a, T> {
    pub(crate) data: &'a [T],
    pub(crate) counts: &'a [usize],
}

impl<'a, T> Iterator for SliceVecIter<'a, T> {
    type Item = &'a [T];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((count, remaining_counts)) = self.counts.split_first() {
            let (data, remaining_data) = self.data.split_at(*count);
            self.counts = remaining_counts;
            self.data = remaining_data;
            Some(data)
        } else {
            None
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) { (self.counts.len(), Some(self.counts.len())) }

    #[inline]
    fn count(self) -> usize { self.len() }
}

impl<'a, T> ExactSizeIterator for SliceVecIter<'a, T> {}
impl<'a, T> FusedIterator for SliceVecIter<'a, T> {}

/// A trait for iterators that are able to be split in roughly half.
/// Used for splitting work among threads in parallel iterator.
pub trait FissileIterator: Iterator + Sized {
    /// Divides one iterator into two, roughly in half.
    ///
    /// The implementation doesn't have to be precise,
    /// but the closer to the midpoint it is, the better
    /// the parallel iterator will behave.
    ///
    /// Returns two split iterators and a number of elements left in first split.
    /// That returned size must be exact.
    fn split(self) -> (Self, Self, usize);
}

impl<'a, T> FissileIterator for Iter<'a, T> {
    fn split(self) -> (Self, Self, usize) {
        let slice = self.as_slice();
        let split_point = slice.len() / 2;
        let (left_slice, right_slice) = slice.split_at(split_point);
        (left_slice.iter(), right_slice.iter(), split_point)
    }
}

impl<'a, T> FissileIterator for SliceVecIter<'a, T> {
    fn split(self) -> (Self, Self, usize) {
        let counts_split_point = self.counts.len() / 2;
        let (left_counts, right_counts) = self.counts.split_at(counts_split_point);
        let data_split_point = left_counts.iter().sum();
        let (left_data, right_data) = self.data.split_at(data_split_point);
        (
            Self {
                data: left_data,
                counts: left_counts,
            },
            Self {
                data: right_data,
                counts: right_counts,
            },
            counts_split_point,
        )
    }
}

pub(crate) struct FissileEnumerate<I: FissileIterator> {
    iter: I,
    count: usize,
}
impl<I: FissileIterator> FissileEnumerate<I> {
    pub(crate) fn new(iter: I) -> Self { Self { iter, count: 0 } }
}
impl<I: FissileIterator> Iterator for FissileEnumerate<I>
where
    I: Iterator,
{
    type Item = (usize, <I as Iterator>::Item);

    #[inline]
    fn next(&mut self) -> Option<(usize, <I as Iterator>::Item)> {
        self.iter.next().map(|a| {
            let ret = (self.count, a);
            self.count += 1;
            ret
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) { self.iter.size_hint() }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<(usize, I::Item)> {
        self.iter.nth(n).map(|a| {
            let i = self.count + n;
            self.count = i + 1;
            (i, a)
        })
    }

    #[inline]
    fn count(self) -> usize { self.iter.count() }

    #[inline]
    fn fold<Acc, Fold>(self, init: Acc, mut fold: Fold) -> Acc
    where
        Fold: FnMut(Acc, Self::Item) -> Acc,
    {
        let mut count = self.count;
        self.iter.fold(init, move |acc, item| {
            let acc = fold(acc, (count, item));
            count += 1;
            acc
        })
    }
}

impl<I: FissileIterator> FissileIterator for FissileEnumerate<I> {
    fn split(self) -> (Self, Self, usize) {
        let (left, right, left_size) = self.iter.split();
        (
            Self {
                iter: left,
                count: self.count,
            },
            Self {
                iter: right,
                count: self.count + left_size,
            },
            left_size,
        )
    }
}

impl<I: ExactSizeIterator + FissileIterator> ExactSizeIterator for FissileEnumerate<I> {
    fn len(&self) -> usize { self.iter.len() }
}

impl<I: FusedIterator + FissileIterator> FusedIterator for FissileEnumerate<I> {}

impl<T: Clone> FissileIterator for Take<Repeat<T>> {
    fn split(mut self) -> (Self, Self, usize) {
        if let Some(value) = self.next() {
            let (len, len_max) = self.size_hint();
            assert_eq!(Some(len), len_max);

            let first_part = len / 2;
            let second_part = len - first_part;
            (
                repeat(value.clone()).take(first_part),
                repeat(value).take(second_part),
                first_part,
            )
        } else {
            (self.clone(), self, 0)
        }
    }
}

// Custom fissile zip iterator. Assumes that it's child iterators will always
// split in the same location. Panics when this is violated.
pub struct FissileZip<A, B> {
    a: A,
    b: B,
}

impl<A, B> FissileZip<A, B> {
    pub(crate) fn new(a: A, b: B) -> Self { Self { a, b } }
}

impl<A: Iterator, B: Iterator> Iterator for FissileZip<A, B> {
    type Item = (A::Item, B::Item);
    fn next(&mut self) -> Option<(A::Item, B::Item)> {
        self.a.next().and_then(|x| self.b.next().map(|y| (x, y)))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (a_lower, a_upper) = self.a.size_hint();
        let (b_lower, b_upper) = self.b.size_hint();

        let lower = std::cmp::min(a_lower, b_lower);

        let upper = match (a_upper, b_upper) {
            (Some(x), Some(y)) => Some(std::cmp::min(x, y)),
            (Some(x), None) => Some(x),
            (None, Some(y)) => Some(y),
            (None, None) => None,
        };

        (lower, upper)
    }
}

impl<A: FissileIterator, B: FissileIterator> FissileIterator for FissileZip<A, B> {
    fn split(self) -> (Self, Self, usize) {
        let (a_left, a_right, a_left_size) = self.a.split();
        let (b_left, b_right, b_left_size) = self.b.split();
        assert_eq!(a_left_size, b_left_size);
        (
            Self::new(a_left, b_left),
            Self::new(a_right, b_right),
            a_left_size,
        )
    }
}
