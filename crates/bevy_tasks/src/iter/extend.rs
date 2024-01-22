use std::collections::LinkedList;

use crate::{Consumer, Folder, Reducer, UnindexedConsumer};

use super::{IntoParallelIterator, ParallelExtend, ParallelIterator};

/// Extends a vector with items from a parallel iterator.
impl<T> ParallelExtend<T> for Vec<T>
where
    T: Send,
{
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = T>,
    {
        let par_iter = par_iter.into_par_iter();
        match par_iter.opt_len() {
            Some(len) => {
                // When Rust gets specialization, we can get here for indexed iterators
                // without relying on `opt_len`.  Until then, `special_extend()` fakes
                // an unindexed mode on the promise that `opt_len()` is accurate.
                crate::iter::collect::special_extend(par_iter, len, self);
            }
            None => {
                // This works like `extend`, but `Vec::append` is more efficient.
                let list = par_iter.drive_unindexed(ListVecConsumer);
                vec_append(self, list);
            }
        }
    }
}

fn len<T>(list: &LinkedList<Vec<T>>) -> usize {
    list.iter().map(Vec::len).sum()
}

fn vec_append<T>(vec: &mut Vec<T>, list: LinkedList<Vec<T>>) {
    vec.reserve(len(&list));
    for mut other in list {
        vec.append(&mut other);
    }
}

struct ListVecConsumer;
struct ListVecFolder<T> {
    vec: Vec<T>,
}
struct ListReducer;

impl<T> Reducer<LinkedList<T>> for ListReducer {
    fn reduce(self, mut left: LinkedList<T>, mut right: LinkedList<T>) -> LinkedList<T> {
        left.append(&mut right);
        left
    }
}
impl<T: Send> Consumer<T> for ListVecConsumer {
    type Folder = ListVecFolder<T>;
    type Reducer = ListReducer;
    type Result = LinkedList<Vec<T>>;

    fn split_at(self, _index: usize) -> (Self, Self, Self::Reducer) {
        (Self, Self, ListReducer)
    }

    fn into_folder(self) -> Self::Folder {
        ListVecFolder { vec: Vec::new() }
    }

    fn full(&self) -> bool {
        false
    }
}

impl<T: Send> UnindexedConsumer<T> for ListVecConsumer {
    fn split_off_left(&self) -> Self {
        Self
    }

    fn to_reducer(&self) -> Self::Reducer {
        ListReducer
    }
}

impl<T> Folder<T> for ListVecFolder<T> {
    type Result = LinkedList<Vec<T>>;

    fn consume(mut self, item: T) -> Self {
        self.vec.push(item);
        self
    }

    fn consume_iter<I>(mut self, iter: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        self.vec.extend(iter);
        self
    }

    fn complete(self) -> Self::Result {
        let mut list = LinkedList::new();
        if !self.vec.is_empty() {
            list.push_back(self.vec);
        }
        list
    }

    fn full(&self) -> bool {
        false
    }
}
