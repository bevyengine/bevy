use std::ops::Index;

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
///
/// Implements a forwards/backwards slice which can be
/// used similar to any other slice.
///
pub enum Slice<'a, T> {
    Forward(&'a [T]),
    Backward(&'a [T]),
}

impl<'a, T> Slice<'a, T> {
    ///
    /// The length of the underlying slice.
    ///
    pub fn len(&self) -> usize {
        match self {
            &Slice::Forward(x) | &Slice::Backward(x) => x.len(),
        }
    }
}

impl<'a, T> Index<usize> for Slice<'a, T> {
    type Output = <[T] as Index<usize>>::Output;

    fn index(&self, idx: usize) -> &Self::Output {
        match self {
            Slice::Forward(x) => x.index(idx),
            Slice::Backward(x) => x.index((x.len() - 1) - idx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Slice;
    #[test]
    fn test_slice() {
        let original = vec![0, 1, 2, 3, 4];
        let slice = &original[..];

        let forward = Slice::Forward(slice);
        assert_eq!(forward[0], 0);
        assert_eq!(forward[forward.len() - 1], 4);
        assert_eq!(forward.len(), 5);

        let backward = Slice::Backward(slice);
        assert_eq!(forward[0], 4);
        assert_eq!(forward[forward.len() - 1], 0);
        assert_eq!(forward.len(), 5);
    }
}
