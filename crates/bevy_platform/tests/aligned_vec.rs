//! Tests for [`bevy_platform::collections::AlignedVec`]
#![expect(unsafe_code, reason = "The Test need to interact with raw memory")]
#![expect(
    clippy::undocumented_unsafe_blocks,
    reason = "This is a test and it is simple enough that the safety contract is self-evident"
)]

extern crate alloc;
use bevy_platform::collections::AlignedVec;

#[test]
fn test_construction() {
    let v = AlignedVec::new(16);
    assert_eq!(v.capacity(), 0);
    assert_eq!(v.len(), 0);
    assert!(v.is_empty());

    let v = AlignedVec::with_capacity(16, 10);
    assert_eq!(v.capacity(), 10);
    assert_eq!(v.len(), 0);

    let v = AlignedVec::with_capacity(16, 0);
    assert_eq!(v.capacity(), 0);
}

#[test]
fn test_push_and_pop() {
    let mut v = AlignedVec::new(16);
    v.push(1);
    assert!(!v.is_empty());
    v.push(2);
    assert_eq!(v.len(), 2);

    assert_eq!(v.pop(), Some(2));
    assert_eq!(v.pop(), Some(1));
    assert_eq!(v.pop(), None);
    assert!(v.is_empty());

    v.extend_from_slice(&[3, 4, 5]);
    assert_eq!(v.pop(), Some(5));
    assert_eq!(v.pop(), Some(4));
    assert_eq!(v.pop(), Some(3));
    assert_eq!(v.pop(), None);

    let mut v = AlignedVec::new(16);
    for i in 0..1024 {
        v.push(i as u8);
    }
    assert_eq!(v.len(), 1024);
    assert!(v.capacity().is_power_of_two());
    for i in 0..1024 {
        assert_eq!(v[i], (i % 256) as u8);
    }
}

#[test]
fn test_indexing() {
    let mut v = AlignedVec::new(16);
    v.extend_from_slice(&[10, 20, 30]);
    assert_eq!(v[0], 10);
    assert_eq!(v[1], 20);
    assert_eq!(v[2], 30);

    let mut v = AlignedVec::new(16);
    v.extend_from_slice(&[1, 2, 3, 4, 5]);
    assert_eq!(&v[..], &[1, 2, 3, 4, 5]);
    assert_eq!(&v[1..3], &[2, 3]);
    assert_eq!(&v[2..], &[3, 4, 5]);
    assert_eq!(&v[..3], &[1, 2, 3]);

    for elem in v.iter_mut() {
        *elem += 10;
    }
    assert_eq!(&*v, &[11, 12, 13, 14, 15]);
}

#[test]
#[should_panic]
fn test_index_out_of_bounds() {
    let v = AlignedVec::new(16);
    let _ = v[0];
}

#[test]
#[should_panic]
fn test_slice_out_of_bounds() {
    let v = AlignedVec::new(16);
    let _ = &v[..1];
}

#[test]
fn test_clear() {
    let mut v = AlignedVec::new(16);
    v.extend_from_slice(&[1, 2, 3, 4]);
    v.clear();
    assert!(v.is_empty());
    assert!(v.capacity() >= 4);
    v.push(5);
    assert_eq!(&*v, &[5]);
}

#[test]
fn test_clone() {
    let v = AlignedVec::new(16);
    let w = v.clone();
    assert!(w.is_empty());
    assert_eq!(w.capacity(), 0);

    let mut v = AlignedVec::new(16);
    v.extend_from_slice(&[1, 2, 3]);
    let w = v.clone();
    assert_eq!(&*v, &*w);
    assert_ne!(
        v.as_ptr(),
        w.as_ptr(),
        "clone should produce disjoint storage"
    );
}

#[test]
fn test_deref() {
    let mut v = AlignedVec::new(16);
    v.extend_from_slice(&[1, 2, 3]);
    let s: &[u8] = &v;
    assert_eq!(s, &[1, 2, 3]);

    {
        let s: &mut [u8] = &mut v;
        s[0] = 10;
    }
    assert_eq!(v[0], 10);
}

#[test]
fn test_as_ref_and_borrow() {
    let mut v = AlignedVec::new(16);
    v.push(42);
    assert_eq!(AsRef::<[u8]>::as_ref(&v), &[42]);
    AsMut::<[u8]>::as_mut(&mut v)[0] = 99;
    assert_eq!(v[0], 99);

    let mut v = AlignedVec::new(16);
    v.push(7);
    use core::borrow::{Borrow, BorrowMut};
    let s: &[u8] = Borrow::<[u8]>::borrow(&v);
    assert_eq!(s, &[7]);
    BorrowMut::<[u8]>::borrow_mut(&mut v)[0] = 8;
    assert_eq!(v[0], 8);
}

