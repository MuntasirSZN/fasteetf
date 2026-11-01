use crate::arena::AllocationState;
use crate::error::EtfError;

#[kani::proof]
fn allocation_respects_alignment() {
    let mut state = AllocationState::new(Some(64));
    let (offset, layout) = state.reserve::<u64>(1).expect("64-byte arena fits one u64");
    assert_eq!(offset % layout.align(), 0);
}

#[kani::proof]
fn allocation_respects_capacity() {
    let mut state = AllocationState::new(Some(16));
    assert_eq!(state.reserve::<u8>(17), Err(EtfError::ArenaExhausted));
}
