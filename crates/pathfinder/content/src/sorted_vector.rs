// pathfinder/content/src/sorted_vector.rs
//
// Copyright © 2020 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A vector that maintains sorted order with insertion sort.

use std::cmp::Ordering;
use std::convert;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct SortedVector<T>
where
    T: PartialOrd,
{
    pub array: Vec<T>,
}

impl<T> SortedVector<T>
where
    T: PartialOrd,
{
    #[inline]
    pub fn new() -> SortedVector<T> {
        SortedVector { array: vec![] }
    }

    #[inline]
    pub fn push(&mut self, value: T) {
        let index = self.binary_search_by(|other| {
            other.partial_cmp(&value).unwrap_or(Ordering::Less)
        }).unwrap_or_else(convert::identity);
        self.array.insert(index, value);
    }

    #[inline]
    pub fn peek(&self) -> Option<&T> {
        self.array.last()
    }

    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        self.array.pop()
    }

    #[inline]
    pub fn clear(&mut self) {
        self.array.clear()
    }

    #[allow(dead_code)]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.array.is_empty()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.array.len()
    }

    #[inline]
    pub fn binary_search_by<'a, F>(&'a self, f: F) -> Result<usize, usize>
                                   where F: FnMut(&'a T) -> Ordering {
        self.array.binary_search_by(f)
    }
}

#[cfg(test)]
mod test {
    use crate::sorted_vector::SortedVector;
    use quickcheck;

    #[test]
    fn test_sorted_vec() {
        quickcheck::quickcheck(prop_sorted_vec as fn(Vec<i32>) -> bool);

        fn prop_sorted_vec(mut values: Vec<i32>) -> bool {
            let mut sorted_vec = SortedVector::new();
            for &value in &values {
                sorted_vec.push(value)
            }

            values.sort();
            let mut results = Vec::with_capacity(values.len());
            while !sorted_vec.is_empty() {
                results.push(sorted_vec.pop().unwrap());
            }
            results.reverse();
            assert_eq!(&values, &results);

            true
        }
    }
}
