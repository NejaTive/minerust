use super::*;

const A: u64 = 8;

#[test]
fn alloc_and_free_roundtrip() {
    let mut a = RangeAllocator::new(1024, A);
    let x = a.alloc(100).unwrap();
    let y = a.alloc(200).unwrap();
    assert_ne!(x, y);
    assert_eq!(a.used(), 104 + 200);
    a.free(x, 100);
    a.free(y, 200);
    assert_eq!(a.used(), 0);
    assert_eq!(a.free_range_count(), 1);
    assert_eq!(a.largest_free(), 1024);
}

#[test]
fn ranges_are_aligned_and_disjoint() {
    let mut a = RangeAllocator::new(16384, A);
    let mut taken: Vec<(u64, u64)> = (1..40)
        .map(|i| {
            let size = i * 13;
            (a.alloc(size).unwrap(), size.next_multiple_of(A))
        })
        .collect();
    taken.sort();
    for pair in taken.windows(2) {
        assert!(pair[0].0 + pair[0].1 <= pair[1].0);
    }
    assert!(taken.iter().all(|(offset, _)| offset % A == 0));
}

#[test]
fn full_arena_refuses() {
    let mut a = RangeAllocator::new(64, A);
    assert!(a.alloc(64).is_some());
    assert!(a.alloc(8).is_none());
}

#[test]
fn neighbours_merge_in_any_order() {
    let mut a = RangeAllocator::new(96, A);
    let r: Vec<u64> = (0..3).map(|_| a.alloc(32).unwrap()).collect();
    a.free(r[0], 32);
    a.free(r[2], 32);
    assert_eq!(a.free_range_count(), 2);
    a.free(r[1], 32);
    assert_eq!(a.free_range_count(), 1);
    assert_eq!(a.largest_free(), 96);
    assert!(a.alloc(96).is_some());
}

#[test]
fn freed_range_is_reused() {
    let mut a = RangeAllocator::new(256, A);
    let x = a.alloc(64).unwrap();
    let _y = a.alloc(64).unwrap();
    a.free(x, 64);
    assert_eq!(a.alloc(64), Some(x));
}

#[test]
fn best_fit_prefers_smallest_hole() {
    let mut a = RangeAllocator::new(512, A);
    let big = a.alloc(200).unwrap();
    let _gap1 = a.alloc(8).unwrap();
    let small = a.alloc(50).unwrap();
    let _gap2 = a.alloc(8).unwrap();
    a.free(big, 200);
    a.free(small, 50);
    assert_eq!(a.alloc(40), Some(small));
}

#[test]
fn grow_extends_free_tail() {
    let mut a = RangeAllocator::new(128, A);
    let x = a.alloc(64).unwrap();
    assert!(a.alloc(128).is_none());
    a.grow(256);
    assert_eq!(a.capacity(), 256);
    assert_eq!(a.largest_free(), 192);
    assert!(a.alloc(192).is_some());
    a.free(x, 64);
    assert_eq!(a.used(), 192);
}

#[test]
fn grow_from_full_arena() {
    let mut a = RangeAllocator::new(64, A);
    a.alloc(64).unwrap();
    a.grow(128);
    assert_eq!(a.alloc(64), Some(64));
}

fn next(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

#[test]
fn random_load_keeps_invariants_and_defragments_on_release() {
    let capacity = 1 << 20;
    let mut a = RangeAllocator::new(capacity, A);
    let mut live: Vec<(u64, u64)> = Vec::new();
    let mut state = 0x9E37_79B9_7F4A_7C15;
    for _ in 0..20_000 {
        if live.len() > 300 || (!live.is_empty() && next(&mut state).is_multiple_of(2)) {
            let i = (next(&mut state) % live.len() as u64) as usize;
            let (offset, size) = live.swap_remove(i);
            a.free(offset, size);
        } else {
            let size = 8 + next(&mut state) % 6000;
            if let Some(offset) = a.alloc(size) {
                live.push((offset, size));
            }
        }
        let expected: u64 = live.iter().map(|&(_, s)| s.next_multiple_of(A)).sum();
        assert_eq!(a.used(), expected);
    }
    let mut sorted = live.clone();
    sorted.sort();
    for pair in sorted.windows(2) {
        assert!(pair[0].0 + pair[0].1.next_multiple_of(A) <= pair[1].0);
    }
    for (offset, size) in live {
        a.free(offset, size);
    }
    assert_eq!(a.used(), 0);
    assert_eq!(a.free_range_count(), 1);
    assert_eq!(a.largest_free(), capacity);
}

#[test]
fn grown_capacity_adds_a_quarter_until_it_fits() {
    assert_eq!(grown_capacity(32, 33, 1 << 30, A), Some(40));
    assert_eq!(
        grown_capacity(32, 200, 1 << 30, A).map(|c| c >= 200),
        Some(true)
    );
    assert_eq!(grown_capacity(1024, 1025, 1 << 30, A), Some(1280));
}

#[test]
fn grown_capacity_stops_at_limit() {
    assert_eq!(grown_capacity(64, 100, 100, A), None);
    assert_eq!(grown_capacity(64, 90, 100, A), Some(96));
    assert_eq!(grown_capacity(96, 200, 100, A), None);
}
