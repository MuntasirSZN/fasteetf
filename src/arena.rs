use core::marker::PhantomData;
use core::mem::MaybeUninit;

use crate::error::EtfError;
use crate::limits::Limits;

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
        let misalignment = (raw_start as usize) & (align - 1);
        let adj = if misalignment == 0 {
            0
        } else {
            align - misalignment
        };
        let ptr = unsafe { raw_start.add(adj) };

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

        let ptr = addr as *mut u8;

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

    /// Allocate space for a single `Term` and return a mutable reference to it.
    #[inline(always)]
    pub(crate) fn alloc_term(&mut self) -> Result<&'a mut crate::types::Term<'a>, EtfError> {
        let size = core::mem::size_of::<crate::types::Term<'a>>();

        // SAFETY: The arena is created with `Bump::new` which aligns the
        // initial pointer to `max_align_t` (typically 16 bytes). Since
        // `Term`'s alignment is ≤ 16 bytes (it contains 8-byte references
        // and f64), the current pointer is guaranteed to be properly aligned.
        // We only need to check that the allocation fits within the buffer.
        let end = (self.ptr as usize).saturating_add(size);
        if end > (self.end as usize) {
            return Err(EtfError::ArenaExhausted);
        }

        unsafe {
            let term_ptr = self.ptr as *mut crate::types::Term<'a>;
            self.ptr = self.ptr.add(size);
            Ok(&mut *term_ptr)
        }
    }
}