#[test]
fn test_raw_pointers() {
    let mut v = AlignedVec::new(16);
    v.extend_from_slice(&[1, 2, 3, 4, 5]);

    assert_eq!(v.as_slice(), &[1, 2, 3, 4, 5]);
    v.as_mut_slice()[0] = 99;
    assert_eq!(v.as_slice()[0], 99);

    let ptr = v.as_ptr();
    for i in 0..3 {
        assert_eq!(
            unsafe { *ptr.add(i) },
            if i == 0 { 99 } else { (i + 1) as u8 }
        );
    }

    let mut_ptr = v.as_mut_ptr();
    unsafe { *mut_ptr.add(0) = 10 }
    assert_eq!(v[0], 10);
}

#[test]
fn test_into_raw_parts() {
    let v = AlignedVec::new(1);
    let (ptr, align, len, cap) = v.into_raw_parts();
    assert!(align == 1 && len == 0 && cap == 0);
    let rebuilt = unsafe { AlignedVec::from_raw_parts(ptr, align, len, cap) };
    assert!(align == rebuilt.alignment() && len == rebuilt.len() && cap == rebuilt.capacity());

    let mut v = AlignedVec::new(16);
    for i in 1..=5 {
        v.push(i);
    }
    let (ptr, align, len, cap) = v.into_raw_parts();
    assert!(align == 16 && len == 5 && cap >= len);
    let rebuilt = unsafe { AlignedVec::from_raw_parts(ptr, align, len, cap) };
    assert_eq!(&*rebuilt, &[1, 2, 3, 4, 5]);
}

#[test]
fn test_iteration() {
    let mut v = AlignedVec::new(16);
    v.extend_from_slice(&[1, 2, 3]);

    let mut collected = Vec::new();
    for &b in v.iter() {
        collected.push(b);
    }
    assert_eq!(collected, vec![1, 2, 3]);

    for b in v.iter_mut() {
        *b *= 2;
    }
    assert_eq!(&*v, &[2, 4, 6]);

    let collected: Vec<u8> = v.iter().copied().collect();
    assert_eq!(collected, vec![2, 4, 6]);
}

#[test]
fn test_extend_from_slice() {
    let mut v = AlignedVec::new(16);
    v.extend_from_slice(&[]);
    assert!(v.is_empty());

    v.push(1);
    v.extend_from_slice(&[]);
    assert_eq!(&*v, &[1]);

    let mut w = AlignedVec::new(16);
    w.extend_from_slice(&[6, 7, 8, 9, 0]);
    v.extend_from_slice(&w);
    assert_eq!(&*v, &[1, 6, 7, 8, 9, 0]);
}

#[test]
fn test_resize() {
    let mut v = AlignedVec::new(16);
    v.push(3);
    v.resize(3, 2);
    assert_eq!(&*v, &[3, 2, 2]);

    let mut v = AlignedVec::new(16);
    v.extend_from_slice(&[1, 2, 3, 4]);
    v.resize(2, 0);
    assert_eq!(&*v, &[1, 2]);

    v.resize(2, 99);
    assert_eq!(&*v, &[1, 2]);
}

#[test]
fn test_capacity_management() {
    let mut v = AlignedVec::new(16);
    assert_eq!(v.capacity(), 0);

    v.reserve(2);
    assert!(v.capacity() >= 2);
    for i in 0..16 {
        v.push(i);
    }
    assert!(v.capacity() >= 16);
    v.reserve(16);
    assert!(v.capacity() >= 32);

    let mut v = AlignedVec::new(16);
    v.reserve_exact(2);
    assert!(v.capacity() >= 2);
    for i in 0..16 {
        v.push(i);
    }
    assert!(v.capacity() >= 16);
    v.reserve_exact(16);
    assert!(v.capacity() >= 32);

    let mut v = AlignedVec::with_capacity(16, 10);
    v.extend_from_slice(&[1, 2, 3]);
    v.shrink_to_fit();
    assert!(v.capacity() >= 3);
    v.clear();
    v.shrink_to_fit();
    assert_eq!(v.capacity(), 0);
}

#[test]
#[should_panic]
fn test_reserve_exact_overflow() {
    let mut v = AlignedVec::new(16);
    v.reserve_exact(isize::MAX as usize - (16 - 1) + 1);
}

