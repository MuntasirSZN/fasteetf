use crate::arena::Bump;
use crate::error::EtfError;
use crate::limits::Limits;

#[kani::proof]
fn bump_alloc_is_aligned() {
    let raw = bumpalo::Bump::<16>::with_min_align_and_capacity(64);
    raw.set_allocation_limit(Some(64));
    let limits = Limits::default();
    let mut b = Bump::new(&raw, &limits);
    let s = b.alloc_slice::<u64>(1).expect("64-byte arena fits one u64");
    assert_eq!(s.len(), 1);
    assert_eq!(s.as_ptr() as usize % core::mem::align_of::<u64>(), 0);
}

#[kani::proof]
fn bump_alloc_respects_capacity() {
    let raw = bumpalo::Bump::<16>::with_min_align_and_capacity(16);
    raw.set_allocation_limit(Some(16));
    let limits = Limits::default();
    let mut b = Bump::new(&raw, &limits);
    assert_eq!(b.alloc_slice::<u8>(17), Err(EtfError::ArenaExhausted));
}
