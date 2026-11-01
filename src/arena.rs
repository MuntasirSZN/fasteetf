use core::alloc::Layout;

use crate::error::EtfError;
use crate::limits::Limits;

/// Thin wrapper around `bumpalo` used to build the parsed term tree.
pub(crate) struct Bump<'a> {
    arena: &'a bumpalo::Bump<16>,
    limits: *const Limits,
    cursor: usize,
    logical_limit: Option<usize>,
}

impl<'a> Bump<'a> {
    pub(crate) fn new(arena: &'a bumpalo::Bump<16>, limits: &Limits) -> Self {
        Self {
            arena,
            limits,
            cursor: 0,
            logical_limit: arena.allocation_limit(),
        }
    }

    #[inline(always)]
    pub(crate) fn limits(&self) -> &Limits {
        // SAFETY: `limits` is caller-owned and guaranteed to outlive parsing.
        unsafe { &*self.limits }
    }

    #[inline(always)]
    pub(crate) fn alloc_slice<T>(&mut self, len: usize) -> Result<&'a mut [T], EtfError> {
        let layout = Layout::array::<T>(len).map_err(|_| EtfError::ArenaExhausted)?;
        let aligned_cursor = self
            .cursor
            .checked_add(layout.align() - 1)
            .ok_or(EtfError::ArenaExhausted)?
            & !(layout.align() - 1);
        let logical_end = aligned_cursor
            .checked_add(layout.size())
            .ok_or(EtfError::ArenaExhausted)?;
        if self
            .logical_limit
            .is_some_and(|limit| logical_end > limit)
        {
            return Err(EtfError::ArenaExhausted);
        }

        let ptr = self
            .arena
            .try_alloc_layout(layout)
            .map_err(|_| EtfError::ArenaExhausted)?;
        self.cursor = logical_end;
        // SAFETY: `try_alloc_layout` returns a valid allocation for `layout`.
        Ok(unsafe { core::slice::from_raw_parts_mut(ptr.as_ptr() as *mut T, len) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_is_aligned() {
        let raw = bumpalo::Bump::<16>::with_min_align_and_capacity(64);
        raw.set_allocation_limit(Some(64));
        let limits = Limits::default();
        let mut bump = Bump::new(&raw, &limits);
        let s = bump.alloc_slice::<u64>(1).expect("alloc should succeed");
        assert_eq!(s.as_ptr() as usize % core::mem::align_of::<u64>(), 0);
    }

    #[test]
    fn exhausted_when_size_exceeds_remaining() {
        let raw = bumpalo::Bump::<16>::with_min_align_and_capacity(16);
        raw.set_allocation_limit(Some(16));
        let limits = Limits::default();
        let mut bump = Bump::new(&raw, &limits);
        let err = bump.alloc_slice::<u8>(17).expect_err("alloc should fail");
        assert!(matches!(err, EtfError::ArenaExhausted));
    }
}