#[test]
fn test_alignment() {
    for align in [1, 2, 4, 8, 16, 64, 256, 4096, 65536] {
        let mut v = AlignedVec::new(align);
        v.push(42);
        assert_eq!(
            v.as_ptr().align_offset(align),
            0,
            "pointer {:#x} should be {align}-aligned",
            v.as_ptr() as usize
        );
        assert_eq!(v.alignment(), align);
    }

    let align = 4096;
    let mut v = AlignedVec::new(align);
    for i in 0..100 {
        v.push(i as u8);
    }
    assert_eq!(v.as_ptr() as usize % align, 0);
    assert_eq!(v.alignment(), align);
}

#[test]
#[should_panic(expected = "align must be a power of 2")]
fn test_new_non_power_of_two_alignment() {
    let _ = AlignedVec::new(3);
}

#[test]
#[should_panic(expected = "align must be 1 or more")]
fn test_new_zero_alignment() {
    let _ = AlignedVec::new(0);
}

#[test]
fn test_debug_fmt() {
    let v = AlignedVec::new(16);
    assert_eq!("[]", format!("{:?}", v));

    let mut v = AlignedVec::new(16);
    v.extend_from_slice(&[0, 1, 255]);
    assert_eq!("[0, 1, 255]", format!("{:?}", v));
}

#[test]
fn test_leak() {
    let mut v = AlignedVec::new(16);
    v.extend_from_slice(&[1, 2, 3]);
    let leaked: &'static mut [u8] = v.leak();
    assert_eq!(leaked, &[1, 2, 3]);
    leaked[0] += 1;
    assert_eq!(leaked, &[2, 2, 3]);
    let layout = core::alloc::Layout::from_size_align(leaked.len(), 16).unwrap();
    unsafe {
        alloc::alloc::dealloc(leaked.as_mut_ptr(), layout);
    }
}

#[test]
fn test_send_sync() {
    fn assert_send<T: Send>(_: &T) {}
    fn assert_sync<T: Sync>(_: &T) {}

    let v = AlignedVec::new(16);
    assert_send(&v);
    assert_sync(&v);
}

#[cfg(feature = "bytemuck")]
#[test]
fn test_bytemuck_cast_slice() {
    let mut v = AlignedVec::new(2);
    v.extend_from_slice(&[0x01, 0x02, 0x03, 0x04]);

    let cast: &[u16] = v.cast_slice();
    assert_eq!(cast, &[0x0201, 0x0403]);

    let cast_mut: &mut [u16] = v.cast_slice_mut();
    cast_mut[0] = 0x0A0B;
    assert_eq!(&*v, &[0x0B, 0x0A, 0x03, 0x04]);

    #[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug, PartialEq, Eq)]
    #[repr(C, align(256))]
    struct A256 {
        a: u8,
        pad: [u8; 255],
    }
    let mut v = AlignedVec::new(256);
    // Casting empty slice should success
    let _: &[A256] = v.cast_slice();
    v.extend_from_slice(&[40]);
    v.extend_from_slice(&[0; 255]);
    v.extend_from_slice(&[1]);
    v.extend_from_slice(&[0; 255]);
    assert_eq!(
        v.cast_slice::<A256>(),
        &[
            A256 {
                a: 40,
                ..bytemuck::Zeroable::zeroed()
            },
            A256 {
                a: 1,
                ..bytemuck::Zeroable::zeroed()
            }
        ]
    );
    v.cast_slice_mut()[1] = A256 {
        a: 20,
        ..bytemuck::Zeroable::zeroed()
    };
    assert_eq!(
        v.cast_slice::<A256>(),
        &[
            A256 {
                a: 40,
                ..bytemuck::Zeroable::zeroed()
            },
            A256 {
                a: 20,
                ..bytemuck::Zeroable::zeroed()
            }
        ]
    );
}

#[cfg(feature = "bytemuck")]
#[test]
#[should_panic]
fn test_bytemuck_cast_slice_align_too_large() {
    let v = AlignedVec::new(1);
    let _: &[u16] = v.cast_slice();
}

#[cfg(feature = "bytemuck")]
#[test]
fn test_bytemuck_into_vec() {
    let mut v = AlignedVec::new(2);
    v.extend_from_slice(&[0x01, 0x02, 0x03, 0x04]);

    let vec: Vec<u16> = v.into_vec();
    assert_eq!(vec, &[0x0201, 0x0403]);
}

#[cfg(feature = "bytemuck")]
#[test]
fn test_bytemuck_from_vec() {
    let v: Vec<u16> = vec![0x0201, 0x0403];
    let aligned: AlignedVec = v.into();
    assert_eq!(aligned.alignment(), 2);
    assert_eq!(&*aligned, &[0x01, 0x02, 0x03, 0x04]);
}
