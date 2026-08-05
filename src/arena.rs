use core::marker::PhantomData;
use core::mem::MaybeUninit;

use crate::error::EtfError;
use crate::limits::Limits;

/// Round `addr` up to the next multiple of `align`.
///
/// `align` must be a non-zero power of two.  Returns the smallest `x >= addr`
/// with `x % align == 0`; the caller must ensure `addr` is not within
/// `align - 1` bytes of `usize::MAX` (the adjustment would otherwise wrap).
#[inline(always)]
pub(crate) fn align_up(addr: usize, align: usize) -> usize {
    let misalignment = addr & (align - 1);
    let adj = if misalignment == 0 {
        0
    } else {
        align - misalignment
    };
    addr + adj
}

/// A simple bump allocator used to build the AST from a pre-allocated scratch buffer.
///
/// The initial pointer is aligned to `max_align_t` (typically 16 bytes) so
/// that `alloc_slice::<Term>` and `alloc_slice::<(Term, Term)>` never need
/// alignment arithmetic on the hot path.
///
/// Resource limits (including the recursion depth budget) are accessed
/// directly through the caller-supplied `Limits` reference and are not
/// stored on the arena — this keeps `Bump` to a single 32-byte payload
/// (ptr, end, limits pointer) that the compiler can keep in registers
/// across the recursive parse.
pub(crate) struct Bump<'a> {
    /// Current allocation pointer (always aligned to max_align_t after init).
    ptr: *mut u8,
    /// End of the buffer (one past the last valid byte).
    end: *mut u8,
    /// Pointer to the caller-supplied resource limits.  Stored as a raw
    /// pointer so we don't force a co-lifetime between the arena buffer
    /// and the limits structure.
    limits: *const Limits,
    _marker: PhantomData<&'a mut [MaybeUninit<u8>]>,
}

impl<'a> Bump<'a> {
    /// Create a new bump allocator from a user-supplied scratch buffer.
    ///
    /// The initial pointer is advanced to the next `max_align_t` boundary
    /// to guarantee that all subsequent `alloc_slice` calls start at a
    /// well-aligned address without runtime alignment fixups.
    pub(crate) fn new(buffer: &'a mut [MaybeUninit<u8>], limits: &Limits) -> Self {
        let raw_start = buffer.as_mut_ptr() as *mut u8;
        let cap = buffer.len();
        let raw_end = unsafe { raw_start.add(cap) };

        // Round ptr up to max_align_t.
        let align = core::mem::align_of::<u128>();
        let aligned = align_up(raw_start as usize, align);
        let ptr = unsafe { raw_start.add(aligned - raw_start as usize) };

        Bump {
            ptr,
            end: raw_end,
            limits,
            _marker: PhantomData,
        }
    }

    /// Access the resource limits embedded in the arena.
    #[inline(always)]
    pub(crate) fn limits(&self) -> &Limits {
        // SAFETY: `limits` always points to a live `Limits` value that
        // outlives the arena — it is provided by the caller and guaranteed
        // to live for the duration of the parse.
        unsafe { &*self.limits }
    }

    /// Allocate space for `len` elements of type `T` and return a mutable
    /// reference to the uninitialised slice.
    #[inline(always)]
    pub(crate) fn alloc_slice<T>(&mut self, len: usize) -> Result<&'a mut [T], EtfError> {
        let align = core::mem::align_of::<T>();
        let size = core::mem::size_of::<T>()
            .checked_mul(len)
            .ok_or(EtfError::ArenaExhausted)?;

        // Align pointer up to T's alignment.
        let addr = (self.ptr as usize + align - 1) & !(align - 1);

        // Check that the aligned pointer is still within bounds.
        if addr >= self.end as usize {
            return Err(EtfError::ArenaExhausted);
        }

        let ptr = self.ptr.with_addr(addr);

        // Check if the allocation would fit before doing ptr.add(size).
        // ptr.add() is UB if it would go out of bounds.
        let remaining = (self.end as usize).saturating_sub(addr);
        if size > remaining {
            return Err(EtfError::ArenaExhausted);
        }

        let end = unsafe { ptr.add(size) };
        self.ptr = end;

        unsafe { Ok(core::slice::from_raw_parts_mut(ptr as *mut T, len)) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::MaybeUninit;

    // Stack arrays of `u8` have alignment 1, so the base pointer is not
    // guaranteed to be max_align_t-aligned and the off-by-one trick below
    // would not be deterministic.  Pin the alignment explicitly.
    #[repr(align(16))]
    struct Aligned([MaybeUninit<u8>; 64]);
    #[repr(align(16))]
    struct AlignedTiny([MaybeUninit<u8>; 16]);

    #[test]
    fn misaligned_buffer_is_rounded_up() {
        // A buffer whose base pointer is not max_align_t-aligned forces the
        // alignment fixup in `Bump::new` (offset by one byte here).
        let mut buf = Aligned([MaybeUninit::uninit(); 64]);
        let mis = &mut buf.0[1..];
        let limits = Limits::default();
        let mut bump = Bump::new(mis, &limits);
        let s = bump.alloc_slice::<u64>(1).expect("alloc should succeed");
        assert_eq!(s.as_ptr() as usize % core::mem::align_of::<u64>(), 0);
    }

    #[test]
    fn exhausted_when_aligning_past_end() {
        // A misaligned buffer too small to hold the alignment fixup plus any
        // allocation hits the `addr >= end` exhaustion check.
        let mut buf = AlignedTiny([MaybeUninit::uninit(); 16]);
        let mis = &mut buf.0[1..];
        let limits = Limits::default();
        let mut bump = Bump::new(mis, &limits);
        let err = bump.alloc_slice::<u64>(1).expect_err("alloc should fail");
        assert!(matches!(err, EtfError::ArenaExhausted));
    }

    #[test]
    fn exhausted_when_size_exceeds_remaining() {
        let mut buf = [MaybeUninit::<u8>::uninit(); 16];
        let limits = Limits::default();
        let mut bump = Bump::new(&mut buf, &limits);
        let err = bump.alloc_slice::<u8>(17).expect_err("alloc should fail");
        assert!(matches!(err, EtfError::ArenaExhausted));
    }
}
