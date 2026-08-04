use crate::arena::Bump;
use crate::error::EtfError;
use crate::limits::Limits;
use core::mem::MaybeUninit;

#[kani::proof]
fn align_up_rounds_to_alignment() {
    let addr: usize = kani::any();
    let align = core::mem::align_of::<u128>();
    kani::assume(addr <= usize::MAX - align);
    let up = crate::arena::align_up(addr, align);
    assert_eq!(up % align, 0);
    assert!(up >= addr);
    assert!(up < addr + align);
}

#[kani::proof]
fn bump_alloc_is_aligned() {
    let mut buf = [MaybeUninit::<u8>::uninit(); 64];
    let limits = Limits::default();
    let mut b = Bump::new(&mut buf, &limits);
    let s = b.alloc_slice::<u64>(1).expect("64-byte arena fits one u64");
    assert_eq!(s.len(), 1);
    assert_eq!(s.as_ptr() as usize % core::mem::align_of::<u64>(), 0);
}

#[kani::proof]
fn bump_alloc_respects_capacity() {
    let mut buf = [MaybeUninit::<u8>::uninit(); 16];
    let limits = Limits::default();
    let mut b = Bump::new(&mut buf, &limits);
    assert_eq!(b.alloc_slice::<u8>(17), Err(EtfError::ArenaExhausted));
}
