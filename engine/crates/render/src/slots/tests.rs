use super::*;

#[test]
fn slots_are_unique_until_capacity() {
    let mut s = SlotAllocator::new(3);
    let got: Vec<u32> = (0..3).map(|_| s.alloc().unwrap()).collect();
    assert_eq!(got, [0, 1, 2]);
    assert_eq!(s.alloc(), None);
    assert_eq!(s.used(), 3);
}

#[test]
fn freed_slot_is_reused_before_new_ones() {
    let mut s = SlotAllocator::new(4);
    let a = s.alloc().unwrap();
    let _b = s.alloc().unwrap();
    s.free(a);
    assert_eq!(s.alloc(), Some(a));
    assert_eq!(s.used(), 2);
}

#[test]
fn growth_adds_slots() {
    let mut s = SlotAllocator::new(1);
    s.alloc().unwrap();
    assert_eq!(s.alloc(), None);
    s.grow(4);
    assert_eq!(s.capacity(), 4);
    assert_eq!(s.alloc(), Some(1));
}
