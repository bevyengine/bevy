use legion::iterator::*;
use legion::storage::SliceVec;

#[test]
fn slice_vec_iterate() {
    let mut vec = SliceVec::default();
    vec.push(vec![1, 2, 3, 4]);
    vec.push(vec![5, 6, 7, 8]);
    vec.push(vec![9, 10]);
    let mut iter = vec.iter();
    assert_eq!(Some(&[1, 2, 3, 4][..]), iter.next());
    assert_eq!(Some(&[5, 6, 7, 8][..]), iter.next());
    assert_eq!(Some(&[9, 10][..]), iter.next());
    assert_eq!(None, iter.next());
}

#[test]
fn slice_vec_iterator_split() {
    let mut vec = SliceVec::default();
    vec.push(vec![1, 2, 3, 4]);
    vec.push(vec![5, 6, 7, 8]);
    vec.push(vec![9, 10]);

    let (mut left, mut right, left_len) = vec.iter().split();
    assert_eq!(left_len, 1);

    assert_eq!(Some(&[1, 2, 3, 4][..]), left.next());
    assert_eq!(None, left.next());
    assert_eq!(Some(&[5, 6, 7, 8][..]), right.next());
    assert_eq!(Some(&[9, 10][..]), right.next());
    assert_eq!(None, right.next());
}
